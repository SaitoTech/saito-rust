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
use tracing::{event, Level};

//
// In addition to responding to global broadcast messages, the
// mempool has a local broadcast channel it uses to coordinate
// attempts to bundle blocks and notify itself when a block has
// been produced.
//
#[derive(Clone, Debug)]
pub enum MempoolMessage {
    LocalTryBundleBlock,
    LocalNewBlock,
}

/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when the node is allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
#[derive(Debug)]
pub struct Mempool {
    blocks_queue: VecDeque<Block>,
    pub transactions: Vec<Transaction>, // vector so we just copy it over
    routing_work_in_mempool: u64,
    wallet_lock: Arc<RwLock<Wallet>>,
    currently_bundling_block: bool,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
    mempool_publickey: SaitoPublicKey,
    mempool_privatekey: SaitoPrivateKey,
}

impl Mempool {
    #[allow(clippy::new_without_default)]
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Mempool {
            blocks_queue: VecDeque::new(),
            transactions: vec![],
            routing_work_in_mempool: 0,
            wallet_lock,
            currently_bundling_block: false,
            broadcast_channel_sender: None,
            mempool_publickey: [0; 33],
            mempool_privatekey: [0; 32],
        }
    }

    pub fn add_block(&mut self, block: Block) {
        let hash_to_insert = block.get_hash();
        if self
            .blocks_queue
            .iter()
            .any(|block| block.get_hash() == hash_to_insert)
        {
            // do nothing
        } else {
            self.blocks_queue.push_back(block);
        }
    }
    pub async fn add_golden_ticket(&mut self, golden_ticket: GoldenTicket) {
        let mut wallet = self.wallet_lock.write().await;
        let transaction = wallet.create_golden_ticket_transaction(golden_ticket).await;

        if self
            .transactions
            .iter()
            .any(|transaction| transaction.is_golden_ticket())
        {
        } else {
            event!(Level::TRACE, "adding golden ticket to mempool...");
            self.transactions.push(transaction);
        }
    }

    pub async fn add_transaction(&mut self, mut transaction: Transaction) {
        event!(
            Level::INFO,
            "add_transaction {:?}",
            transaction.get_transaction_type()
        );
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

        if self
            .transactions
            .iter()
            .any(|transaction| transaction.get_signature() == tx_sig_to_insert)
        {
        } else {
            self.transactions.push(transaction);
            self.routing_work_in_mempool += routing_work_available_for_me;
        }
    }

    pub async fn bundle_block(
        &mut self,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        current_timestamp: u64,
    ) -> Block {
        let blockchain = blockchain_lock.read().await;
        let previous_block_hash = blockchain.get_latest_block_hash();

        let block = Block::generate(
            &mut self.transactions,
            previous_block_hash,
            self.wallet_lock.clone(),
            blockchain_lock.clone(),
            current_timestamp,
        )
        .await;

        self.routing_work_in_mempool = 0;

        block
    }

    pub async fn can_bundle_block(
        &self,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        current_timestamp: u64,
    ) -> bool {
        if self.currently_bundling_block {
            return false;
        }
        if self.transactions.is_empty() {
            return false;
        }

        let blockchain = blockchain_lock.read().await;

        if let Some(previous_block) = blockchain.get_latest_block() {
            let work_available = self.get_routing_work_available();
            let work_needed = self.get_routing_work_needed(previous_block, current_timestamp);
            let time_elapsed = current_timestamp - previous_block.get_timestamp();
            event!(
                Level::INFO,
                "can_bundle_block. work available: {:?} -- work needed: {:?} -- time elapsed: {:?} ",
                work_available,
                work_needed,
                time_elapsed
            );
            work_available >= work_needed
        } else {
            true
        }
    }

    pub fn delete_transactions(&mut self, transactions: &Vec<Transaction>) {
        let mut tx_hashmap = HashMap::new();
        for transaction in transactions {
            let hash = transaction.get_hash_for_signature();
            tx_hashmap.entry(hash).or_insert(true);
        }

        self.routing_work_in_mempool = 0;

        self.transactions
            .retain(|x| tx_hashmap.contains_key(&x.get_hash_for_signature()) != true);

        for transaction in &self.transactions {
            self.routing_work_in_mempool +=
                transaction.get_routing_work_for_publickey(self.mempool_publickey);
        }
    }

    ///
    /// Calculates the work available in mempool to produce a block
    ///
    pub fn get_routing_work_available(&self) -> u64 {
        if self.routing_work_in_mempool > 0 {
            return self.routing_work_in_mempool;
        }
        0
    }

    //
    // Return work needed in Nolan
    //
    pub fn get_routing_work_needed(&self, previous_block: &Block, current_timestamp: u64) -> u64 {
        let previous_block_timestamp = previous_block.get_timestamp();
        let previous_block_burnfee = previous_block.get_burnfee();

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
    // pub async fn can_bundle_block(
    //     &self,
    //     blockchain_lock: Arc<RwLock<Blockchain>>,
    //     current_timestamp: u64,
    // ) -> bool {
    //     if self.currently_processing_block {
    //         return false;
    //     }
    //     if self.transactions.is_empty() {
    //         return false;
    //     }

    //     let blockchain = blockchain_lock.read().await;

    //     if let Some(previous_block) = blockchain.get_latest_block() {
    //         let work_available = self.calculate_work_available();
    //         let work_needed = self.calculate_work_needed(previous_block, current_timestamp);
    //         let time_elapsed = current_timestamp - previous_block.get_timestamp();
    //         event!(
    //             Level::INFO,
    //             "can_bundle_block. work available: {:?} -- work needed: {:?} -- time elapsed: {:?} ",
    //             work_available,
    //             work_needed,
    //             time_elapsed
    //         );
    //         work_available >= work_needed
    //     } else {
    //         true
    //     }
    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    pub fn set_mempool_publickey(&mut self, publickey: SaitoPublicKey) {
        self.mempool_publickey = publickey;
    }

    pub fn set_mempool_privatekey(&mut self, privatekey: SaitoPrivateKey) {
        self.mempool_privatekey = privatekey;
    }

    pub async fn send_blocks_to_blockchain(
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) {
        let mut mempool = mempool_lock.write().await;
        mempool.currently_bundling_block = true;
        let mut blockchain = blockchain_lock.write().await;
        while let Some(block) = mempool.blocks_queue.pop_front() {
            mempool.delete_transactions(&block.get_transactions());
            blockchain.add_block(block).await;
        }
        mempool.currently_bundling_block = false;
    }
}

pub async fn try_bundle_block(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    current_timestamp: u64,
) -> Option<Block> {
    event!(Level::INFO, "try_bundle_block");
    // We use a boolean here so we can avoid taking the write lock most of the time
    let can_bundle;
    {
        let mempool = mempool_lock.read().await;
        event!(Level::INFO, "got mempool_lock");
        can_bundle = mempool
            .can_bundle_block(blockchain_lock.clone(), current_timestamp)
            .await;
    }
    if can_bundle {
        let mut mempool = mempool_lock.write().await;
        Some(
            mempool
                .bundle_block(blockchain_lock.clone(), current_timestamp)
                .await,
        )
    } else {
        None
    }
}

//
// This initialization function starts a dedicated thread that listens
// for local and global broadcast messages and triggers the necessary
// responses.
//
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    //
    // mempool gets global broadcast channel and keypair
    //
    {
        // do not seize mempool write access
        let mut mempool = mempool_lock.write().await;
        let publickey;
        let privatekey;
        {
            let wallet = mempool.wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }
        mempool.set_broadcast_channel_sender(broadcast_channel_sender.clone());
        mempool.set_mempool_publickey(publickey);
        mempool.set_mempool_privatekey(privatekey);
    }

    //
    // create local broadcast channel
    //
    let (mempool_channel_sender, mut mempool_channel_receiver) = mpsc::channel(4);

    //
    // local channel sender -- send in clone as thread takes ownership
    //
    let bundle_block_sender = mempool_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            bundle_block_sender
                .send(MempoolMessage::LocalTryBundleBlock)
                .await
                .expect("error: LocalTryBundleBlock message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });

    //
    // global and local channel receivers
    //
    loop {
        tokio::select! {

        //
         // local broadcast channel receivers
         //
            Some(message) = mempool_channel_receiver.recv() => {
                match message {

                    //
                    // attempt to bundle block
                    //
                    MempoolMessage::LocalTryBundleBlock => {
            let current_timestamp = create_timestamp();
                        if let Some(block) = try_bundle_block(
                            mempool_lock.clone(),
                            blockchain_lock.clone(),
                            current_timestamp,
                        ).await {
                            let mut mempool = mempool_lock.write().await;
                            mempool.add_block(block);
                            mempool_channel_sender.send(MempoolMessage::LocalNewBlock).await.expect("Failed to send LocalNewBlock message");
                        }

                    },

                    //
                    // attempt to send to blockchain
                    //
                    MempoolMessage::LocalNewBlock => {
                        Mempool::send_blocks_to_blockchain(mempool_lock.clone(), blockchain_lock.clone()).await;
                    },

                }
            }


        //
        // global broadcast channel receivers
        //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    SaitoMessage::MinerNewGoldenTicket { ticket : golden_ticket } => {
                       // when miner produces golden ticket
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
    use crate::{block::Block, test_utilities::test_manager::TestManager, wallet::Wallet};

    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[test]
    fn mempool_new_test() {
        let wallet = Wallet::new();
        let mempool = Mempool::new(Arc::new(RwLock::new(wallet)));
        assert_eq!(mempool.blocks_queue, VecDeque::new());
    }

    #[test]
    fn mempool_add_block_test() {
        let wallet = Wallet::new();
        let mut mempool = Mempool::new(Arc::new(RwLock::new(wallet)));
        let block = Block::new();
        mempool.add_block(block.clone());
        assert_eq!(Some(block), mempool.blocks_queue.pop_front())
    }

    #[tokio::test]
    async fn mempool_bundle_blocks_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());
        let mempool_lock = test_manager.mempool_lock.clone();

        // BLOCK 1 - VIP transactions
        test_manager
            .add_block(create_timestamp(), 3, 0, false, vec![])
            .await;

        {
            let blockchain = blockchain_lock.read().await;
            assert_eq!(blockchain.get_latest_block_id(), 1);
        }

        for _i in 0..4 {
            let block = test_manager.generate_block_via_mempool().await;
            {
                let mut mempool = test_manager.mempool_lock.write().await;
                mempool.add_block(block);
            }
            Mempool::send_blocks_to_blockchain(mempool_lock.clone(), blockchain_lock.clone()).await;
        }

        // check chain consistence
        test_manager.check_blockchain().await;
    }
    // TODO fix this test
    // #[ignore]
    // #[tokio::test]
    // async fn mempool_bundle_and_send_blocks_to_blockchain_test() {
    //     let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
    //     {
    //         let mut wallet = wallet_lock.write().await;
    //         wallet.load_keys("test/testwallet", Some("asdf"));
    //     }
    //     let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    //     let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    //     let publickey;
    //     let mut prev_block;
    //     {
    //         let wallet = wallet_lock.read().await;
    //         publickey = wallet.get_publickey();
    //     }
    //     add_vip_block(
    //         publickey,
    //         [0; 32],
    //         blockchain_lock.clone(),
    //         wallet_lock.clone(),
    //     )
    //     .await;
    //     // make sure to create the mock_timestamp_generator after the VIP block.
    //     let mut mock_timestamp_generator = MockTimestampGenerator::new();
    //     {
    //         let blockchain = blockchain_lock.read().await;
    //         prev_block = blockchain.get_latest_block().unwrap().clone();
    //         assert_eq!(prev_block.get_id(), 1);
    //     }
    // }
    /*******
        #[tokio::test]
        async fn mempool_bundle_and_send_blocks_to_blockchain_test() {
            let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
            let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
            let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));

            let publickey;
            let mut prev_block;
            {
                let wallet = wallet_lock.read().await;
                publickey = wallet.get_publickey();
            }
            add_vip_block(
                publickey,
                [0; 32],
                blockchain_lock.clone(),
                wallet_lock.clone(),
            )
            .await;

            let current_timestamp = create_timestamp();
            {
                let blockchain = blockchain_lock.read().await;
                prev_block = blockchain.get_latest_block().unwrap().clone();
                assert_eq!(prev_block.get_id(), 1);
            }

            for i in 0..4 {
                let block = make_block_with_mempool(
                    current_timestamp + (i * 120000),
                    mempool_lock.clone(),
                    blockchain_lock.clone(),
                    wallet_lock.clone(),
                )
                .await;
                assert_eq!(prev_block.get_hash(), block.get_previous_block_hash());

                let latest_block_id = block.get_id();
                let latest_block_hash = block.get_hash();
                let latest_block_prev_hash = block.get_previous_block_hash();
                {
                    let prev_hash_before = block.get_previous_block_hash();
                    {
                        let mut mempool = mempool_lock.write().await;
                        mempool.add_block(block);
                    }
                    Mempool::send_blocks_to_blockchain(mempool_lock.clone(), blockchain_lock.clone()).await;
                    let blockchain = blockchain_lock.read().await;
                    let latest_block = blockchain.get_latest_block().unwrap();

                    TestManager::check_block_consistency(&latest_block);

                    let serialized_latest_block = latest_block.serialize_for_net(BlockType::Full);
                    let deserialize_latest_block = Block::deserialize_for_net(&serialized_latest_block);
                    assert_eq!(latest_block.get_hash(), deserialize_latest_block.get_hash());

                    let prev_hash_after = latest_block.get_previous_block_hash();
                    assert_eq!(prev_hash_before, prev_hash_after);

                    assert_eq!(latest_block_id, blockchain.get_latest_block_id());
                    assert_eq!(latest_block_hash, blockchain.get_latest_block_hash());
                    assert_eq!(latest_block.get_hash(), blockchain.get_latest_block_hash());

                    assert!(blockchain.get_block_sync(&prev_hash_before).is_some());
                    assert!(blockchain.get_block(&prev_hash_before).await.is_some());

                    assert_eq!(
                        prev_block.get_hash(),
                        blockchain
                            .get_latest_block()
                            .unwrap()
                            .get_previous_block_hash()
                    );
                    assert_eq!(
                        latest_block_prev_hash,
                        blockchain
                            .get_latest_block()
                            .unwrap()
                            .get_previous_block_hash()
                    );
                    prev_block = blockchain.get_latest_block().unwrap().clone();
                }
            }
        }
    *******/
}
