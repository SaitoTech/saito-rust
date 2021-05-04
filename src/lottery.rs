use crate::{
    block::Block,
    crypto::{hash, generate_random_data, PublicKey},
    transaction::{Transaction, TransactionBroadcastType},
    slip::Slip,
    golden_ticket::GoldenTicket
};

use std::sync::{Arc, RwLock};
use bigint::uint::U256;

// use crate::wallet::Wallet;
// use crate::network::NetworkMessage;
// use crate::types::BlockMessage;

// pub trait LotteryGame {
//     fn play(&mut self, prevblk: &Block, wallet: &RwLock<Wallet>, consensus_addr: &Recipient<NetworkMessage>);
//     fn generate_random_solution(&self) -> [u8; 32];
//     fn is_valid_solution(&self, random_solution: [u8; 32], prevblk: &Block) -> bool;
//     fn find_winner(&self, solution: &[u8; 32], prevblk: &Block) -> PublicKey;
//     fn create_gt_solution(&self, random_solution: [u8; 32], block_hash: [u8; 32], publickey: PublicKey) -> GoldenTicket;
// }

#[derive(Clone)]
pub struct Miner {
    pub active: bool,
    pub difficulty: f32,
    pub paysplit: f32,
}