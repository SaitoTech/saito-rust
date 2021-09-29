//
// TestManager provides a set of functions to simplify testing. It's goal is to
// help make tests more succinct.
//
use crate::block::{Block, BlockType};
use crate::blockchain::Blockchain;
use crate::burnfee::HEARTBEAT;
use crate::crypto::{SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoUTXOSetKey};
use crate::golden_ticket::GoldenTicket;
use crate::mempool::Mempool;
use crate::miner::Miner;
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionType};
use crate::wallet::Wallet;
use ahash::AHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

//
//
// generate_block 		<-- create a block
// generate_block_and_metadata 	<--- create block with metadata (difficulty, has_golden ticket, etc.)
// generate_transaction 	<-- create a transaction
// add_block 			<-- create and add block to longest_chain
// add_block_on_hash		<-- create and add block elsewhere on chain
// on_chain_reorganization 	<-- test monetary policy
//
//

#[derive(Debug, Clone)]
pub struct TestManager {
    pub mempool_lock: Arc<RwLock<Mempool>>,
    pub blockchain_lock: Arc<RwLock<Blockchain>>,
    pub wallet_lock: Arc<RwLock<Wallet>>,
    pub latest_block_hash: SaitoHash,
}

impl TestManager {
    pub fn new(blockchain_lock: Arc<RwLock<Blockchain>>, wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Self {
            mempool_lock: Arc::new(RwLock::new(Mempool::new(wallet_lock.clone()))),
            blockchain_lock: blockchain_lock.clone(),
            wallet_lock: wallet_lock.clone(),
            latest_block_hash: [0; 32],
        }
    }

    //
    // add block at end of longest chain
    //
    pub async fn add_block(
        &mut self,
        timestamp: u64,
        vip_txs: usize,
        normal_txs: usize,
        has_golden_ticket: bool,
        additional_txs: Vec<Transaction>,
    ) {
        let parent_hash = self.latest_block_hash;
        //println!("ADDING BLOCK 2! {:?}", parent_hash);
        let _block = self
            .add_block_on_hash(
                timestamp,
                vip_txs,
                normal_txs,
                has_golden_ticket,
                additional_txs,
                parent_hash,
            )
            .await;
    }

    //
    // add block on parent hash
    //
    pub async fn add_block_on_hash(
        &mut self,
        timestamp: u64,
        vip_txs: usize,
        normal_txs: usize,
        has_golden_ticket: bool,
        additional_txs: Vec<Transaction>,
        parent_hash: SaitoHash,
    ) -> SaitoHash {
        let mut block = self
            .generate_block_and_metadata(
                parent_hash,
                timestamp,
                vip_txs,
                normal_txs,
                has_golden_ticket,
                additional_txs,
            )
            .await;

        let privatekey: SaitoPrivateKey;
        let publickey: SaitoPublicKey;

        {
            let wallet = self.wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }
        block.sign(publickey, privatekey);

        self.latest_block_hash = block.get_hash();
        Blockchain::add_block_to_blockchain(self.blockchain_lock.clone(), block).await;
        self.latest_block_hash
    }

    //
    // generate_blockchain can be used to add multiple chains of blocks that are not
    // on the longest-chain, and thus will attempt to create transactions that reflect
    // the UTXOSET on the longest-chain at their time of creation.
    //
    // in order to prevent this from being a problem, this function is limited to
    // including a golden ticket every block so as to ensure there is no staking
    // payout in fork-block-5 that is created given expected state of staking table
    // on the other fork. There are no NORMAL transactions permitted and the golden
    // ticket is required every block.
    //
    pub async fn generate_blockchain(
        &mut self,
        chain_length: u64,
        starting_hash: SaitoHash,
    ) -> SaitoHash {
        let mut current_timestamp = create_timestamp();
        let mut parent_hash = starting_hash;

        {
            if parent_hash != [0; 32] {
                let blockchain = self.blockchain_lock.read().await;
                let block_option = blockchain.get_block(&parent_hash).await;
                let block = block_option.unwrap();
                current_timestamp = block.get_timestamp() + 120000;
            }
        }

        for i in 0..chain_length as u64 {
            let mut vip_txs = 10;
            if i > 0 {
                vip_txs = 0;
            }

            let normal_txs = 0;
            //let mut normal_txs = 1;
            //if i == 0 { normal_txs = 0; }
            //normal_txs = 0;

            let mut has_gt = true;
            if parent_hash == [0; 32] {
                has_gt = false;
            }
            //if i%2 == 0 { has_gt = false; }

            parent_hash = self
                .add_block_on_hash(
                    current_timestamp + (i * 120000),
                    vip_txs,
                    normal_txs,
                    has_gt,
                    vec![],
                    parent_hash,
                )
                .await;
        }

        parent_hash
    }

    //
    // generate block
    //
    pub async fn generate_block(
        &self,
        parent_hash: SaitoHash,
        timestamp: u64,
        vip_transactions: usize,
        normal_transactions: usize,
        golden_ticket: bool,
        additional_transactions: Vec<Transaction>,
    ) -> Block {
        let mut transactions: Vec<Transaction> = vec![];
        let mut miner = Miner::new(self.wallet_lock.clone());
        let blockchain = self.blockchain_lock.read().await;
        let privatekey: SaitoPrivateKey;
        let publickey: SaitoPublicKey;

        {
            let wallet = self.wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }

        if 0 < vip_transactions {
            let mut tx = Transaction::generate_vip_transaction(
                self.wallet_lock.clone(),
                publickey,
                10_000_000,
                vip_transactions as u64,
            )
            .await;
            tx.generate_metadata(publickey);
            tx.sign(privatekey);
            transactions.push(tx);
        }

        for _i in 0..normal_transactions {
            let mut transaction =
                Transaction::generate_transaction(self.wallet_lock.clone(), publickey, 5000, 5000)
                    .await;
            // sign ...
            transaction.sign(privatekey);
            transaction.generate_metadata(publickey);
            transactions.push(transaction);
        }

        for i in 0..additional_transactions.len() {
            transactions.push(additional_transactions[i].clone());
        }

        if golden_ticket {
            let blk = blockchain.get_block(&parent_hash).await.unwrap();
            let last_block_difficulty = blk.get_difficulty();
            let golden_ticket: GoldenTicket = miner
                .mine_on_block_until_golden_ticket_found(parent_hash, last_block_difficulty)
                .await;
            let mut tx2: Transaction;
            {
                let mut wallet = self.wallet_lock.write().await;
                tx2 = wallet.create_golden_ticket_transaction(golden_ticket).await;
            }
            tx2.generate_metadata(publickey);
            transactions.push(tx2);
        }

        //
        // create block
        //
        let block = Block::generate(
            &mut transactions,
            parent_hash,
            self.wallet_lock.clone(),
            self.blockchain_lock.clone(),
            timestamp,
        )
        .await;

        block
    }

    pub async fn generate_block_and_metadata(
        &self,
        parent_hash: SaitoHash,
        timestamp: u64,
        vip_transactions: usize,
        normal_transactions: usize,
        golden_ticket: bool,
        additional_transactions: Vec<Transaction>,
    ) -> Block {
        let mut block = self
            .generate_block(
                parent_hash,
                timestamp,
                vip_transactions,
                normal_transactions,
                golden_ticket,
                additional_transactions,
            )
            .await;
        block.generate_metadata();
        block
    }

    pub async fn generate_block_via_mempool(&self) -> Block {
        let latest_block_hash;
        let mut latest_block_timestamp = 0;

        let transaction = self.generate_transaction(1000, 1000).await;
        {
            let mut mempool = self.mempool_lock.write().await;
            mempool.add_transaction(transaction).await;
        }

        // get timestamp of previous block
        {
            let blockchain = self.blockchain_lock.read().await;
            latest_block_hash = blockchain.get_latest_block_hash();
            if latest_block_hash != [0; 32] {
                let block = blockchain.get_block(&latest_block_hash).await;
                latest_block_timestamp = block.unwrap().get_timestamp();
            }
        }

        let next_block_timestamp = latest_block_timestamp + (HEARTBEAT * 2);

        let block_option = crate::mempool::try_bundle_block(
            self.mempool_lock.clone(),
            self.blockchain_lock.clone(),
            next_block_timestamp,
        )
        .await;

        assert!(block_option.is_some());
        block_option.unwrap()
    }

    //
    // generate transaction
    //
    pub async fn generate_transaction(&self, amount: u64, fee: u64) -> Transaction {
        let publickey: SaitoPublicKey;
        let privatekey: SaitoPrivateKey;

        {
            let wallet = self.wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }

        let mut transaction =
            Transaction::generate_transaction(self.wallet_lock.clone(), publickey, amount, fee)
                .await;
        transaction.sign(privatekey);
        transaction
    }

    //
    // check that the blockchain connects properly
    //
    pub async fn check_blockchain(&self) {
        let blockchain = self.blockchain_lock.read().await;

        for i in 1..blockchain.blocks.len() {
            let block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(i as u64);
            let previous_block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id((i as u64) - 1);

            let block = blockchain.get_block_sync(&block_hash);
            let previous_block = blockchain.get_block_sync(&previous_block_hash);

            assert_eq!(block.is_none(), false);
            if i != 1 && previous_block_hash != [0; 32] {
                assert_eq!(previous_block.is_none(), false);
                assert_eq!(
                    block.unwrap().get_previous_block_hash(),
                    previous_block.unwrap().get_hash()
                );
            }
        }
    }

    //
    // check that everything spendable in the main UTXOSET is spendable on the longest
    // chain and vice-versa.
    //
    pub async fn check_utxoset(&self) {
        let blockchain = self.blockchain_lock.read().await;
        let mut utxoset: AHashMap<SaitoUTXOSetKey, u64> = AHashMap::new();
        let latest_block_id = blockchain.get_latest_block_id();

        println!("----");
        println!("----");
        println!("---- check utxoset ");
        println!("----");
        println!("----");
        for i in 1..=latest_block_id {
            let block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(i as u64);
            println!("WINDING ID HASH - {} {:?}", i, block_hash);
            let block = blockchain.get_block(&block_hash).await.unwrap();
            for j in 0..block.get_transactions().len() {
                block.get_transactions()[j].on_chain_reorganization(&mut utxoset, true, i as u64);
            }
        }

        //
        // check main utxoset matches longest-chain
        //
        for (key, value) in &blockchain.utxoset {
            match utxoset.get(key) {
                Some(value2) => {
                    //
                    // everything spendable in blockchain.utxoset should be spendable on longest-chain
                    //
                    if *value == 1 {
                        //println!("for key: {:?}", key);
                        //println!("comparing {} and {}", value, value2);
                        assert_eq!(value, value2);
                    } else {
                        //
                        // everything spent in blockchain.utxoset should be spent on longest-chain
                        //
                        if *value > 1 {
                            println!("comparing key: {:?}", key);
                            println!("comparing blkchn {} and sanitycheck {}", value, value2);
                            assert_eq!(value, value2);
                        } else {
                            //
                            // unspendable (0) does not need to exist
                            //
                        }
                    }
                }
                None => {
                    //
                    // if the value is 0, the token is unspendable on the main chain and
                    // it may still be in the UTXOSET simply because it was not removed
                    // but rather set to an unspendable value. These entries will be
                    // removed on purge, although we can look at deleting them on unwind
                    // as well if that is reasonably efficient.
                    //
                    if *value > 0 {
                        println!("Value does not exist in actual blockchain!");
                        println!("comparing {:?} with on-chain value {}", key, value);
                        assert_eq!(1, 2);
                    }
                }
            }
        }

        //
        // check longest-chain matches utxoset
        //
        for (key, value) in &utxoset {
            //println!("{:?} / {}", key, value);
            match blockchain.utxoset.get(key) {
                Some(value2) => {
                    //
                    // everything spendable in longest-chain should be spendable on blockchain.utxoset
                    //
                    if *value == 1 {
                        //                        println!("comparing {} and {}", value, value2);
                        assert_eq!(value, value2);
                    } else {
                        //
                        // everything spent in longest-chain should be spendable on blockchain.utxoset
                        //
                        if *value > 1 {
                            //                            println!("comparing {} and {}", value, value2);
                            assert_eq!(value, value2);
                        } else {
                            //
                            // unspendable (0) does not need to exist
                            //
                        }
                    }
                }
                None => {
                    println!("comparing {:?} with expected value {}", key, value);
                    println!("Value does not exist in actual blockchain!");
                    assert_eq!(1, 2);
                }
            }
        }
    }

    pub async fn check_token_supply(&self) {
        let mut token_supply: u64 = 0;
        let mut current_supply: u64 = 0;
        let mut block_inputs: u64;
        let mut block_outputs: u64;
        let mut previous_block_treasury: u64;
        let mut current_block_treasury: u64 = 0;
        let mut unpaid_but_uncollected: u64 = 0;
        let mut block_contains_fee_tx: u64;
        let mut block_fee_tx_idx: usize = 0;

        let blockchain = self.blockchain_lock.read().await;
        let latest_block_id = blockchain.get_latest_block_id();

        for i in 1..=latest_block_id {
            let block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(i as u64);
            let block = blockchain.get_block(&block_hash).await.unwrap();

            block_inputs = 0;
            block_outputs = 0;
            block_contains_fee_tx = 0;

            previous_block_treasury = current_block_treasury;
            current_block_treasury = block.get_treasury();

            for t in 0..block.get_transactions().len() {
                //
                // we ignore the inputs in staking / fee transactions as they have
                // been pulled from the staking treasury and are already technically
                // counted in the money supply as an output from a previous slip.
                // we only care about the difference in token supply represented by
                // the difference in the staking_treasury.
                //
                if block.get_transactions()[t].get_transaction_type() == TransactionType::Fee {
                    block_contains_fee_tx = 1;
                    block_fee_tx_idx = t as usize;
                } else {
                    for z in 0..block.get_transactions()[t].inputs.len() {
                        block_inputs += block.get_transactions()[t].inputs[z].get_amount();
                    }
                    for z in 0..block.get_transactions()[t].outputs.len() {
                        block_outputs += block.get_transactions()[t].outputs[z].get_amount();
                    }
                }

                //
                // block one sets circulation
                //
                if i == 1 {
                    token_supply =
                        block_outputs + block.get_treasury() + block.get_staking_treasury();
                    current_supply = token_supply;
                } else {
                    //
                    // figure out how much is in circulation
                    //
                    if block_contains_fee_tx == 0 {
                        current_supply -= block_inputs;
                        current_supply += block_outputs;

                        unpaid_but_uncollected += block_inputs;
                        unpaid_but_uncollected -= block_outputs;

                        //
                        // treasury increases must come here uncollected
                        //
                        if current_block_treasury > previous_block_treasury {
                            unpaid_but_uncollected -=
                                current_block_treasury - previous_block_treasury;
                        }
                    } else {
                        //
                        // calculate total amount paid
                        //
                        let mut total_fees_paid: u64 = 0;
                        let fee_transaction = &block.get_transactions()[block_fee_tx_idx];
                        for output in fee_transaction.get_outputs() {
                            total_fees_paid += output.get_amount();
                        }

                        current_supply -= block_inputs;
                        current_supply += block_outputs;
                        current_supply += total_fees_paid;

                        unpaid_but_uncollected += block_inputs;
                        unpaid_but_uncollected -= block_outputs;
                        unpaid_but_uncollected -= total_fees_paid;

                        //
                        // treasury increases must come here uncollected
                        //
                        if current_block_treasury > previous_block_treasury {
                            unpaid_but_uncollected -=
                                current_block_treasury - previous_block_treasury;
                        }
                    }

                    //
                    // token supply should be constant
                    //
                    let total_in_circulation = current_supply
                        + unpaid_but_uncollected
                        + block.get_treasury()
                        + block.get_staking_treasury();

                    //
                    // we check that overall token supply has not changed
                    //
                    assert_eq!(total_in_circulation, token_supply);
                }
            }
        }
    }

    pub fn check_block_consistency(block: &Block) {
        //
        // tests are run with blocks in various stages of
        // completion. in order to ensure that the tests here
        // can be comprehensive, we generate metadata if the
        // pre_hash has not been created.
        //
        let mut block2 = block.clone();

        let serialized_block = block2.serialize_for_net(BlockType::Full);
        let mut deserialized_block = Block::deserialize_for_net(&serialized_block);

        block2.generate_metadata();
        deserialized_block.generate_metadata();
        block2.generate_hashes();
        deserialized_block.generate_hashes();

        assert_eq!(block2.get_id(), deserialized_block.get_id());
        assert_eq!(block2.get_timestamp(), deserialized_block.get_timestamp());
        assert_eq!(
            block2.get_previous_block_hash(),
            deserialized_block.get_previous_block_hash()
        );
        assert_eq!(block.get_creator(), deserialized_block.get_creator());
        assert_eq!(
            block2.get_merkle_root(),
            deserialized_block.get_merkle_root()
        );
        assert_eq!(block2.get_signature(), deserialized_block.get_signature());
        assert_eq!(block2.get_treasury(), deserialized_block.get_treasury());
        assert_eq!(block2.get_burnfee(), deserialized_block.get_burnfee());
        assert_eq!(block2.get_difficulty(), deserialized_block.get_difficulty());
        assert_eq!(
            block2.get_staking_treasury(),
            deserialized_block.get_staking_treasury()
        );
        // assert_eq!(block2.get_total_fees(), deserialized_block.get_total_fees());
        assert_eq!(
            block2.get_routing_work_for_creator(),
            deserialized_block.get_routing_work_for_creator()
        );
        // assert_eq!(block2.get_lc(), deserialized_block.get_lc());
        assert_eq!(
            block2.get_has_golden_ticket(),
            deserialized_block.get_has_golden_ticket()
        );
        assert_eq!(
            block2.get_has_fee_transaction(),
            deserialized_block.get_has_fee_transaction()
        );
        assert_eq!(
            block2.get_golden_ticket_idx(),
            deserialized_block.get_golden_ticket_idx()
        );
        assert_eq!(
            block2.get_fee_transaction_idx(),
            deserialized_block.get_fee_transaction_idx()
        );
        // assert_eq!(block2.get_total_rebroadcast_slips(), deserialized_block.get_total_rebroadcast_slips());
        // assert_eq!(block2.get_total_rebroadcast_nolan(), deserialized_block.get_total_rebroadcast_nolan());
        // assert_eq!(block2.get_rebroadcast_hash(), deserialized_block.get_rebroadcast_hash());
        //
        // in production blocks are required to have at least one transaction
        // but in testing we sometimes have blocks that do not have transactions
        // deserialization sets those blocks to Header blocks by default so we
        // only want to run this test if there are transactions in play
        //
        if block2.get_transactions().len() > 0 {
            assert_eq!(block2.get_block_type(), deserialized_block.get_block_type());
        }
        assert_eq!(block2.get_pre_hash(), deserialized_block.get_pre_hash());
        assert_eq!(block2.get_hash(), deserialized_block.get_hash());

        let hashmap1 = &block2.slips_spent_this_block;
        let hashmap2 = &deserialized_block.slips_spent_this_block;

        //
        for (key, _value) in hashmap1 {
            let value1 = hashmap1.get(key).unwrap();
            let value2 = hashmap2.get(key).unwrap();
            assert_eq!(value1, value2)
        }

        for (key, _value) in hashmap2 {
            let value1 = hashmap1.get(key).unwrap();
            let value2 = hashmap2.get(key).unwrap();
            assert_eq!(value1, value2)
        }
    }

    pub fn set_latest_block_hash(&mut self, latest_block_hash: SaitoHash) {
        self.latest_block_hash = latest_block_hash;
    }
}

#[cfg(test)]
mod tests {}
