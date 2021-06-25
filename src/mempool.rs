use crate::{
    block::Block, blockchain::Blockchain, consensus::SaitoMessage, crypto::SaitoHash, slip::Slip,
    transaction::Transaction,
};
use ahash::AHashMap;
use std::{sync::Arc, thread::sleep, time::Duration};
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
pub struct Mempool {
    blocks: AHashMap<SaitoHash, Block>,
}

impl Mempool {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Mempool {
            blocks: AHashMap::new(),
        }
    }

    pub fn add_block(&mut self, block: Block) {
        // adds block if it doesn't already exist,
        // else it ignores attempt to add block to AHashMap
        self.blocks.entry(block.get_hash()).or_insert(block);
    }

    pub fn take_block(&mut self, hash: SaitoHash) -> Option<Block> {
        self.blocks.remove(&hash)
    }

    pub fn can_bundle_block(&self, _previous_block: Option<&Block>) -> bool {
        true
    }

    pub fn bundle_block(&mut self, previous_block: Option<&Block>) -> Block {
        let mut block = self.generate_block_from_mempool_transactions(previous_block);
        block.set_hash();
        block
    }

    pub fn generate_block_from_mempool_transactions(
        &mut self,
        previous_block: Option<&Block>,
    ) -> Block {
        let mut block = Block::default();

        if let Some(previous_block) = previous_block {
            block.set_id(previous_block.get_id());
            block.set_previous_block_hash(previous_block.get_hash());
        }

        block.set_hash();

        for i in 0..1000 {
            println!("Creating Transaction {:?}", i);

            let mut transaction = Transaction::default();

            transaction.set_message((0..1024).map(|_| rand::random::<u8>()).collect());

            let mut input1 = Slip::default();
            input1.set_publickey([1; 33]);
            input1.set_amount(1000000);
            input1.set_uuid([1; 64]);

            let mut output1 = Slip::default();
            output1.set_publickey([1; 33]);
            output1.set_amount(1000000);
            output1.set_uuid([1; 64]);

            transaction.add_input(input1);
            transaction.add_output(output1);
        }

        block
    }
}

// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    _broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let (mempool_channel_sender, mut mempool_channel_receiver) = mpsc::channel(4);

    let try_bundle_sender = mempool_channel_sender.clone();

    // loops to trigger messages
    tokio::spawn(async move {
        loop {
            try_bundle_sender
                .send(MempoolMessage::TryBundle)
                .await
                .expect("error: TryBundle message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    loop {
        tokio::select! {
            Some(message) = mempool_channel_receiver.recv() => {
                match message {
                    // the dominant local message is TryBundle, which triggrs
                    // periodic attempts to analyze the state of the mempool
                    // and make blocks if appropriate.
                    MempoolMessage::TryBundle => {
                        let mempool_read = mempool_lock.read().await;
                        let blockchain_read = blockchain_lock.read().await;

                        let previous_block = blockchain_read.get_latest_block();

                        if mempool_read.can_bundle_block(previous_block) {
                            let mut mempool = mempool_lock.write().await;
                            let block = mempool.bundle_block(previous_block);

                            let mut blockchain = blockchain_lock.write().await;
                            blockchain.add_block(block);
                        }
                    },
                }
            }

            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    // triggered when a block is received over the network and
                    // will be added to the `Blockchain`
                    SaitoMessage::MempoolNewBlock { hash } => {
                        let mut mempool = mempool_lock.write().await;
                        if let Some(block) = mempool.take_block(hash) {
                            let mut blockchain = blockchain_lock.write().await;
                            blockchain.add_block(block);
                        }
                    }
                    SaitoMessage::MempoolNewTransaction { transaction: _transaction } => {
                        let mut _mempool = mempool_lock.write().await;
                        println!("NEW TRANSACTION IN MEMPOOL");
                    },
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
