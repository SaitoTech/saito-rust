//
// TestManager provides a set of functions to simplify testing. It's goal is to
// help make tests more succinct.
//
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::crypto::{SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoUTXOSetKey};
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::transaction::{Transaction, TransactionType};
use crate::wallet::Wallet;
use ahash::AHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

//
//
// generate_block 	<-- create a block
// generate_transaction <-- create a transaction
// add_block 		<-- create and add block to longest_chain
// add_block_on_hash	<-- create and add block elsewhere on chain
// on_chain_reorganization <-- test monetary policy
//
//

#[derive(Debug, Clone)]
pub struct TestManager {
    pub blockchain_lock: Arc<RwLock<Blockchain>>,
    pub wallet_lock: Arc<RwLock<Wallet>>,
    pub latest_block_hash: SaitoHash,
}

impl TestManager {
    pub fn new(blockchain_lock: Arc<RwLock<Blockchain>>, wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Self {
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
        let block = self
            .add_block_on_hash(
                timestamp,
                vip_txs,
                normal_txs,
                has_golden_ticket,
                additional_txs,
                parent_hash,
            )
            .await;
        block
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
    ) {
        let mut block = self
            .generate_block(
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

        for _i in 0..vip_transactions {
            let mut tx = Transaction::generate_vip_transaction(
                self.wallet_lock.clone(),
                publickey,
                10_000_000,
            )
            .await;
            tx.generate_metadata(publickey);
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
    // check that everything spendable in the main UTXOSET is spendable on the longest
    // chain and vice-versa.
    //
    pub async fn check_utxoset(&self) {
        let blockchain = self.blockchain_lock.read().await;
        let mut utxoset: AHashMap<SaitoUTXOSetKey, u64> = AHashMap::new();
        let latest_block_id = blockchain.get_latest_block_id();

        for i in 1..=latest_block_id {
            let block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(i as u64);
            let block = blockchain.get_block(&block_hash).await.unwrap();
            for i in 0..block.get_transactions().len() {
                block.get_transactions()[i].on_chain_reorganization(&mut utxoset, true, i as u64);
            }
        }

        //
        // check main utxoset matches longest-chain
        //
        for (key, value) in &blockchain.utxoset {
            //println!("{:?} / {}", key, value);
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
                            //println!("comparing {} and {}", value, value2);
                            assert_eq!(value, value2);
                        } else {
                            //
                            // unspendable (0) does not need to exist
                            //
                        }
                    }
                }
                None => {
                    //                    println!("comparing {:?} with expected value {}", key, value);
                    //                    println!("Value does not exist in actual blockchain!");
                    assert_eq!(1, 2);
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
        let serialized_block = block.serialize_for_net();

        let deserialized_block = Block::deserialize_for_net(&serialized_block);

        assert_eq!(block.get_id(), deserialized_block.get_id());
        assert_eq!(block.get_timestamp(), deserialized_block.get_timestamp());
        assert_eq!(
            block.get_previous_block_hash(),
            deserialized_block.get_previous_block_hash()
        );
        assert_eq!(block.get_creator(), deserialized_block.get_creator());
        assert_eq!(
            block.get_merkle_root(),
            deserialized_block.get_merkle_root()
        );
        assert_eq!(block.get_signature(), deserialized_block.get_signature());
        assert_eq!(block.get_treasury(), deserialized_block.get_treasury());
        assert_eq!(block.get_burnfee(), deserialized_block.get_burnfee());
        assert_eq!(block.get_difficulty(), deserialized_block.get_difficulty());
        assert_eq!(
            block.get_staking_treasury(),
            deserialized_block.get_staking_treasury()
        );
        assert_eq!(block.get_total_fees(), deserialized_block.get_total_fees());
        assert_eq!(
            block.get_routing_work_for_creator(),
            deserialized_block.get_routing_work_for_creator()
        );
        assert_eq!(block.get_lc(), deserialized_block.get_lc());
        assert_eq!(
            block.get_has_golden_ticket(),
            deserialized_block.get_has_golden_ticket()
        );
        assert_eq!(
            block.get_has_fee_transaction(),
            deserialized_block.get_has_fee_transaction()
        );
        assert_eq!(
            block.get_golden_ticket_idx(),
            deserialized_block.get_golden_ticket_idx()
        );
        assert_eq!(
            block.get_fee_transaction_idx(),
            deserialized_block.get_fee_transaction_idx()
        );
        // assert_eq!(block.get_total_rebroadcast_slips(), deserialized_block.get_total_rebroadcast_slips());
        // assert_eq!(block.get_total_rebroadcast_nolan(), deserialized_block.get_total_rebroadcast_nolan());
        // assert_eq!(block.get_rebroadcast_hash(), deserialized_block.get_rebroadcast_hash());
        assert_eq!(block.get_block_type(), deserialized_block.get_block_type());
        // assert_eq!(block.slips_spent_this_block, deserialized_block.slips_spent_this_block);
        assert_eq!(block.get_pre_hash(), deserialized_block.get_pre_hash());
        assert_eq!(block.get_hash(), deserialized_block.get_hash());
    }
}

#[cfg(test)]
mod tests {}
