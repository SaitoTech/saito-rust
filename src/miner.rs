use crate::{
    block::Block,
    blockchain::Blockchain,
    consensus::SaitoMessage,
    crypto::{generate_random_bytes, hash, SaitoHash},
    golden_ticket::GoldenTicket,
    transaction::Transaction,
    wallet::Wallet,
};

use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};
use bigint::uint::U256;

#[derive(Debug, Clone)]
pub enum MinerMessage {
    StartMining,
    StopMining,
    MineGoldenTicket,
}

pub struct Miner {
    pub is_active: bool,
    pub target: SaitoHash,
    pub wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
}

impl Miner {
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Miner {
        Miner {
            is_active: false,
            target: [0; 32],
            wallet_lock,
            broadcast_channel_sender: None,
        }
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    fn is_valid_solution(&self, solution: SaitoHash, target_block_hash: SaitoHash) -> bool {
        let difficulty_curve: f64 = 1.0;
        let difficulty = difficulty_curve.round() as usize;
        let difficulty_grain: f64 = difficulty_curve % 1.0;

        let random_solution_decimal = U256::from_big_endian(&solution[0..difficulty]);
        let previous_hash_decimal = U256::from_big_endian(&target_block_hash[0..difficulty]);
        let difficulty_grain = U256::from((difficulty_grain * 16.0).round() as u32);

        random_solution_decimal >= previous_hash_decimal
            && (random_solution_decimal - previous_hash_decimal) <= difficulty_grain
    }

    pub async fn mine(&self, target_block_hash: SaitoHash) {
        if self.is_active {

            let solution = hash(&generate_random_bytes(32));

            if self.is_valid_solution(solution, target_block_hash) {

            { 
                let vote = 0;
                let wallet = self.wallet_lock.write().await;
                let gt = GoldenTicket::new(vote, target_block_hash, solution, wallet.get_publickey());

                        if !self.broadcast_channel_sender.is_none() {
                            self.broadcast_channel_sender.as_ref().unwrap()
                                .send(SaitoMessage::MinerNewGoldenTicket { ticket: gt })
                                .expect("error: MinerNewGoldenTicket message failed to send");
                        }
            }

		    // stop mining
                self.set_is_active(false);

	        }
        }
    }

    pub fn set_is_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    pub fn set_target(&mut self, target: SaitoHash) {
        self.target = target;
    }
}

pub async fn run(
    miner_lock: Arc<RwLock<Miner>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    //
    // miner gets global broadcast channel
    //
    {
        let mut miner = miner_lock.write().await;
        miner.set_broadcast_channel_sender(broadcast_channel_sender.clone());
    }

    let (miner_channel_sender, mut miner_channel_receiver) = mpsc::channel(4);

    let mine_ticket_sender = miner_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            mine_ticket_sender
                .send(MinerMessage::MineGoldenTicket)
                .await
                .expect("Failed to send MineGoldenTicket message");
            sleep(Duration::from_millis(100));
        }
    });

    loop {
        tokio::select! {

            //
            // Miner Channel Messages
            //
            Some(message) = miner_channel_receiver.recv() => {
                match message {
                    //
                    // Mine 1 Ticket
                    //
                    MinerMessage::MineGoldenTicket => {
                        let blockchain = blockchain_lock.read().await;
                        let miner = miner_lock.read().await;
                        let target = blockchain.get_latest_block_hash();
                        miner.mine(target).await;
                    },
                    _ => {}
                }
            }


            //
            // Saito Channel Messages
            //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    SaitoMessage::BlockchainNewLongestChainBlock { hash : block_hash } => {
                        let mut miner = miner_lock.write().await;
                        miner.set_target(block_hash);
                        miner.set_is_active(true);
                    },
                    _ => {}
                }
            }

        }
    }
}

mod test {}
