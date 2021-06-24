use crate::{blockchain::Blockchain, consensus::SaitoMessage};
use ::std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum MempoolMessage {
    TryBundle,
}

/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when thenodeis allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {}

impl Mempool {
    pub fn new() -> Self {
        Mempool {}
    }

    pub fn can_bundle_block(&self, _blockchain_lock: Arc<RwLock<Blockchain>>) -> bool {
        true
    }

    pub fn bundle_block(&mut self, _blockchain_lock: Arc<RwLock<Blockchain>>) {
        println!("Bundling a Block!");
    }
}

//
// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
//
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let (mempool_sender, mut mempool_receiver) = mpsc::channel(4);

    tokio::spawn(async move {
        loop {
            mempool_sender
                .send(MempoolMessage::TryBundle)
                .await
                .expect("error: TryBundle message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    loop {
        tokio::select! {
            Some(message) = mempool_receiver.recv() => {
                match message {
                    MempoolMessage::TryBundle => {
                        let mut mempool = mempool_lock.write().await;
                        if mempool.can_bundle_block(blockchain_lock.clone()) {
                            mempool.bundle_block(blockchain_lock.clone());
                            broadcast_channel_sender.send(SaitoMessage::NewBlock).expect("Error sending new block");
                        }
                    }
                }
            }
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    SaitoMessage::MempoolAddBlock=> {
                        let mut _mempool = mempool_lock.write().await;
                        println!("ADD BLOCK TO MEMPOOL");
                    }
                    SaitoMessage::MempoolAddTransaction=> {
                        let mut _mempool = mempool_lock.write().await;
                        println!("ADD TRANSACTION TO MEMPOOL");
                    },
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn mempool_new_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_can_bundle_block_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_bundle_block_test() {
        assert_eq!(true, true);
    }
}
