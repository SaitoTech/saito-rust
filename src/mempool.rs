use secp256k1::PublicKey;

use crate::{
    block::{Block, BlockCore, TREASURY},
    blockchain::BLOCKCHAIN_GLOBAL,
    burnfee::BurnFee,
    keypair::Keypair,
    time::{create_timestamp, format_timestamp},
    transaction::Transaction,
    types::SaitoMessage,
    utxoset::UtxoSet,
};

use std::sync::{Arc, RwLock};

pub const GENESIS_PERIOD: u64 = 21500;

/// The `Mempool` is the structure that collects blocks and transactions
/// and is control of discerning whether the node is allowed to create a block.
/// It bundles the block, the sends it to `Blockchain` to be added to the longest chain.
/// New `Block`s coming in over the network will hit the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    /// Keypair
    keypair: Arc<RwLock<Keypair>>,
    /// A list of `Transaction`s to be bundled into `Block`s
    transactions: Vec<Transaction>,
}

impl Mempool {
    /// Creates new `Memppol`
    pub fn new(keypair: Arc<RwLock<Keypair>>) -> Self {
        Mempool {
            keypair,
            transactions: vec![],
        }
    }

    /// Processes `SaitoMessage` and attempts to return `Block`
    ///
    /// * `message` - `SaitoMessage` enum commanding `Mempool` operation
    /// * `previous_block` - `Block` at longest chain position in `Blockchain`
    pub fn process(&mut self, message: SaitoMessage) -> Option<Block> {
        match message {
            SaitoMessage::Transaction { payload } => {
                self.transactions.push(payload);
                self.try_bundle()
            }
            SaitoMessage::TryBundle => self.try_bundle(),
            _ => None,
        }
    }

    /// Attempt to create a new `Block`
    ///
    /// * `previous_block` - `Option` of previous `Block`
    fn try_bundle(&mut self) -> Option<Block> {
        if self.can_bundle_block() {
            Some(self.bundle_block())
        } else {
            None
        }
    }

    /// Check to see if the `Mempool` has enough work to bundle a block
    ///
    /// * `previous_block` - `Option` of previous `Block`
    fn can_bundle_block(&self) -> bool {
        println!("can bundle block");
        let work_available = self.calculate_work_available();

        let blockchain_mutex = Arc::clone(&BLOCKCHAIN_GLOBAL);
        let mut blockchain = blockchain_mutex.lock().unwrap();
        let previous_block_option = blockchain.latest_block();

        match previous_block_option {
            Some(previous_block) => {
                let current_timestamp = create_timestamp();
                let work_needed = BurnFee::return_work_needed(
                    previous_block.start_burnfee(),
                    current_timestamp,
                    previous_block.timestamp(),
                );

                println!(
                    "TS: {} -- WORK ---- {:?} -- {:?} --- TX COUNT {:?}",
                    format_timestamp(current_timestamp),
                    work_needed,
                    work_available,
                    self.transactions.len(),
                );
                // TODO -- add check for transactions in Mempool
                work_available >= work_needed
            }
            None => true,
        }
    }

    /// Calculates the work available to pay the network to produce a `Block`
    /// based off of `Transaction` fees in the `Mempool`
    fn calculate_work_available(&self) -> u64 {
        println!("LOCK calculate_work_available");
        let blockchain_mutex = Arc::clone(&BLOCKCHAIN_GLOBAL);
        let blockchain = blockchain_mutex.lock().unwrap();
        let retval = self
            .transactions
            .iter()
            .map(|tx| blockchain.utxoset.transaction_routing_fees(tx))
            .sum();
        println!("RELEASE calculate_work_available");
        retval
    }

    /// Clear the transactions from the `Mempool`
    fn clear_transactions(&mut self) {
        self.transactions = vec![];
    }

    /// Create a new `Block` from the `Mempool`'s list of `Transaction`s
    ///
    /// * `previous_block` - `Option` of the previous block on the longest chain
    fn bundle_block(&mut self) -> Block {
        println!("bundle block");
        let publickey: PublicKey;

        {
            let keypair = self.keypair.read().unwrap();
            publickey = keypair.public_key().clone();
        }

        let block: Block;
        let block_core: BlockCore;

        let blockchain_mutex = Arc::clone(&BLOCKCHAIN_GLOBAL);
        let blockchain = blockchain_mutex.lock().unwrap();
        match blockchain.latest_block() {
            Some(previous_block) => {
                let timestamp = create_timestamp();

                let treasury = previous_block.treasury();
                let coinbase = (treasury as f64 / GENESIS_PERIOD as f64).round() as u64;

                block_core = BlockCore::new(
                    previous_block.id() + 1,
                    timestamp,
                    previous_block.hash(),
                    publickey,
                    coinbase,
                    treasury - coinbase,
                    BurnFee::burn_fee_adjustment_calculation(
                        previous_block.start_burnfee(),
                        timestamp,
                        previous_block.timestamp(),
                    ),
                    0.0,
                    &mut self.transactions,
                );

                block = Block::new(block_core);

                // TODO -- include reclaimed fees here
            }
            None => {
                block_core = BlockCore::new(
                    0,
                    create_timestamp(),
                    [0; 32],
                    publickey.clone(),
                    0,
                    TREASURY,
                    10.0,
                    0.0,
                    &mut vec![],
                );

                block = Block::new(block_core);
            }
        }
        // TODO -- calculate difficulty and paysplit changes
        // https://github.com/orgs/SaitoTech/projects/5#card-61347666

        self.clear_transactions();

        block
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::keypair::Keypair;
    use crate::utxoset::UtxoSet;
    use std::sync::{Arc, Mutex};

    #[test]
    fn mempool_test() {
        assert_eq!(true, true);
        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let mempool = Mempool::new(keypair);
        assert_eq!(mempool.transactions, vec![]);
    }

    // #[test]
    // fn mempool_try_bundle_some_test() {
    //     let keypair = Arc::new(RwLock::new(Keypair::new()));
    //     let utxoset = Arc::new(Mutex::new(UtxoSet::new()));
    //     let mut mempool = Mempool::new(keypair, utxoset);

    //     let new_block = mempool.try_bundle(None);

    //     match new_block {
    //         Some(block) => {
    //             assert_eq!(block.id(), 0);
    //             assert_eq!(*block.previous_block_hash(), [0; 32]);
    //             assert_eq!(mempool.transactions, vec![]);
    //         }
    //         None => {}
    //     }
    // }

    // #[test]
    // fn mempool_try_bundle_none_test() {
    //     let keypair = Arc::new(RwLock::new(Keypair::new()));
    //     let utxoset = Arc::new(Mutex::new(UtxoSet::new()));
    //     let mut mempool = Mempool::new(keypair, utxoset);

    //     let core = BlockCore::default();
    //     let prev_block = Block::new(core);

    //     let new_block = mempool.try_bundle(Some(&prev_block));

    //     match new_block {
    //         Some(_) => {}
    //         None => {
    //             assert_eq!(true, true)
    //         }
    //     }
    // }
}
