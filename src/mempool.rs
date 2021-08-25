use crate::{
    block::Block,
    blockchain::Blockchain,
    burnfee::BurnFee,
    consensus::SaitoMessage,
    crypto::{SaitoPrivateKey, SaitoPublicKey},
    golden_ticket::GoldenTicket,
    time::create_timestamp,
    transaction::Transaction,
    wallet::Wallet,
};
use std::{collections::HashMap, collections::VecDeque, sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum MempoolMessage {
    TryBundleBlock,
    GenerateBlock,
    // GenerateTransaction,
    ProcessBlocks,
}

#[derive(Clone, PartialEq)]
pub enum AddBlockResult {
    Accepted,
    Exists,
}
#[derive(Clone, PartialEq)]
pub enum AddTransactionResult {
    Accepted,
    Rejected,
    Invalid,
    Exists,
}

/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when thenodeis allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    blocks: VecDeque<Block>,
    pub transactions: Vec<Transaction>, // vector so we just copy it over
    routing_work_in_mempool: u64,
    wallet_lock: Arc<RwLock<Wallet>>,
    currently_processing_block: bool,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,

    mempool_publickey: SaitoPublicKey,
    mempool_privatekey: SaitoPrivateKey,
}

impl Mempool {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Mempool {
            blocks: VecDeque::new(),
            transactions: vec![],
            routing_work_in_mempool: 0,
            wallet_lock,
            currently_processing_block: false,
            broadcast_channel_sender: None,
            mempool_publickey: [0; 33],
            mempool_privatekey: [0; 32],
        }
    }

    pub fn set_mempool_publickey(&mut self, publickey: SaitoPublicKey) {
        self.mempool_publickey = publickey;
    }

    pub fn set_mempool_privatekey(&mut self, privatekey: SaitoPrivateKey) {
        self.mempool_privatekey = privatekey;
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    pub fn add_block(&mut self, block: Block) -> AddBlockResult {
        let hash_to_insert = block.get_hash();
        if self
            .blocks
            .iter()
            .any(|block| block.get_hash() == hash_to_insert)
        {
            return AddBlockResult::Exists;
        } else {
            self.blocks.push_back(block);
            return AddBlockResult::Accepted;
        }
    }

    // this handles any transactions broadcast on the same system. it receives the transaction
    // directly and so does not validate against the UTXOset etc.
    pub async fn add_transaction(&mut self, mut transaction: Transaction) -> AddTransactionResult {
        let tx_sig_to_insert = transaction.get_signature();

        //
        // this assigns the amount of routing work that this transaction
        // contains to us, which is why we need to provide our publickey
        // so that we can calculate routing work.
        //
        let publickey;
        {
            let wallet = self.wallet_lock.read().await;
            publickey = wallet.get_publickey();
        }
        transaction.generate_metadata(publickey);

        let routing_work_available_for_me =
            transaction.get_routing_work_for_publickey(self.mempool_publickey);

        //        println!("adding tx with routing work for me: {}", routing_work_available_for_me);

        if self
            .transactions
            .iter()
            .any(|transaction| transaction.get_signature() == tx_sig_to_insert)
        {
            return AddTransactionResult::Exists;
        } else {
            self.transactions.push(transaction);
            self.routing_work_in_mempool += routing_work_available_for_me;
            return AddTransactionResult::Accepted;
        }
    }

    //
    // when we generate a block ourselves, we automatically prune the mempool but
    // when we accept a block produced by others this may not be the case. So we
    // have this function to manually remove the transactions in the blocks we
    // are adding if their hash_for_signature matches.
    //
    pub fn delete_transactions(&mut self, transactions: &Vec<Transaction>) {
        let mut tx_hashmap = HashMap::new();

        for transaction in transactions {
            let hash = transaction.get_hash_for_signature();
            tx_hashmap.entry(hash).or_insert(true);
        }

        //
        // TODO
        //
        // reorder the transactions vector so that all of the
        // elements we want to delete are at the front, then
        // sweep them out in a single operation.
        //
        // for now just delete
        //
        self.routing_work_in_mempool = 0;
        self.transactions
            .retain(|x| tx_hashmap.contains_key(&x.get_hash_for_signature()) != true);

        for transaction in &self.transactions {
            self.routing_work_in_mempool +=
                transaction.get_routing_work_for_publickey(self.mempool_publickey);
        }
    }

    // this handles golden tickets broadcast on the same system. it wraps them
    // in a transaction and then saves them in the mempool if they are targetting
    // the right block. this pushes the golden ticket indiscriminately into the
    // transaction array -- we can do a better job.
    pub async fn add_golden_ticket(&mut self, golden_ticket: GoldenTicket) -> AddTransactionResult {
        // convert into transaction
        let mut wallet = self.wallet_lock.write().await;
        // hash_for_signature generated in creating txs
        let transaction = wallet.create_golden_ticket_transaction(golden_ticket).await;

        if self
            .transactions
            .iter()
            .any(|transaction| transaction.is_golden_ticket())
        {
            return AddTransactionResult::Exists;
        } else {
            println!("adding golden ticket to mempool...");
            self.transactions.push(transaction);
            return AddTransactionResult::Accepted;
        }
    }

    ///
    /// Calculates the work available in mempool to produce a block
    ///
    pub fn calculate_work_available(&self) -> u64 {
        if self.routing_work_in_mempool > 0 {
            return self.routing_work_in_mempool;
        }
        return 0;
    }

    //
    // Return work needed in Nolan
    //
    pub fn calculate_work_needed(&self, previous_block: &Block) -> u64 {
        let previous_block_timestamp = previous_block.get_timestamp();
        let previous_block_burnfee = previous_block.get_burnfee();
        let current_timestamp = create_timestamp();

        let work_needed: u64 = BurnFee::return_routing_work_needed_to_produce_block_in_nolan(
            previous_block_burnfee,
            current_timestamp,
            previous_block_timestamp,
        );

        work_needed
    }

    ///
    /// Check to see if the `Mempool` has enough work to bundle a block
    ///

    pub async fn can_bundle_block(&self, blockchain_lock: Arc<RwLock<Blockchain>>) -> bool {
        if self.currently_processing_block {
            return false;
        }
        if self.transactions.len() == 0 {
            return false;
        }

        let blockchain = blockchain_lock.read().await;

        if let Some(previous_block) = blockchain.get_latest_block() {
            let work_available = self.calculate_work_available();
            let work_needed = self.calculate_work_needed(previous_block);
            let time_elapsed = create_timestamp() - previous_block.get_timestamp();
            println!(
                "work available: {:?} -- work needed: {:?} -- time elapsed: {:?} ",
                work_available, work_needed, time_elapsed
            );
            work_available >= work_needed
        } else {
            true
        }
    }

    pub async fn generate_block(&mut self, blockchain_lock: Arc<RwLock<Blockchain>>) -> Block {
        let blockchain = blockchain_lock.read().await;
        let previous_block_hash = blockchain.get_latest_block_hash();
        let block = Block::generate(
            &mut self.transactions,
            previous_block_hash,
            self.wallet_lock.clone(),
            blockchain_lock.clone(),
        )
        .await;

        self.routing_work_in_mempool = 0;

        block
    }
}

// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    //
    // mempool gets global broadcast channel
    //
    {
        let mut mempool = mempool_lock.write().await;
        let publickey;
        let privatekey;
        mempool.set_broadcast_channel_sender(broadcast_channel_sender.clone());
        {
            let wallet = mempool.wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }

        mempool.set_mempool_publickey(publickey);
        mempool.set_mempool_privatekey(privatekey);
    }

    // generate blocks 4000, w/ capacity of 4 fails
    let (mempool_channel_sender, mut mempool_channel_receiver) = mpsc::channel(4);
    let generate_block_sender = mempool_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            generate_block_sender
                .send(MempoolMessage::TryBundleBlock)
                .await
                .expect("error: TryBundleBlock message failed to send");
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
                        let can_bundle;
                        {
                            let mempool = mempool_lock.read().await;
                            can_bundle = mempool.can_bundle_block(blockchain_lock.clone()).await;
                        }
                        if can_bundle {
                            let mut mempool = mempool_lock.write().await;
                            let block = mempool.generate_block(blockchain_lock.clone()).await;
                            if AddBlockResult::Accepted == mempool.add_block(block) {
                                mempool_channel_sender.send(MempoolMessage::ProcessBlocks).await.expect("Failed to send ProcessBlocks message");
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


                   // ProcessBlocks will add blocks FIFO from the queue into blockchain
                   MempoolMessage::ProcessBlocks => {

                       let mut mempool = mempool_lock.write().await;
                       mempool.currently_processing_block = true;
                       let mut blockchain = blockchain_lock.write().await;
                       while let Some(block) = mempool.blocks.pop_front() {
                           mempool.delete_transactions(&block.transactions);
                           blockchain.add_block(block, true).await;
                       }
                       mempool.currently_processing_block = false;
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
                        mempool_channel_sender.send(MempoolMessage::ProcessBlocks).await.expect("Failed to send ProcessBlocks message");
                    }
                    SaitoMessage::MempoolNewTransaction { transaction } => {
                        let mut mempool = mempool_lock.write().await;
                        mempool.add_transaction(transaction).await;
                    },
                    SaitoMessage::MinerNewGoldenTicket { ticket : golden_ticket } => {
                        let mut mempool = mempool_lock.write().await;
                        mempool.add_golden_ticket(golden_ticket).await;
                    },
                    _ => {},
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
        let wallet = Wallet::new("test/testwallet", Some("asdf"));
        let mempool = Mempool::new(Arc::new(RwLock::new(wallet)));
        assert_eq!(mempool.blocks, VecDeque::new());
    }

    #[test]
    fn mempool_add_block_test() {
        let wallet = Wallet::new("test/testwallet", Some("asdf"));
        let mut mempool = Mempool::new(Arc::new(RwLock::new(wallet)));

        let block = Block::new();

        mempool.add_block(block.clone());

        assert_eq!(Some(block), mempool.blocks.pop_front())
    }
}
