use crate::{blockchain::Blockchain, consensus::SaitoMessage};
use ::std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::broadcast;
use tokio::sync::RwLock;



/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when thenodeis allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    broadcast_channel_sender:  broadcast::Sender<SaitoMessage>,
}

impl Mempool {
    pub fn new(bcs : broadcast::Sender<SaitoMessage>) -> Self {
        Mempool {
	    broadcast_channel_sender : bcs,
        }
    }

    pub fn can_bundle_block(&self, _blockchain_lock: Arc<RwLock<Blockchain>>) -> bool {
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
    fn mempool_can_bundle_block_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_bundle_block_test() {
        assert_eq!(true, true);
    }
}


//
// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
//
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    mempool_channel_sender: broadcast::Sender<SaitoMessage>,
    mut mempool_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {

    tokio::spawn(async move {
        loop {
            mempool_channel_sender
                .send(SaitoMessage::TryBundle)
                .expect("error: TryBundle message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    while let Ok(message) = mempool_channel_receiver.recv().await {
        match message {
            SaitoMessage::TryBundle => {
                let mut mempool = mempool_lock.write().await;
                if mempool.can_bundle_block(blockchain_lock.clone()) {
                    mempool.bundle_block(blockchain_lock.clone());
                }
            }
        }
    }

    Ok(())
}


