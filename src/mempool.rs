use crate::{blockchain::Blockchain, consensus::SaitoMessage};
use ::std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::broadcast;
use tokio::sync::RwLock;

pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    mempool_channel_sender: broadcast::Sender<SaitoMessage>,
    mut mempool_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    // current substitute for some calculation to try and bundle
    // based on the burnfee curve
    tokio::spawn(async move {
        loop {
            mempool_channel_sender
                .send(SaitoMessage::TryBundle)
                .expect("error: TryBundle message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    while let Ok(message) = mempool_channel_receiver.recv().await {
        let mut mempool = mempool_lock.write().await;
        match message {
            SaitoMessage::TryBundle => {
                if mempool.can_bundle(blockchain_lock.clone()) {
                    mempool.bundle_block(blockchain_lock.clone());
                }
            }
        }
    }

    Ok(())
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

    pub fn can_bundle(&self, _blockchain_lock: Arc<RwLock<Blockchain>>) -> bool {
        true
    }

    pub fn bundle_block(&mut self, _blockchain_lock: Arc<RwLock<Blockchain>>) {
        println!("Bundling a Block!");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mempool_new_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_can_bundle_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_bundle_block_test() {
        assert_eq!(true, true);
    }
}
