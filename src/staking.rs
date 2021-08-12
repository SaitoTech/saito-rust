use crate::{
    block::Block,
    blockchain::{Blockchain, GENESIS_PERIOD},
    crypto::{hash, SaitoHash},
    golden_ticket::GoldenTicket,
    slip::{Slip, SlipType},
    time::{create_timestamp},
    transaction::{TransactionType},
    wallet::Wallet,
};
use bigint::uint::U256;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

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

    pub fn add_staker_with_number(&mut self, slip : Slip, random_number : SaitoHash) {

        //
        // find winning nolan
        //
        let x = U256::from_big_endian(&random_number);
	let y = self.stakers.len() + 1;
        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);

        let insert_into_pos = zy.low_u64();

	self.stakers.insert(insert_into_pos as usize, slip);

    }

    pub fn find_winning_staker(&self, random_number : SaitoHash) -> Option<Slip> {

        if self.stakers.len() == 0 { return None; }

        //
        // find winning staker
        //
        let x = U256::from_big_endian(&random_number);
	let y = self.stakers.len();
        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);

        let retrieve_from_pos = zy.low_u64();

	let winning_slip = self.stakers[retrieve_from_pos as usize].clone();

	return Some(winning_slip);    
    }


    //
    // resets the staker table
    //
    // without using floating-point division, we calculate the share that each staker
    // should earn of the upcoming sweep through the stakers table, and insert the 
    // pending and pending-deposits slips into the staking table with the updated
    // expected payout.
    //
    pub fn reset_staker_table(&mut self , staking_treasury: u64) {

	//
        // move pending into staking table
	//
	for i in 0..self.pending.len() { self.add_staker(self.pending[i].clone()); }
	for i in 0..self.deposits.len() { self.add_staker(self.deposits[i].clone()); }
	self.pending = vec![];
	self.deposits = vec![];

	if self.stakers.len() == 0 {
	    return;
	}

	//
	// adjust the slip amounts based on genesis period
	//
	let staking_payout_per_block : u64 = staking_treasury / GENESIS_PERIOD;

println!("expected payout per block:  {}", staking_payout_per_block);

	//
	// calculate average amount staked
	//
	let mut total_staked: u64 = 0;
	for i in 0..self.stakers.len() {
	    // anything that was pending needs updating
	    self.stakers[i].set_slip_type(SlipType::StakerOutput);
	    total_staked += self.stakers[i].get_amount();
	}
	let average_staked = total_staked / self.stakers.len() as u64;

	//
	// calculate the payout for average stake
	//
println!("average staked: {}", average_staked);
	let m = U256::from_big_endian(&staking_payout_per_block.to_be_bytes());
	let p = U256::from_big_endian(&self.stakers.len().to_be_bytes());

	let (q, _r) = m.overflowing_div(p);
	let average_staker_payout = q.as_u64();

println!("average payout: {}", average_staker_payout);

	//
	// and adjust the payout based on this....
	//
	for i in 0..self.stakers.len() { 

println!("processing staker: {}", i);

	    //
	    // get the total staked
	    //
	    let my_staked_amount = self.stakers[i].get_amount();

println!("staker has: {}", my_staked_amount);

	    //
	    // figure how much we are due...
	    //
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked )
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
	    // my stake PLUS (my stake / 1 * ( my_stake / average_staked ) * ( ( treasury / genesis_period )
	    //

	    let a = U256::from_big_endian(&my_staked_amount.to_be_bytes());
	    let b = U256::from_big_endian(&average_staker_payout.to_be_bytes());
	    let nominator : U256 = a.saturating_mul(b);
	    let denominator = U256::from_big_endian(&average_staked.to_be_bytes());

	    let (z, f)  = nominator.overflowing_div(denominator);

	    let mut staking_profit: u64 = 0;
	    if f != true { staking_profit = z.as_u64(); }

	    let my_payout = my_staked_amount + staking_profit;

	    self.stakers[i].set_amount(my_payout);

println!("The payout for staker {} with {} is {}", i, my_staked_amount, my_payout);

	}
    }

    pub fn add_deposit(&mut self, slip : Slip) {
	self.deposits.push(slip);
    }

    pub fn add_staker(&mut self, slip : Slip) {
	self.stakers.push(slip);
    }

    pub fn add_pending(&mut self, slip : Slip) {
	self.pending.push(slip);
    }


    pub fn remove_deposit(&mut self, slip : Slip) -> bool {
	for i in 0..self.deposits.len() {
	    if slip.get_utxoset_key() == self.deposits[i].get_utxoset_key() {
		let _removed_slip = self.deposits.remove(i);    
		return true;
	    }
        }
	return false;
    }


    pub fn remove_staker(&mut self, slip : Slip) -> bool {
	for i in 0..self.stakers.len() {
	    if slip.get_utxoset_key() == self.stakers[i].get_utxoset_key() {
		let _removed_slip = self.stakers.remove(i);    
		return true;
	    }
        }
	return false;
    }

    pub fn remove_pending(&mut self, slip : Slip) -> bool {
	for i in 0..self.pending.len() {
	    if slip.get_utxoset_key() == self.pending[i].get_utxoset_key() {
		let _removed_slip = self.pending.remove(i);    
		return true;
	    }
        }
	return false;
    }



    //
    // handle staking / pending / deposit tables
    //
    pub fn on_chain_reorganization(
        &mut self,
	block: &Block,
        longest_chain: bool,
    ) -> bool {

	//
	// add/remove deposits
	//
        for tx in &block.transactions {
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
	// update staking tables
	//
	if block.get_has_fee_transaction() && block.get_has_golden_ticket() {

	    let fee_transaction = &block.transactions[block.get_fee_transaction_idx() as usize];
	    let golden_ticket_transaction = &block.transactions[block.get_golden_ticket_idx() as usize];

	    //
            // grab random input from golden ticket
            //
            let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                golden_ticket_transaction.get_message().to_vec(),
            );
            let mut router_random_number1 = hash(&golden_ticket.get_random().to_vec()); // router block1
	    let staker_random_number = hash(&router_random_number1.to_vec());	// staker block2
	    let router_random_number2 = hash(&staker_random_number.to_vec());	// router block2

	    if fee_transaction.outputs.len() < 3 { return true; }
	    if fee_transaction.inputs.len() < 1 { return true; }

	    let staker_output = fee_transaction.outputs[2].clone(); // 3rd output is staker
	    let staker_input = fee_transaction.inputs[0].clone(); // 1st input is staker


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
		if self.stakers.len() == 0 {
		    //self.reset_staker_table(block.get_staking_treasury());
		    self.reset_staker_table(100_000_000);
		}

		//
		// move staker to pending
		//
		let lucky_staker_option = self.find_winning_staker(staker_random_number);
		if let Some(lucky_staker) = lucky_staker_option {
		    self.remove_staker(lucky_staker.clone());
		    self.add_pending(lucky_staker.clone());
		}

		//
		// re-create staker table, if needed
		//
		if self.stakers.len() == 0 {
		    //self.reset_staker_table(block.get_staking_treasury());
		    self.reset_staker_table(100_000_000);
		}


	    //
	    // roll backward
	    //
	    } else {

		//
		// reset pending if necessary
		//
		if self.stakers.len() == 0 {
		    for i in 0..self.pending.len() {
		        self.stakers.push(self.pending[i].clone());
		    }
		    for i in 0..self.deposits.len() {
		        self.stakers.push(self.deposits[i].clone());
		    }
		    self.pending = vec![];
		    self.deposits = vec![];
		}

		//
		//
		//
	  	self.remove_pending(staker_output.clone());
		for z in 0..fee_transaction.inputs.len() {
		    let slip_type = staker_input.get_slip_type();
		    if slip_type == SlipType::StakerDeposit {
		        self.add_deposit(staker_input.clone());
		    }
		    if slip_type == SlipType::StakerOutput {
		        self.add_pending(staker_input.clone());
		    }
		}



		//
		// reset pending if necessary
		//
		if self.pending.len() == 0 {
		    for i in 0..self.pending.len() {
		        self.stakers.push(self.pending[i].clone());
		    }
		    for i in 0..self.deposits.len() {
		        self.stakers.push(self.deposits[i].clone());
		    }
		    self.pending = vec![];
		    self.deposits = vec![];
		}

		println!("roll backward...");

	    }
	}

        true

    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utilities::mocks::make_mock_block_with_info;
    use crate::{
        block::Block,
	golden_ticket::GoldenTicket,
	miner::Miner,
	slip::{Slip, SlipType},
	transaction::Transaction,
    };


    #[test]
    fn staking_table_test() {

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

	staking.reset_staker_table(1_000_000_000); // 10 Saito

	assert_eq!(staking.stakers[0].get_amount(), 210000000);
	assert_eq!(staking.stakers[1].get_amount(), 315000000);
	assert_eq!(staking.stakers[2].get_amount(), 420000000);
	assert_eq!(staking.stakers[3].get_amount(), 525000000);
	assert_eq!(staking.stakers[4].get_amount(), 630000000);
    }


    #[tokio::test]
    async fn blockchain_roll_forward_staking_table_test() {

        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let publickey;

        let mut transactions: Vec<Transaction> = vec![];
        let mut latest_block_id: u64;
        let mut latest_block_hash = [0; 32];
        let mut latest_block_difficulty: u64;
        let mut miner = Miner::new(wallet_lock.clone());

        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
        }

	//
	// initialize blockchain staking table
	//
	{
	    let mut blockchain = blockchain_lock.write().await;
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

	    blockchain.staking.add_deposit(slip1);
	    blockchain.staking.add_deposit(slip2);
	    blockchain.staking.add_deposit(slip3);
	    blockchain.staking.add_deposit(slip4);
	    blockchain.staking.add_deposit(slip5);

	    blockchain.staking.reset_staker_table(1_000_000_000); // 10 Saito

	    assert_eq!(blockchain.staking.stakers[0].get_amount(), 210000000);
	    assert_eq!(blockchain.staking.stakers[1].get_amount(), 315000000);
	    assert_eq!(blockchain.staking.stakers[2].get_amount(), 420000000);
	    assert_eq!(blockchain.staking.stakers[3].get_amount(), 525000000);
	    assert_eq!(blockchain.staking.stakers[4].get_amount(), 630000000);
 	}

	//
	// BLOCK 1
	//
	let mut current_timestamp = create_timestamp();
	let mut block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 10, 0, false).await;
        latest_block_hash = block.get_hash();

        //
        // add to blockchain
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }


	//
	// BLOCK 2
	//
	current_timestamp = create_timestamp() + 120000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();

        //
        // add to blockchain
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }


	//
	// BLOCK 3
	//
	current_timestamp = create_timestamp() + 240000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();

        //
        // add to blockchain
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }


	//
	// BLOCK 4
	//
	current_timestamp = create_timestamp() + 360000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, false).await;
        latest_block_hash = block.get_hash();

        //
        // add to blockchain
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }


	//
	// BLOCK 5
	//
	current_timestamp = create_timestamp() + 480000;
	block = make_mock_block_with_info(blockchain_lock.clone(), wallet_lock.clone(), publickey, latest_block_hash, current_timestamp, 0, 1, true).await;
        latest_block_hash = block.get_hash();

        //
        // add to blockchain
        //
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.add_block(block).await;
        }



	//
	// TEST STAKER PAID
	//
        let mut blockchain = blockchain_lock.write().await;
	let blk = blockchain.get_block(latest_block_hash).await;


        assert_eq!(blk.get_has_fee_transaction(), true);
        assert_eq!(blk.get_fee_transaction_idx(), 2); // normal tx, golden ticket, fee tx

println!("{:?}", blk.transactions[2].get_outputs());
        assert_eq!(blk.transactions[2].get_outputs()[2].get_slip_type(), SlipType::StakerOutput);



    }




}
