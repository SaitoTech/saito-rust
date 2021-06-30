use crate::{
    block::Block,
    blockchain::Blockchain,
    consensus::SaitoMessage,
    crypto::{hash, verify},
    slip::Slip,
    transaction::Transaction,
    wallet::Wallet,
};
use std::{collections::VecDeque, sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum MempoolMessage {
    GenerateBlock,
    ProcessBlocks,
}

#[derive(Clone, PartialEq)]
pub enum AddBlockResult {
    Accepted,
    Exists,
}

/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when thenodeis allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    blocks: VecDeque<Block>,
    wallet_lock: Arc<RwLock<Wallet>>,
}

impl Mempool {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Mempool {
            blocks: VecDeque::new(),
            wallet_lock,
        }
    }

    pub fn add_block(&mut self, block: Block) -> AddBlockResult {
        let hash_to_insert = block.get_hash();
        if self
            .blocks
            .iter()
            .any(|block| block.get_hash() == hash_to_insert)
        {
            AddBlockResult::Exists
        } else {
            self.blocks.push_back(block);
            AddBlockResult::Accepted
        }
    }

    pub async fn generate_block(&mut self, blockchain_lock: Arc<RwLock<Blockchain>>) -> Block {
        let blockchain = blockchain_lock.read().await;
        let previous_block_id = blockchain.get_latest_block_id();
        let previous_block_hash = blockchain.get_latest_block_hash();

        let mut block = Block::default();

        block.set_id(previous_block_id + 1);
        block.set_previous_block_hash(previous_block_hash);

        let wallet = self.wallet_lock.read().await;

        for _i in 0..10 {
            println!("creating tx {:?}", _i);

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

            // sign ...
            transaction.sign(wallet.get_privatekey());
            let tx_sig = transaction.get_signature();

            // ... and verify
            let vbytes = transaction.serialize_for_signature();
            let hash = hash(&vbytes);
            let v = verify(&hash, tx_sig, wallet.get_publickey());
            if !v {
                println!("Transaction does not Validate: {:?}", v);
            }
        }

        let block_merkle_root = block.generate_merkle_root();
        block.set_merkle_root(block_merkle_root);
        let block_hash = block.generate_hash();
        block.set_hash(block_hash);

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

    let generate_block_sender = mempool_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            generate_block_sender
                .send(MempoolMessage::GenerateBlock)
                .await
                .expect("error: GenerateBlock message failed to send");
            sleep(Duration::from_millis(5000));
        }
    });

    loop {
        tokio::select! {
            Some(message) = mempool_channel_receiver.recv() => {
                match message {
                    // GenerateBlock makes periodic attempts to analyse the state of
                    // the mempool and produce blocks if possible.
                    MempoolMessage::GenerateBlock => {
                        let mut mempool = mempool_lock.write().await;
                        let block = mempool.generate_block(blockchain_lock.clone()).await;
                        if AddBlockResult::Accepted == mempool.add_block(block) {
                            mempool_channel_sender.send(MempoolMessage::ProcessBlocks).await.expect("Failed to send ProcessBlocks message")
                        }
                    },

                    // ProcessBlocks will add blocks FIFO from the queue
                    // into blockchain
                    MempoolMessage::ProcessBlocks => {
                        let mut mempool = mempool_lock.write().await;
                        let mut blockchain = blockchain_lock.write().await;
                        while let Some(block) = mempool.blocks.pop_front() {
                            blockchain.add_block(block);
                        }
                    },
                }
            }


            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    // triggered when a block is received over the network and
                    // will be added to the `Blockchain`
                    SaitoMessage::MempoolNewBlock { hash: _hash } => {
                        // TODO: there is still an open question about how blocks
                        // over the network will be placed into the mempool queue
                        //
                        // For now, let's assume that the network has a reference
                        // to mempool and is adding the block through that reference
                        // then calls mempool to process the blocks in the queue
                        mempool_channel_sender.send(MempoolMessage::ProcessBlocks).await.expect("Failed to send ProcessBlocks message")
                    }
                    SaitoMessage::MempoolNewTransaction { transaction: _transaction } => {
                        let mut _mempool = mempool_lock.write().await;
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{block::Block, wallet::Wallet};

    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[test]
    fn mempool_new_test() {
        let wallet = Wallet::new();
        let mempool = Mempool::new(Arc::new(RwLock::new(wallet)));
        assert_eq!(mempool.blocks, VecDeque::new());
    }

    #[test]
    fn mempool_add_block_test() {
        let wallet = Wallet::new();
        let mut mempool = Mempool::new(Arc::new(RwLock::new(wallet)));

        let block = Block::default();

        mempool.add_block(block.clone());

        assert_eq!(Some(block), mempool.blocks.pop_front())
    }
}
