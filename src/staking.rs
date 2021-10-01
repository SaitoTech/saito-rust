//
// TODO
// - array is not the best data structure
// - insert needs to place into a specified position, probabaly ordered by publickey and then UUID
//
use crate::{
    block::Block,
    blockchain::GENESIS_PERIOD,
    crypto::{hash, SaitoHash},
    golden_ticket::GoldenTicket,
    slip::{Slip, SlipType},
    transaction::TransactionType,
};
use bigint::uint::U256;
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct Staking {
    // deposits waiting to join staking table for the first time
    pub deposits: Vec<Slip>,
    // in staking table waiting for selection / payout
    pub stakers: Vec<Slip>,
    // waiting for reset of staking table
    pub pending: Vec<Slip>,
}

impl Staking {
    pub fn new() -> Staking {
        Staking {
            deposits: vec![],
            stakers: vec![],
            pending: vec![],
        }
    }

    pub fn find_winning_staker(&self, random_number: SaitoHash) -> Option<Slip> {
        if self.stakers.is_empty() {
            return None;
        }

        //
        // find winning staker
        //
        let x = U256::from_big_endian(&random_number);
        let y = self.stakers.len();
        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);

        let retrieve_from_pos = zy.low_u64();

        let winning_slip = self.stakers[retrieve_from_pos as usize].clone();

        Some(winning_slip)
    }

    //
    // resets the staker table
    //
    // without using floating-point division, we calculate the share that each staker
    // should earn of the upcoming sweep through the stakers table, and insert the
    // pending and pending-deposits slips into the staking table with the updated
    // expected payout.
    //
    // returns three vectors with slips to SPEND, UNSPEND, DELETE
    //
    // wny?
    //
    // we cannot pass the UTXOSet into the staking object to update as that would
    // require multiple mutable borrows of the blockchain object, so we receive
    // vectors of the slips that need to be inserted, spent or deleted in the
    // blockchain and handle after-the-fact. This permits keeping the UTXOSET
    // up-to-date with the staking tables.
    //
    pub fn reset_staker_table(
        &mut self,
        staking_treasury: u64,
    ) -> (Vec<Slip>, Vec<Slip>, Vec<Slip>) {
        event!(Level::TRACE, "===========================");
        event!(Level::TRACE, "=== RESET STAKING TABLE ===");
        event!(Level::TRACE, "===========================");

        let res_spend: Vec<Slip> = vec![];
        let res_unspend: Vec<Slip> = vec![];
        let res_delete: Vec<Slip> = vec![];

        //
        // move pending into staking table
        //
        for i in 0..self.pending.len() {
            self.add_staker(self.pending[i].clone());
        }
        for i in 0..self.deposits.len() {
            self.add_staker(self.deposits[i].clone());
        }
        self.pending = vec![];
        self.deposits = vec![];

        if self.stakers.is_empty() {
            return (res_spend, res_unspend, res_delete);
        }

        //
        // adjust the slip amounts based on genesis period
        //
        let staking_payout_per_block: u64 = staking_treasury / GENESIS_PERIOD;

        //
        // calculate average amount staked
        //
        let mut total_staked: u64 = 0;
        for i in 0..self.stakers.len() {
            // TODO - delete when ready
            // we do not update the slip type anymore because we need
            // the slip to be spendable when it is issued.
            // we update the payout rather than the amount so the slip
            // can validate as spendable as well.
            //self.stakers[i].set_slip_type(SlipType::StakerOutput);
            total_staked += self.stakers[i].get_amount();
        }
        let average_staked = total_staked / self.stakers.len() as u64;

        //
        // calculate the payout for average stake
        //
        let m = U256::from_big_endian(&staking_payout_per_block.to_be_bytes());
        let p = U256::from_big_endian(&self.stakers.len().to_be_bytes());

        let (q, _r) = m.overflowing_div(p);
        let average_staker_payout = q.as_u64();

        //
        // and adjust the payout based on this....
        //
        for i in 0..self.stakers.len() {
            //
            // get the total staked
            //
            let my_staked_amount = self.stakers[i].get_amount();

            //
            // figure how much we are due...
            //
            // my stake PLUS (my stake / 1 * ( my_stake / average_staked )
            // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
            // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
            //
            let a = U256::from_big_endian(&my_staked_amount.to_be_bytes());
            let b = U256::from_big_endian(&average_staker_payout.to_be_bytes());
            let nominator: U256 = a.saturating_mul(b);
            let denominator = U256::from_big_endian(&average_staked.to_be_bytes());

            let (z, f) = nominator.overflowing_div(denominator);

            let mut staking_profit: u64 = 0;
            if !f {
                staking_profit = z.as_u64();
            }

            self.stakers[i].set_payout(staking_profit);
        }

        (res_spend, res_unspend, res_delete)
    }

    pub fn validate_slip_in_deposits(&self, slip: Slip) -> bool {
        for i in 0..self.deposits.len() {
            if slip.get_utxoset_key() == self.deposits[i].get_utxoset_key() {
                return true;
            }
        }
        false
    }

    pub fn validate_slip_in_stakers(&self, slip: Slip) -> bool {
        for i in 0..self.stakers.len() {
            if slip.get_utxoset_key() == self.stakers[i].get_utxoset_key() {
                return true;
            }
        }
        false
    }

    pub fn validate_slip_in_pending(&self, slip: Slip) -> bool {
        for i in 0..self.pending.len() {
            if slip.get_utxoset_key() == self.pending[i].get_utxoset_key() {
                return true;
            }
        }
        false
    }

    pub fn add_deposit(&mut self, slip: Slip) {
        self.deposits.push(slip);
    }

    //
    // slips are added in ascending order based on publickey and then
    // UUID. this is simply to ensure that chain reorgs do not cause
    // disagreements about which staker is selected.
    //
    pub fn add_staker(&mut self, slip: Slip) -> bool {
        //
        // TODO skip-hop algorithm instead of brute force
        //
        if self.stakers.len() == 0 {
            self.stakers.push(slip);
            return true;
        } else {
            for i in 0..self.stakers.len() {
                let how_compares = slip.compare(self.stakers[i].clone());
                // 1 - self is bigger
                // 2 - self is smaller
                // insert at position i
                if how_compares == 2 {
                    println!("we are bigger than slip at position: {}", i);
                    if self.stakers.len() == (i + 1) {
                        self.stakers.push(slip);
                        return true;
                    }
                } else {
                    if how_compares == 1 {
                        self.stakers.insert(i, slip);
                        return true;
                    }
                    if how_compares == 3 {
                        return false;
                    }
                }
            }

            println!("we were smaller all the way through -- add at end");
            self.stakers.push(slip);
            return true;
        }
    }

    pub fn add_pending(&mut self, slip: Slip) {
        self.pending.push(slip);
    }

    pub fn remove_deposit(&mut self, slip: Slip) -> bool {
        for i in 0..self.deposits.len() {
            if slip.get_utxoset_key() == self.deposits[i].get_utxoset_key() {
                let _removed_slip = self.deposits.remove(i);
                return true;
            }
        }
        false
    }

    pub fn remove_staker(&mut self, slip: Slip) -> bool {
        for i in 0..self.stakers.len() {
            if slip.get_utxoset_key() == self.stakers[i].get_utxoset_key() {
                let _removed_slip = self.stakers.remove(i);
                return true;
            }
        }
        false
    }

    pub fn remove_pending(&mut self, slip: Slip) -> bool {
        for i in 0..self.pending.len() {
            if slip.get_utxoset_key() == self.pending[i].get_utxoset_key() {
                let _removed_slip = self.pending.remove(i);
                return true;
            }
        }
        false
    }

    //
    // handle staking / pending / deposit tables
    //
    // returns slips to SPEND, UNSPEND and DELETE
    //
    // this is required as staking table controlled by blockchain and Rust
    // restricts us from passing the UTXOSET into this part of the program.
    //
    pub fn on_chain_reorganization(
        &mut self,
        block: &Block,
        longest_chain: bool,
    ) -> (Vec<Slip>, Vec<Slip>, Vec<Slip>) {
        let res_spend: Vec<Slip> = vec![];
        let res_unspend: Vec<Slip> = vec![];
        let res_delete: Vec<Slip> = vec![];

        //
        // add/remove deposits
        //
        for tx in block.get_transactions() {
            if tx.get_transaction_type() == TransactionType::StakerWithdrawal {
                //
                // someone has successfully withdrawn so we need to remove this slip
                // from the necessary table if moving forward, or add it back if
                // moving backwards.
                //
                // roll forward
                //
                if longest_chain {
                    if tx.inputs[0].get_slip_type() == SlipType::StakerWithdrawalPending {
                        self.remove_pending(tx.inputs[0].clone());
                    }
                    if tx.inputs[0].get_slip_type() == SlipType::StakerWithdrawalStaking {
                        self.remove_staker(tx.inputs[0].clone());
                    }
                //
                // roll backward
                //
                } else {
                    if tx.inputs[0].get_slip_type() == SlipType::StakerWithdrawalPending {
                        self.add_pending(tx.inputs[0].clone());
                    }
                    if tx.inputs[0].get_slip_type() == SlipType::StakerWithdrawalStaking {
                        self.add_staker(tx.inputs[0].clone());
                    }
                }
            }

            if tx.get_transaction_type() == TransactionType::StakerDeposit {
                for i in 0..tx.outputs.len() {
                    if tx.outputs[i].get_slip_type() == SlipType::StakerDeposit {
                        //
                        // roll forward
                        //
                        if longest_chain {
                            self.add_deposit(tx.outputs[i].clone());

                        //
                        // roll backward
                        //
                        } else {
                            self.remove_deposit(tx.outputs[i].clone());
                        }
                    }
                }
            }
        }

        //
        // reset tables if needed
        //
        if longest_chain {
            //
            // reset stakers if necessary
            //
            if self.stakers.is_empty() {
                let (_res_spend, _res_unspend, _res_delete) =
                    self.reset_staker_table(block.get_staking_treasury());
            }
        } else {
            //
            // reset pending if necessary
            //
            if self.pending.is_empty() {
                self.pending = vec![];
                self.deposits = vec![];
                for i in 0..self.stakers.len() {
                    if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
                        self.pending.push(self.stakers[i].clone());
                    }
                    if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
                        self.deposits.push(self.stakers[i].clone());
                    }
                }
                self.stakers = vec![];
            }
        }

        //
        // update staking tables
        //
        if block.get_has_fee_transaction() && block.get_has_golden_ticket() {
            let fee_transaction =
                &block.get_transactions()[block.get_fee_transaction_idx() as usize];

            let golden_ticket_transaction =
                &block.get_transactions()[block.get_golden_ticket_idx() as usize];

            //
            // grab random input from golden ticket
            //
            let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                golden_ticket_transaction.get_message().to_vec(),
            );

            // pick router and burn one
            let mut next_random_number = hash(&golden_ticket.get_random().to_vec());
            next_random_number = hash(&next_random_number.to_vec());
            next_random_number = hash(&next_random_number.to_vec());

            if fee_transaction.outputs.len() < 3 {
                return (res_spend, res_unspend, res_delete);
            }
            if fee_transaction.inputs.is_empty() {
                return (res_spend, res_unspend, res_delete);
            }

            //
            // roll forward
            //
            if longest_chain {
                //
                // re-create staker table, if needed
                //
                // we do this at both the start and the end of this function so that
                // we will always have a table that can be handled regardless of
                // vacillations in on_chain_reorg, such as resetting the table and
                // then non-longest-chaining the same block
                //
                event!(
                    Level::TRACE,
                    "Rolling forward and moving into pending: {}!",
                    self.stakers.len()
                );
                if self.stakers.is_empty() {
                    let (_res_spend, _res_unspend, _res_delete) =
                        self.reset_staker_table(block.get_staking_treasury());
                }

                //
                // process outbound staking payments
                //
                let mut slips_to_remove_from_staking = vec![];
                let mut slips_to_add_to_pending = vec![];

                let mut staker_slip_num = 0;
                for i in 0..fee_transaction.outputs.len() {
                    let staker_output = fee_transaction.outputs[i].clone();
                    // we have already handled all stakers
                    if fee_transaction.inputs.len() <= staker_slip_num {
                        break;
                    }
                    if staker_output.get_slip_type() == SlipType::StakerOutput {
                        // ROUTER BURNED FIRST
                        next_random_number = hash(&next_random_number.to_vec()); // router + burn
                        next_random_number = hash(&next_random_number.to_vec()); // burn second

                        //
                        // move staker to pending
                        //
                        let lucky_staker_option = self.find_winning_staker(next_random_number); // use first

                        if let Some(lucky_staker) = lucky_staker_option {
                            event!(Level::TRACE, "the lucky staker is: {:?}", lucky_staker);
                            event!(
                                Level::TRACE,
                                "moving from staker into pending: {}",
                                lucky_staker.get_amount()
                            );

                            slips_to_remove_from_staking.push(lucky_staker.clone());
                            slips_to_add_to_pending.push(staker_output.clone());
                        }
                        staker_slip_num += 1;

                        // setup for router selection next loop
                        next_random_number = hash(&next_random_number.to_vec());
                    }
                }

                //
                // we handle the slips together like this as we can occasionally
                // get duplicates if the same slip is selected recursively, but
                // we do not pay out duplicates. so we only add to pending if we
                // successfully remove from the staker table.
                //
                for i in 0..slips_to_remove_from_staking.len() {
                    if self.remove_staker(slips_to_remove_from_staking[i].clone()) == true {
                        self.add_pending(slips_to_add_to_pending[i].clone());
                    }
                }

                //
                // re-create staker table, if needed
                //
                if self.stakers.is_empty() {
                    let (_res_spend, _res_unspend, _res_delete) =
                        self.reset_staker_table(block.get_staking_treasury());
                }

            //
            // roll backward
            //
            } else {
                event!(Level::TRACE, "roll backward...");

                //
                // reset pending if necessary
                //
                if self.pending.is_empty() {
                    self.pending = vec![];
                    self.deposits = vec![];
                    for i in 0..self.stakers.len() {
                        if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
                            self.pending.push(self.stakers[i].clone());
                        }
                        if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
                            self.deposits.push(self.stakers[i].clone());
                        }
                    }
                    self.stakers = vec![];
                }

                //
                // process outbound staking payments
                //
                let mut staker_slip_num = 0;
                for i in 0..fee_transaction.outputs.len() {
                    let staker_output = fee_transaction.outputs[i].clone();
                    if fee_transaction.inputs.len() < staker_slip_num {
                        break;
                    }
                    let staker_input = fee_transaction.inputs[staker_slip_num].clone();

                    if staker_output.get_slip_type() == SlipType::StakerOutput {
                        //
                        // remove from pending to staker (awaiting payout)
                        //
                        self.remove_pending(staker_output.clone());
                        let slip_type = staker_input.get_slip_type();
                        if slip_type == SlipType::StakerDeposit {
                            self.add_deposit(staker_input.clone());
                        }
                        if slip_type == SlipType::StakerOutput {
                            self.add_staker(staker_input.clone());
                        }

                        staker_slip_num += 1;
                    }
                }

                //
                // reset pending if necessary
                //
                if self.pending.is_empty() {
                    self.pending = vec![];
                    self.deposits = vec![];
                    for i in 0..self.stakers.len() {
                        if self.stakers[i].get_slip_type() == SlipType::StakerOutput {
                            self.pending.push(self.stakers[i].clone());
                        }
                        if self.stakers[i].get_slip_type() == SlipType::StakerDeposit {
                            self.deposits.push(self.stakers[i].clone());
                        }
                    }
                    self.stakers = vec![];
                }
            }
        }

        (res_spend, res_unspend, res_delete)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utilities::test_manager::TestManager;
    use crate::transaction::Transaction;
    use crate::{
        blockchain::Blockchain,
        slip::{Slip, SlipType},
        time::create_timestamp,
        wallet::Wallet,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    //
    // does adding staking slips in different orders give us the same
    // results?
    //
    #[test]
    fn staking_add_staker_slips_in_different_order_and_check_sorting_works() {
        let mut staking1 = Staking::new();
        let mut staking2 = Staking::new();

        let mut slip1 = Slip::new();
        slip1.set_amount(1);
        slip1.set_slip_type(SlipType::StakerDeposit);

        let mut slip2 = Slip::new();
        slip2.set_amount(2);
        slip2.set_slip_type(SlipType::StakerDeposit);

        let mut slip3 = Slip::new();
        slip3.set_amount(3);
        slip3.set_slip_type(SlipType::StakerDeposit);

        let mut slip4 = Slip::new();
        slip4.set_amount(4);
        slip4.set_slip_type(SlipType::StakerDeposit);

        let mut slip5 = Slip::new();
        slip5.set_amount(5);
        slip5.set_slip_type(SlipType::StakerDeposit);

        staking1.add_staker(slip1.clone());
        assert_eq!(staking1.stakers.len(), 1);
        staking1.add_staker(slip2.clone());
        assert_eq!(staking1.stakers.len(), 2);
        staking1.add_staker(slip3.clone());
        assert_eq!(staking1.stakers.len(), 3);
        staking1.add_staker(slip4.clone());
        assert_eq!(staking1.stakers.len(), 4);
        staking1.add_staker(slip5.clone());
        assert_eq!(staking1.stakers.len(), 5);

        staking2.add_staker(slip2.clone());
        staking2.add_staker(slip4.clone());
        staking2.add_staker(slip1.clone());
        staking2.add_staker(slip5.clone());
        staking2.add_staker(slip3.clone());
        staking2.add_staker(slip2.clone());
        staking2.add_staker(slip1.clone());

        assert_eq!(staking1.stakers.len(), 5);
        assert_eq!(staking2.stakers.len(), 5);

        for i in 0..staking2.stakers.len() {
            println!(
                "{} -- {}",
                staking1.stakers[i].get_amount(),
                staking2.stakers[i].get_amount()
            );
        }

        for i in 0..staking2.stakers.len() {
            assert_eq!(
                staking1.stakers[i].clone().serialize_for_net(),
                staking2.stakers[i].clone().serialize_for_net()
            );
            assert_eq!(
                staking1.stakers[i]
                    .clone()
                    .compare(staking2.stakers[i].clone()),
                3
            ); // 3 = the same
        }
    }

    //
    // do staking deposits work properly and create proper payouts?
    //
    #[test]
    fn staking_add_deposit_slip_and_payout_calculation_test() {
        let mut staking = Staking::new();

        let mut slip1 = Slip::new();
        slip1.set_amount(200_000_000);
        slip1.set_slip_type(SlipType::StakerDeposit);

        let mut slip2 = Slip::new();
        slip2.set_amount(300_000_000);
        slip2.set_slip_type(SlipType::StakerDeposit);

        let mut slip3 = Slip::new();
        slip3.set_amount(400_000_000);
        slip3.set_slip_type(SlipType::StakerDeposit);

        let mut slip4 = Slip::new();
        slip4.set_amount(500_000_000);
        slip4.set_slip_type(SlipType::StakerDeposit);

        let mut slip5 = Slip::new();
        slip5.set_amount(600_000_000);
        slip5.set_slip_type(SlipType::StakerDeposit);

        staking.add_deposit(slip1);
        staking.add_deposit(slip2);
        staking.add_deposit(slip3);
        staking.add_deposit(slip4);
        staking.add_deposit(slip5);

        let (_res_spend, _res_unspend, _res_delete) = staking.reset_staker_table(1_000_000_000); // 10 Saito

        assert_eq!(
            staking.stakers[4].get_amount() + staking.stakers[4].get_payout(),
            210000000
        );
        assert_eq!(
            staking.stakers[3].get_amount() + staking.stakers[3].get_payout(),
            315000000
        );
        assert_eq!(
            staking.stakers[2].get_amount() + staking.stakers[2].get_payout(),
            420000000
        );
        assert_eq!(
            staking.stakers[1].get_amount() + staking.stakers[1].get_payout(),
            525000000
        );
        assert_eq!(
            staking.stakers[0].get_amount() + staking.stakers[0].get_payout(),
            630000000
        );
    }

    //
    // do we get proper results removing stakers and adding to pending? this is
    // important because we rely on remove_stakers() to not remove non-existing
    // entities, otherwise we need more complicated dupe-detection code.
    //
    #[test]
    fn staking_remove_staker_code_handles_duplicates_properly() {
        let mut staking = Staking::new();

        let mut slip1 = Slip::new();
        slip1.set_amount(200_000_000);
        slip1.set_slip_type(SlipType::StakerDeposit);

        let mut slip2 = Slip::new();
        slip2.set_amount(300_000_000);
        slip2.set_slip_type(SlipType::StakerDeposit);

        let mut slip3 = Slip::new();
        slip3.set_amount(400_000_000);
        slip3.set_slip_type(SlipType::StakerDeposit);

        let mut slip4 = Slip::new();
        slip4.set_amount(500_000_000);
        slip4.set_slip_type(SlipType::StakerDeposit);

        let mut slip5 = Slip::new();
        slip5.set_amount(600_000_000);
        slip5.set_slip_type(SlipType::StakerDeposit);

        staking.add_deposit(slip1.clone());
        staking.add_deposit(slip2.clone());
        staking.add_deposit(slip3.clone());
        staking.add_deposit(slip4.clone());
        staking.add_deposit(slip5.clone());

        let (_res_spend, _res_unspend, _res_delete) = staking.reset_staker_table(1_000_000_000); // 10 Saito

        assert_eq!(staking.stakers.len(), 5);
        assert_eq!(staking.remove_staker(slip1.clone()), true);
        assert_eq!(staking.remove_staker(slip2.clone()), true);
        assert_eq!(staking.remove_staker(slip1.clone()), false);
        assert_eq!(staking.stakers.len(), 3);
        assert_eq!(staking.remove_staker(slip5.clone()), true);
        assert_eq!(staking.remove_staker(slip5.clone()), false);
        assert_eq!(staking.stakers.len(), 2);
    }

    //
    // will staking payouts and the reset / rollover of the staking table work
    // properly with single-payouts per block?
    //
    #[tokio::test]
    async fn staking_create_blockchain_with_two_staking_deposits_one_staker_payout_per_block() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        //
        // initialize blockchain staking table
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            let wallet = wallet_lock.read().await;
            let publickey = wallet.get_publickey();

            let mut slip1 = Slip::new();
            slip1.set_amount(200_000_000);
            slip1.set_slip_type(SlipType::StakerDeposit);

            let mut slip2 = Slip::new();
            slip2.set_amount(300_000_000);
            slip2.set_slip_type(SlipType::StakerDeposit);

            slip1.set_publickey(publickey);
            slip2.set_publickey(publickey);

            slip1.generate_utxoset_key();
            slip2.generate_utxoset_key();

            // add to utxoset
            slip1.on_chain_reorganization(&mut blockchain.utxoset, true, 1);
            slip2.on_chain_reorganization(&mut blockchain.utxoset, true, 1);

            blockchain.staking.add_deposit(slip1);
            blockchain.staking.add_deposit(slip2);

            blockchain.staking.reset_staker_table(1_000_000_000); // 10 Saito
        }

        let current_timestamp = create_timestamp();

        //
        // BLOCK 1
        //
        let block1 = test_manager
            .generate_block_and_metadata([0; 32], current_timestamp, 3, 0, false, vec![])
            .await;
        let block1_hash = block1.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block1).await;

        //
        // BLOCK 2
        //
        let block2 = test_manager
            .generate_block_and_metadata(
                block1_hash,
                current_timestamp + 120000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block2_hash = block2.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block2).await;

        //
        // we have yet to find a single golden ticket, so all in stakers
        //
        {
            let blockchain = blockchain_lock.write().await;
            assert_eq!(blockchain.staking.stakers.len(), 2);
            assert_eq!(blockchain.staking.pending.len(), 0);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 3
        //
        let block3 = test_manager
            .generate_block_and_metadata(
                block2_hash,
                current_timestamp + 240000,
                0,
                1,
                true,
                vec![],
            )
            .await;
        let block3_hash = block3.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block3).await;

        //
        // we have found a single golden ticket, so we have paid a single staker
        //
        {
            let blockchain = blockchain_lock.write().await;
            assert_eq!(blockchain.staking.stakers.len(), 1);
            assert_eq!(blockchain.staking.pending.len(), 1);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 4
        //
        let block4 = test_manager
            .generate_block_and_metadata(
                block3_hash,
                current_timestamp + 360000,
                0,
                1,
                false,
                vec![],
            )
            .await;
        let block4_hash = block4.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block4).await;

        {
            let blockchain = blockchain_lock.write().await;
            assert_eq!(blockchain.staking.stakers.len(), 1);
            assert_eq!(blockchain.staking.pending.len(), 1);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 5
        //
        test_manager.set_latest_block_hash(block4_hash);
        test_manager
            .add_block(current_timestamp + 480000, 0, 1, true, vec![])
            .await;

        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.staking.stakers.len(), 2);
            assert_eq!(blockchain.staking.pending.len(), 0);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 6
        //
        test_manager
            .add_block(current_timestamp + 600000, 0, 1, false, vec![])
            .await;

        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.staking.stakers.len(), 2);
            assert_eq!(blockchain.staking.pending.len(), 0);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 7
        //
        test_manager
            .add_block(current_timestamp + 720000, 0, 1, true, vec![])
            .await;

        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.staking.stakers.len(), 1);
            assert_eq!(blockchain.staking.pending.len(), 1);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 8
        //
        test_manager
            .add_block(current_timestamp + 840000, 0, 1, false, vec![])
            .await;

        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.staking.stakers.len(), 1);
            assert_eq!(blockchain.staking.pending.len(), 1);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 9
        //
        test_manager
            .add_block(current_timestamp + 960000, 0, 1, true, vec![])
            .await;
        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.staking.stakers.len(), 2);
            assert_eq!(blockchain.staking.pending.len(), 0);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }
    }

    //
    // will staking payouts and the reset / rollover of the staking table work
    // properly with recurvise staking payouts that stretch back at least two blocks
    //
    #[tokio::test]
    async fn staking_create_blockchain_with_many_staking_deposits_many_staker_payouts_per_block() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        //
        // initialize blockchain staking table
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            let wallet = wallet_lock.read().await;
            let publickey = wallet.get_publickey();

            let mut slip1 = Slip::new();
            slip1.set_amount(200_000_000);
            slip1.set_slip_type(SlipType::StakerDeposit);

            let mut slip2 = Slip::new();
            slip2.set_amount(300_000_000);
            slip2.set_slip_type(SlipType::StakerDeposit);

            let mut slip3 = Slip::new();
            slip3.set_amount(400_000_000);
            slip3.set_slip_type(SlipType::StakerDeposit);

            slip1.set_publickey(publickey);
            slip2.set_publickey(publickey);
            slip3.set_publickey(publickey);

            slip1.generate_utxoset_key();
            slip2.generate_utxoset_key();
            slip3.generate_utxoset_key();

            // add to utxoset
            slip1.on_chain_reorganization(&mut blockchain.utxoset, true, 1);
            slip2.on_chain_reorganization(&mut blockchain.utxoset, true, 1);
            slip3.on_chain_reorganization(&mut blockchain.utxoset, true, 1);

            blockchain.staking.add_deposit(slip1);
            blockchain.staking.add_deposit(slip2);
            blockchain.staking.add_deposit(slip3);

            blockchain.staking.reset_staker_table(1_000_000_000); // 10 Saito
        }

        let current_timestamp = create_timestamp();

        //
        // BLOCK 1
        //
        let block1 = test_manager
            .generate_block_and_metadata([0; 32], current_timestamp, 10, 0, false, vec![])
            .await;
        let block1_hash = block1.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block1).await;

        //
        // BLOCK 2
        //
        test_manager.set_latest_block_hash(block1_hash);
        test_manager
            .add_block(current_timestamp + 120000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 3
        //
        test_manager
            .add_block(current_timestamp + 240000, 0, 1, false, vec![])
            .await;

        //
        // we have yet to find a single golden ticket, so all in stakers
        //
        {
            let blockchain = blockchain_lock.write().await;
            assert_eq!(blockchain.staking.stakers.len(), 3);
            assert_eq!(blockchain.staking.pending.len(), 0);
            assert_eq!(blockchain.staking.deposits.len(), 0);
        }

        //
        // BLOCK 4 - GOLDEN TICKET
        //
        test_manager
            .add_block(current_timestamp + 360000, 0, 1, true, vec![])
            .await;

        //
        // BLOCK 5
        //
        test_manager
            .add_block(current_timestamp + 480000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 6
        //
        test_manager
            .add_block(current_timestamp + 600000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 7
        //
        test_manager
            .add_block(current_timestamp + 720000, 0, 1, true, vec![])
            .await;

        //
        // BLOCK 8
        //
        test_manager
            .add_block(current_timestamp + 840000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 9
        //
        test_manager
            .add_block(current_timestamp + 960000, 0, 1, true, vec![])
            .await;

        // staking must have been handled properly for all blocks to validate
        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.get_latest_block_id(), 9);
            assert_eq!(
                3,
                blockchain.staking.stakers.len()
                    + blockchain.staking.pending.len()
                    + blockchain.staking.deposits.len()
            );
        }
    }
    // TODO fix this test
    #[ignore]
    #[tokio::test]
    async fn blockchain_staking_deposits_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());
        let current_timestamp = create_timestamp();
        let publickey;
        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
        }

        //
        // BLOCK 1 -- VIP transactions
        //
        test_manager
            .add_block(current_timestamp, 10, 0, false, vec![])
            .await;
        let block1_hash;

        {
            let blockchain = blockchain_lock.read().await;
            block1_hash = blockchain.get_latest_block_hash();
        }

        //
        // BLOCK 2 -- staking deposits
        //
        let mut stx1: Transaction;
        let mut stx2: Transaction;
        {
            let mut wallet = wallet_lock.write().await;
            stx1 = wallet.create_staking_deposit_transaction(100000).await;
            stx2 = wallet.create_staking_deposit_transaction(200000).await;
            stx1.generate_metadata(publickey);
            stx2.generate_metadata(publickey);
        }
        let mut transactions: Vec<Transaction> = vec![];
        transactions.push(stx1);
        transactions.push(stx2);
        let block2 = Block::generate(
            &mut transactions,
            block1_hash,
            wallet_lock.clone(),
            blockchain_lock.clone(),
            current_timestamp + 120000,
        )
        .await;
        let block2_hash = block2.get_hash();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block2).await;

        //
        // BLOCK 3 - payout
        //
        test_manager.set_latest_block_hash(block2_hash);
        test_manager
            .add_block(current_timestamp + 240000, 0, 1, true, vec![])
            .await;

        //
        // BLOCK 4
        //
        test_manager
            .add_block(current_timestamp + 360000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 5
        //
        test_manager
            .add_block(current_timestamp + 480000, 0, 1, true, vec![])
            .await;
        let block5_hash;
        {
            let blockchain = blockchain_lock.read().await;
            block5_hash = blockchain.get_latest_block_hash();
        }

        //
        // BLOCK 6 -- withdraw a staking deposit
        //
        let mut wstx1: Transaction;
        {
            let mut wallet = wallet_lock.write().await;
            let blockchain = blockchain_lock.write().await;
            wstx1 = wallet
                .create_staking_withdrawal_transaction(&blockchain.staking)
                .await;
            wstx1.generate_metadata(publickey);
        }
        let mut transactions: Vec<Transaction> = vec![];
        println!("----------");
        println!("---{:?}---", wstx1);
        println!("----------");
        transactions.push(wstx1);
        let block6 = Block::generate(
            &mut transactions,
            block5_hash,
            wallet_lock.clone(),
            blockchain_lock.clone(),
            current_timestamp + 600000,
        )
        .await;
        let block6_id = block6.get_id();
        Blockchain::add_block_to_blockchain(blockchain_lock.clone(), block6).await;

        {
            let blockchain = blockchain_lock.read().await;
            println!(
                "LATESTID: {} / {}",
                block6_id,
                blockchain.get_latest_block_id()
            );
            assert_eq!(blockchain.get_latest_block_id(), 6);
            assert_eq!(
                blockchain.staking.stakers.len() + blockchain.staking.pending.len(),
                1
            );
        }
    }

    #[tokio::test]
    async fn blockchain_roll_forward_staking_table_test_with_test_manager() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        let publickey;
        let current_timestamp = create_timestamp();

        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
            println!("publickey: {:?}", publickey);
        }

        //
        // BLOCK 1
        //
        test_manager
            .add_block(current_timestamp, 3, 0, false, vec![])
            .await;

        //
        // BLOCK 2
        //
        test_manager
            .add_block(current_timestamp + 120000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 3
        //
        test_manager
            .add_block(current_timestamp + 240000, 0, 1, false, vec![])
            .await;

        //
        // BLOCK 4
        //
        test_manager
            .add_block(current_timestamp + 360000, 0, 1, true, vec![])
            .await;

        test_manager.check_utxoset().await;
        test_manager.check_token_supply().await;
    }
}
