use crate::{
    consensus::SaitoMessage,
    crypto::{generate_random_bytes, hash, SaitoHash, SaitoPublicKey},
    golden_ticket::GoldenTicket,
    wallet::Wallet,
};
use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

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

    pub async fn mine(&mut self) {
        if self.is_active {
            let publickey: SaitoPublicKey;

            {
                let wallet = self.wallet_lock.read().await;
                publickey = wallet.get_publickey();
            }

            let random_bytes = hash(&generate_random_bytes(32));
            let solution = GoldenTicket::generate_solution(self.target, random_bytes, publickey);
            if GoldenTicket::is_valid_solution(solution, self.difficulty) {
                {
                    let vote = 0;
                    let gt = GoldenTicket::new(vote, self.target, random_bytes, publickey);

                    if !self.broadcast_channel_sender.is_none() {
                        self.broadcast_channel_sender
                            .as_ref()
                            .unwrap()
                            .send(SaitoMessage::MinerNewGoldenTicket { ticket: gt })
                            .expect("error: MinerNewGoldenTicket message failed to send");
                    }
                }

                // stop mining
                self.set_is_active(false);
            }
        }
    }

    // function used primarily for test fuctions
    pub async fn mine_on_block_until_golden_ticket_found(
        &mut self,
        block_hash: SaitoHash,
        block_difficulty: u64,
    ) -> GoldenTicket {
        let wallet = self.wallet_lock.read().await;
        let publickey = wallet.get_publickey();
        let mut random_bytes = hash(&generate_random_bytes(32));
        let mut solution = GoldenTicket::generate_solution(self.target, random_bytes, publickey);

        while !GoldenTicket::is_valid_solution(solution, block_difficulty) {
            random_bytes = hash(&generate_random_bytes(32));
            solution = GoldenTicket::generate_solution(block_hash, random_bytes, publickey);
        }

        let vote = 0;

        GoldenTicket::new(vote, self.target, random_bytes, publickey)
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

mod test {}
