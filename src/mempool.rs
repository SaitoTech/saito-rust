use crate::{
    block::Block,
    blockchain::Blockchain,
    burnfee::BurnFee,
    consensus::SaitoMessage,
    crypto::{hash, verify},
    slip::Slip,
    time::create_timestamp,
    transaction::Transaction,
    wallet::Wallet,
};
use std::{collections::VecDeque, sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum MempoolMessage {
    TryBundleBlock,
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
    currently_processing_blocks: bool,
}

impl Mempool {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Mempool {
            blocks: VecDeque::new(),
            wallet_lock,
            currently_processing_blocks: false,
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

    ///
    /// Calculates the work available in mempool to produce a block
    ///
    pub async fn calculate_work_available(&self) -> u64 {
        return 2 * 100_000_000;
    }

    //
    // Return work needed in Nolan
    //
    pub async fn calculate_work_needed(&self, blockchain_lock: Arc<RwLock<Blockchain>>) -> u64 {
        let blockchain = blockchain_lock.read().await;
        let previous_block_timestamp = blockchain.get_latest_block_timestamp();
        let current_timestamp = create_timestamp();

        let previous_block_burnfee = blockchain.get_latest_block_burnfee();

        let work_needed: u64 = BurnFee::return_routing_work_needed_to_produce_block_in_nolan(
            previous_block_burnfee,
            current_timestamp,
            previous_block_timestamp,
        );

        println!(
            "time elapsed: {}",
            current_timestamp - previous_block_timestamp
        );

        work_needed
    }

    ///
    /// Check to see if the `Mempool` has enough work to bundle a block
    ///
    pub async fn can_bundle_block(&self, blockchain_lock: Arc<RwLock<Blockchain>>) -> bool {
        if self.currently_processing_blocks {
            return false;
        }
        let work_available = self.calculate_work_available().await;
        let work_needed = self.calculate_work_needed(blockchain_lock).await;

        println!("WA: {:?} -- WN: {:?}", work_available, work_needed);

        if work_available >= work_needed {
            return true;
        }

        return false;
    }

    pub async fn generate_block(&mut self, blockchain_lock: Arc<RwLock<Blockchain>>) -> Block {
        let blockchain = blockchain_lock.read().await;
        let previous_block_id = blockchain.get_latest_block_id();
        let previous_block_hash = blockchain.get_latest_block_hash();

        let mut block = Block::default();

        let previous_block_burnfee = blockchain.get_latest_block_burnfee();
        let previous_block_timestamp = blockchain.get_latest_block_timestamp();
        let current_timestamp = create_timestamp();

        //println!("BUILDING FROM BF: {}", previous_block_burnfee);
        //println!("BUILDING FROM TS: {}", previous_block_timestamp);
        //println!("BUILDING FROM ID: {}", previous_block_id);

        let new_burnfee: u64 =
            BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
                previous_block_burnfee,
                current_timestamp,
                previous_block_timestamp,
            );

        block.set_id(previous_block_id + 1);
        block.set_previous_block_hash(previous_block_hash);
        block.set_burnfee(new_burnfee);

        let wallet = self.wallet_lock.read().await;

        for _i in 0..10 {
            //            println!("creating tx {:?}", _i);

            let mut transaction = Transaction::default();

            transaction.set_message((0..1024).map(|_| rand::random::<u8>()).collect());

            let mut input1 = Slip::default();
            input1.set_publickey(wallet.get_publickey());
            input1.set_amount(1000000);
            input1.set_uuid([1; 64]);

            let mut output1 = Slip::default();
            output1.set_publickey(wallet.get_publickey());
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

            block.add_transaction(transaction);
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
            //          generate_block_sender
            //               .send(MempoolMessage::GenerateBlock)
            //               .await
            //               .expect("error: GenerateBlock message failed to send");
            //          sleep(Duration::from_millis(20000));
            generate_block_sender
                .send(MempoolMessage::TryBundleBlock)
                .await
                .expect("error: GenerateBlock message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    loop {
        tokio::select! {
            Some(message) = mempool_channel_receiver.recv() => {
                match message {

                    // TryBundleBlock makes periodic attempts to produce blocks and does so
            // if the mempool can bundle blocks....
                    MempoolMessage::TryBundleBlock => {
                        let mut mempool = mempool_lock.write().await;
                    let can_bundle = mempool.can_bundle_block(blockchain_lock.clone()).await;
                        if can_bundle {
                            let block = mempool.generate_block(blockchain_lock.clone()).await;
                            if AddBlockResult::Accepted == mempool.add_block(block) {
                                mempool_channel_sender.send(MempoolMessage::ProcessBlocks).await.expect("Failed to send ProcessBlocks message")
                            }
                        }
                    },

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
            mempool.currently_processing_blocks = true;
                        let mut blockchain = blockchain_lock.write().await;
                        while let Some(block) = mempool.blocks.pop_front() {
                            blockchain.add_block(block).await;
                        }
            mempool.currently_processing_blocks = false;
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
