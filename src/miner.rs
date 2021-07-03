use crate::{
    consensus::SaitoMessage,
    crypto::{generate_random_bytes, hash, SaitoHash},
    golden_ticket::GoldenTicket,
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
    pub difficulty: u64,
    pub wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
}

impl Miner {
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Miner {
        Miner {
            is_active: false,
            target: [0; 32],
            difficulty: 0,
            wallet_lock,
            broadcast_channel_sender: None,
        }
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    fn is_valid_solution(&self, solution: SaitoHash) -> bool {
        let difficulty_order = (self.difficulty as f64 / 1_0000_0000 as f64).round() as usize;
        let difficulty_grain = self.difficulty % 1_0000_0000;

        let random_solution = U256::from_big_endian(&solution[..difficulty_order]);
        let target_solution= U256::from_big_endian(&self.target[..difficulty_order]);
        let difficulty_grain = U256::from(difficulty_grain * 16);

        random_solution >= target_solution
            && (random_solution - target_solution) <= difficulty_grain
    }

    pub async fn mine(&mut self) {
println!("MINE! {}", self.is_active);
        if self.is_active {

            let solution = hash(&generate_random_bytes(32));

            if self.is_valid_solution(solution) {

                {
                    let vote = 0;
                    let wallet = self.wallet_lock.write().await;
                    let gt = GoldenTicket::new(vote, self.target, solution, wallet.get_publickey());

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

    pub fn set_difficulty(&mut self, difficulty: u64) {
        self.difficulty = difficulty;
    }
}

pub async fn run(
    miner_lock: Arc<RwLock<Miner>>,
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
                        let mut miner = miner_lock.write().await;
                        miner.mine().await;
                    },
                    _ => {}
                }
            }


            //
            // Saito Channel Messages
            //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    SaitoMessage::BlockchainNewLongestChainBlock { hash : block_hash, difficulty } => {
                        let mut miner = miner_lock.write().await;
                        miner.set_target(block_hash);
                        miner.set_difficulty(difficulty);
                        miner.set_is_active(true);
                    },
                    _ => {}
                }
            }

        }
    }
}

mod test {

    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[test]
    fn miner_is_valid_solution_test() {
        let target_hash = [1; 32];
        let solution_hash = [4 as u8; 32];

        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mut miner = Miner::new(wallet_lock);

        miner.set_target(target_hash);

        miner.is_valid_solution(solution_hash);

        assert!(true);
    }

}
