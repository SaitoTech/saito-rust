//
// TestManager provides a set of functions to simplify testing. It's goal is to
// help make tests more succinct.
//
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::crypto::{SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoUTXOSetKey};
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::transaction::Transaction;
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
        println!("ADDING BLOCK 2! {:?}", parent_hash);
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
        return block;
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
        let block = self
            .generate_block(
                parent_hash,
                timestamp,
                vip_txs,
                normal_txs,
                has_golden_ticket,
                additional_txs,
            )
            .await;

        println!("ADDING BLOCK! {}", block.get_id());

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
            let blk = blockchain.get_block(parent_hash).await;
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
        let mut block = Block::generate(
            &mut transactions,
            parent_hash,
            self.wallet_lock.clone(),
            self.blockchain_lock.clone(),
            timestamp,
        )
        .await;

        block.sign(publickey, privatekey);

        return block;
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
            let block = blockchain.get_block(block_hash).await;
            for i in 0..block.transactions.len() {
                block.transactions[i].on_chain_reorganization(&mut utxoset, true, i as u64);
            }
        }

        //
        // check main utxoset matches longest-chain
        //
        for (key, value) in &blockchain.utxoset {
            println!("{:?} / {}", key, value);
            match utxoset.get(key) {
                Some(value2) => {
                    //
                    // everything spendable in blockchain.utxoset should be spendable on longest-chain
                    //
                    if *value == 1 {
			println!("for key: {:?}", key);
                        println!("comparing {} and {}", value, value2);
                        assert_eq!(value, value2);
                    } else {
                        //
                        // everything spent in blockchain.utxoset should be spent on longest-chain
                        //
                        if *value > 1 {
                            println!("comparing {} and {}", value, value2);
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

        //
        // check longest-chain matches utxoset
        //
        for (key, value) in &utxoset {
            println!("{:?} / {}", key, value);
            match blockchain.utxoset.get(key) {
                Some(value2) => {
                    //
                    // everything spendable in longest-chain should be spendable on blockchain.utxoset
                    //
                    if *value == 1 {
                        println!("comparing {} and {}", value, value2);
                        assert_eq!(value, value2);
                    } else {
                        //
                        // everything spent in longest-chain should be spendable on blockchain.utxoset
                        //
                        if *value > 1 {
                            println!("comparing {} and {}", value, value2);
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
        /***
                let mut token_supply: u64 = 0;
                let mut current_supply: u64 = 0;
                let mut block_inputs: u64 = 0;
                let mut block_outputs: u64 = 0;
                let mut last_block_treasury: u64 = 0;
                let mut last_block_staking_treasury: u64 = 0;
                let mut unpaid_but_uncollected1: u64 = 0;
                let mut unpaid_but_uncollected2: u64 = 0;
                let mut unpaid_but_uncollected3: u64 = 0;
                let mut block_contains_fee_tx: u64 = 0;
                let mut block_contains_gt_tx: u64 = 0;
                let mut block_fee_tx_idx: u64 = 0;
                let mut block_gt_tx_idx: u64 = 0;

                let blockchain = self.blockchain_lock.read().await;
                let latest_block_id = blockchain.get_latest_block_id();

                for i in 1..=latest_block_id {
                    let block_hash = blockchain
                        .blockring
                        .get_longest_chain_block_hash_by_block_id(i as u64);
                    let block = blockchain.get_block(block_hash).await;

                    block_inputs = 0;
                    block_outputs = 0;
                    block_contains_fee_tx = 0;
                    block_contains_gt_tx = 0;

                    //
                    //
                    //
                    for i in 0..block.transactions.len() {
                        //
                        // we ignore the inputs in staking / fee transactions as they have
                        // been pulled from the staking treasury and are already technically
                        // counted in the money supply as an output from a previous slip.
                        // we only care about the difference in token supply represented by
                        // the difference in the staking_treasury.
                        //
                        if block.transactions[i].get_transaction_type() == TransactionType::GoldenTicket {
                            block_contains_gt_tx = 1;
                            block_gt_tx_idx = i as u64;
                        }
                        if block.transactions[i].get_transaction_type() == TransactionType::Fee {
                            block_contains_fee_tx = 1;
                            block_fee_tx_idx = i as u64;
                        }
                        if block.transactions[i].get_transaction_type() != TransactionType::Fee {
                            for z in 0..block.transactions[i].inputs.len() {
                                block_inputs += block.transactions[i].inputs[z].get_amount();
                            }
                        }
                        for z in 0..block.transactions[i].outputs.len() {
                            block_outputs += block.transactions[i].outputs[z].get_amount();
                        }
                    }

                    println!(
                        "Block {} --- Golden Ticket? {}",
                        block.get_id(),
                        block_contains_gt_tx
                    );

                    unpaid_but_uncollected3 = unpaid_but_uncollected2;
                    unpaid_but_uncollected2 = unpaid_but_uncollected1;
                    unpaid_but_uncollected1 = 0;

                    if block_inputs > block_outputs {
                        unpaid_but_uncollected1 = block_inputs - block_outputs;
                    }

                    //
                    // token supply set in block #1
                    //
                    if i == 1 {
                        token_supply = block_outputs + block.get_treasury() + block.get_staking_treasury();
                        current_supply = token_supply;

                        println!("initial supply: {}", token_supply);

                    //
                    // if outputs > inputs, difference will be staking treasury and treasury
                    //
                    } else {
                        println!("block inputs {}", block_inputs);
                        println!("block outputs {}", block_outputs);
                        println!("lbt {}", last_block_treasury);
                        println!("lbst {}", last_block_staking_treasury);
                        println!("t {}", block.get_treasury());
                        println!("st {}", block.get_staking_treasury());
                        println!("ubu1: {}", unpaid_but_uncollected1);
                        println!("ubu2: {}", unpaid_but_uncollected2);
                        println!("ubu3: {}", unpaid_but_uncollected3);

                        current_supply -= block_inputs;
                        current_supply += block_outputs;
                        current_supply -= last_block_staking_treasury;
                        current_supply += block.get_staking_treasury();
                        current_supply -= last_block_treasury;
                        current_supply += block.get_treasury();

                        //
                        // staking treasury and treasury collect payment after 2 blocks
                        // staking treasury
                        //
                        if block_contains_gt_tx == 1 {
                            if block_contains_fee_tx == 1 {
                                println!("block contains fee tx?");
                                //
                                // miner and router payment this block
                                //
                                //current_supply += unpaid_but_uncollected1;
                            }

                            //
                            // WITH STAKING PAYOUT
                            //
                            if unpaid_but_uncollected2 > 0 {
                                println!("UNPAID BUT UNCOLLECTED!: {}", unpaid_but_uncollected2);

                                //
                                // remove any token supply outstanding from previous block
                                //
                                current_supply -= unpaid_but_uncollected2;

                                //
                                // these payments now show up in outputs / staking_treasury
                                //
                                let router_part = unpaid_but_uncollected2 / 2;
                                let _staker_part = unpaid_but_uncollected2 - router_part;

                                //
                                // and the block is dealt with
                                //
                                unpaid_but_uncollected2 = 0;

                                println!("giving us current supply of {}", current_supply);
                            }

                            //
                            // everything else is issued....
                            //
                            unpaid_but_uncollected1 = 0;
                        } else {
                            // add new amount
                            current_supply += unpaid_but_uncollected1;

                            println!(
                                "and we add upbu1 to current_supply: {} -- {}",
                                unpaid_but_uncollected1, current_supply
                            );

                            //
                            // this will be in the treasury now as it has fallen
                            // too far back to be collected.
                            //
                            if unpaid_but_uncollected3 > 0 {
                                current_supply -= unpaid_but_uncollected3;
                            }
                        }
                    }

                    last_block_staking_treasury = block.get_staking_treasury();
                    last_block_treasury = block.get_treasury();

                    //
                    // we check that overall token supply has not changed
                    //
                    println!("checking token supply in block {}", i);
                    assert_eq!(current_supply, token_supply);
                }

                assert_eq!(1, 0);
        ***/
    }
}

#[cfg(test)]
mod tests {}
