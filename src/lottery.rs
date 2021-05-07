use crate::{
    block::Block,
    crypto::{hash, generate_random_data, PublicKey},
    transaction::{Transaction, TransactionBroadcastType},
    slip::Slip,
    golden_ticket::GoldenTicket,
    wallet::Wallet,
    types::SaitoMessage
};

// use std::pin::Pin;
// use std::future::Future;

use bigint::uint::U256;

pub trait LotteryGame {
    fn play(&mut self, prevblk: &Block, wallet: &mut Wallet) -> Option<Transaction>;
    fn generate_golden_ticket_transaction(
        &mut self,
        solution: [u8; 32],
        prevblk: &Block,
        wallet: &mut Wallet) -> Transaction;
    fn generate_random_solution(&self) -> [u8; 32];
    fn is_valid_solution(&self, random_solution: [u8; 32], prevblk: &Block) -> bool;
    fn find_winner(&self, solution: &[u8; 32], prevblk: &Block) -> PublicKey;
    fn create_gt_solution(&self, random_solution: [u8; 32], block_hash: [u8; 32], publickey: PublicKey) -> GoldenTicket;
}

#[derive(Clone)]
pub struct Miner {
    pub active: bool,
    pub difficulty: f32,
    pub paysplit: f32,
}

impl LotteryGame for Miner {
    fn play(&mut self, prevblk: &Block, wallet: &mut Wallet) -> Option<Transaction> {
        let solution = self.generate_random_solution();
        match self.is_valid_solution(solution, prevblk) {
            true => Some(self.generate_golden_ticket_transaction(solution, prevblk, wallet)),
            false => None
        }
    }

    fn generate_golden_ticket_transaction(
        &mut self,
        solution: [u8; 32],
        prevblk: &Block,
        wallet: &mut Wallet
    ) -> Transaction {
        let publickey = wallet.get_publickey();
        let gt_solution = self.create_gt_solution(
            solution,
            prevblk.get_bsh(),
            publickey,
        );

        // Find winning node
        let winning_tx_address = self.find_winner(&solution, &prevblk);

        // we need to calculate the fees that are gonna go in the slips here
        let paid_burn_fee = prevblk.get_paid_burnfee();

        // This is just inputs - outputs for all transactions in the block
        let total_fees_for_creator = prevblk.get_available_fees(&prevblk.get_creator());

        // get the fees available from our publickey
        let total_fees_in_block = prevblk.get_available_fees(&publickey);

        // calculate the amount the creator can take for themselves
        let mut creator_surplus = 0;
        if total_fees_for_creator > paid_burn_fee {
            creator_surplus = total_fees_for_creator - paid_burn_fee;
        }

        // find the amount that will be divied out to miners and nodes
        let total_fees_for_miners_and_nodes =
            (total_fees_in_block - creator_surplus) + prevblk.get_coinbase();

        // Calculate shares
        let miner_share = (total_fees_for_miners_and_nodes as f32 * prevblk.get_paysplit()).round() as u64;
        let node_share  = total_fees_for_miners_and_nodes - miner_share;

        // create our golden ticket tx (au_tx)
        let mut golden_tx: Transaction = match wallet.create_transaction(
            publickey,
            TransactionBroadcastType::GoldenTicket,
            100_000,
            0
        ) {
            Some(tx) => tx,
            None => Transaction::new(),
        };

        let mut miner_slip = Slip::new(publickey);
        miner_slip.set_amt(miner_share);

        let mut node_slip = Slip::new(winning_tx_address);
        node_slip.set_amt(node_share);

        golden_tx.add_to_slip(miner_slip);
        golden_tx.add_to_slip(node_slip);
        golden_tx.set_msg(bincode::serialize(&gt_solution).unwrap());

        // sign TX
        golden_tx.set_sig(
            wallet.create_signature(golden_tx.get_signature_source().as_slice())
        );

        return golden_tx;
    }

    fn generate_random_solution(&self) -> [u8; 32] {
        let mut hashed_solution: [u8; 32] = [0; 32];
        hash(generate_random_data(), &mut hashed_solution);
        return hashed_solution;
    }

    fn is_valid_solution(&self, random_solution: [u8; 32], prevblk: &Block) -> bool {
        let difficulty = prevblk.get_difficulty().round() as usize;
        let difficulty_grain: f32 = prevblk.get_difficulty() % 1.0;

        let random_solution_decimal = U256::from_big_endian(&random_solution[0..difficulty]);
        let previous_hash_decimal = U256::from_big_endian(&prevblk.get_bsh()[0..difficulty]);
        let difficulty_grain = U256::from((difficulty_grain * 16.0).round() as u32);

        if random_solution_decimal >= previous_hash_decimal
            && (random_solution_decimal - previous_hash_decimal) <= difficulty_grain {
            return true
        } else {
            return false;
        }
    }

    fn find_winner(&self, solution: &[u8; 32], prevblk: &Block) -> PublicKey {
        // sanity check
        //
        if prevblk.body.txs.len() == 0 {
            return prevblk.get_creator();
        }

        // convery byte array to decimal
        let max_solution: u64 = std::u64::MAX;
        let mut decimal_solution: u64 = 0;

        for i in 0..8 {
            decimal_solution |= solution[i] as u64;
            decimal_solution <<= 8 * i;
        }

        let winning_float: f32 = decimal_solution as f32 / max_solution as f32;

        let last_block_cumulative_fees = prevblk.body.txs.last().unwrap().get_fees_cumulative();
        let winning_cumulative_fee_number = (last_block_cumulative_fees as f32 * winning_float as f32).round() as u64;
        let mut winning_tx: Option<&Transaction> = None;

        for tx in prevblk.body.txs.iter() {
            if tx.get_fees_cumulative() < winning_cumulative_fee_number {
                winning_tx = Some(&tx);
            }
            else {
                break;
            }
        }

        match winning_tx {
            Some(tx) => {
                let pathlength = tx.get_path().len();
                if  pathlength == 0 {
                    return tx.get_from_slips()
                        .last()
                        .unwrap()
                        .return_add()
                } else {
                    let mut winning_addr: PublicKey;
                    let z: f32 = winning_float * pathlength as f32;

                    // default to using this first one in the path here
                    winning_addr = tx.get_path()
                                     .first()
                                     .unwrap()
                                     .to;
                    for j in 0..pathlength {
                        let a: f32 = 2.0 - (1.0 / 2_u32.pow(j as u32 - 1) as f32) as f32;
                        if a <= z {
                            winning_addr = tx.get_path()[j].to;
                            break;
                        }
                    }
                    return winning_addr
                }
            },
            None => return prevblk.get_creator(),
        }

    }

    fn create_gt_solution(&self, random: [u8; 32], target: [u8; 32], publickey: PublicKey) -> GoldenTicket {
        return GoldenTicket::new(1, target, random, publickey) ;
    }
}

impl Miner {
    pub fn new() -> Miner {
        return Miner { active: true, difficulty: 2.0, paysplit: 0.5 }
    }
}

#[derive(Clone)]
pub struct Lottery<G: LotteryGame> {
    pub game: G,
    pub target: Option<Block>,
    pub wallet: Wallet,
}

impl<G> Lottery<G> where G: LotteryGame {
    pub fn new(game: G, wallet: Wallet) -> Lottery<G> {
        return Lottery { game, target: None, wallet }
    }

    pub fn play(&mut self, block: Block) -> Option<Transaction> {
        self.game.play(&block, &mut self.wallet)
    }

    pub fn create_solution(&mut self, block: Block) -> Transaction {
        self.game.generate_golden_ticket_transaction(
            self.game.generate_random_solution(),
            &block,
            &mut self.wallet
        )
    }
}