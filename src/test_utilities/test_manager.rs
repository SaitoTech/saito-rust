use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::crypto::{SaitoHash, SaitoPublicKey, SaitoPrivateKey};
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::transaction::Transaction;
use crate::wallet::Wallet;

use std::sync::Arc;
use tokio::sync::RwLock;


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


    pub async fn add_block(&mut self, timestamp: u64, vip_txs: usize, normal_txs: usize, has_golden_ticket: bool, additional_txs: Vec<Transaction>) {

        let mut block = self.generate_block(
            self.latest_block_hash,
            timestamp,
            vip_txs,
            normal_txs,
            has_golden_ticket,
	    additional_txs,
        ).await;
        self.latest_block_hash = block.get_hash();
        Blockchain::add_block_to_blockchain(self.blockchain_lock.clone(), block, true).await;

    }


    pub async fn generate_block(
	&self,
	parent_hash: SaitoHash,
        timestamp: u64,
        vip_transactions: usize,
        normal_transactions: usize,
        golden_ticket: bool,
        additional_transactions: Vec<Transaction>
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
            let mut tx = Transaction::generate_vip_transaction(self.wallet_lock.clone(), publickey, 10_000_000).await;
            tx.generate_metadata(publickey);
            transactions.push(tx);
        }

        for _i in 0..normal_transactions {
            let mut transaction = Transaction::generate_transaction(self.wallet_lock.clone(), publickey, 5000, 5000).await;
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
        let block = Block::generate(
            &mut transactions,
            parent_hash,
            self.wallet_lock.clone(),
            self.blockchain_lock.clone(),
            timestamp,
        )
        .await;

        return block;
    }


}

#[cfg(test)]
mod tests {}
