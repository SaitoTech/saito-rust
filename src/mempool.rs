use crate::{
    block::Block,
    blockchain::BlockIndex,
    burnfee::BurnFee,
    keypair::Keypair,
    time::{create_timestamp, format_timestamp},
    transaction::Transaction,
    types::SaitoMessage,
};
use std::sync::{Arc, RwLock};

pub const GENESIS_PERIOD: u64 = 21500;

/// The `Mempool` is the structure that collects blocks and transactions
/// and is control of discerning whether the node is allowed to create a block.
/// It bundles the block, the sends it to `Blockchain` to be added to the longest chain.
/// New `Block`s coming in over the network will hit the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    /// `Keypair` used to sign and bundle blocks
    _keypair: Arc<RwLock<Keypair>>,
    /// A list of blocks to be added to the longest chain
    _blocks: Vec<Block>,
    /// A list of `Transaction`s to be bundled into `Block`s
    transactions: Vec<Transaction>,
    ///
    _burnfee: BurnFee,
    /// The current work available in fees found in the list of `Transaction`s
    work_available: u64,
}

impl Mempool {
    /// Creates new `Memppol`
    pub fn new(keypair: Arc<RwLock<Keypair>>) -> Self {
        Mempool {
            _keypair: keypair,
            _burnfee: BurnFee::new(10.0),
            _blocks: vec![],
            transactions: vec![],
            work_available: 0,
        }
    }

    /// Processes `SaitoMessage` and attempts to return `Block`
    ///
    /// * `message` - `SaitoMessage` enum commanding `Mempool` operation
    /// * `previous_block_index` - `BlockIndex` of previous block in `Blockchain` longest chain
    pub fn process(
        &mut self,
        message: SaitoMessage,
        previous_block_index: Option<&BlockIndex>,
    ) -> Option<Block> {
        match message {
            SaitoMessage::Transaction { payload } => {
                self.transactions.push(payload);
                self.try_bundle(previous_block_index)
            }
            SaitoMessage::TryBundle => self.try_bundle(previous_block_index),
            _ => None,
        }
    }

    /// Attempt to create a new `Block`
    ///
    /// * `previous_block` - `Option` of previous `Block`
    fn try_bundle(&mut self, previous_block_index: Option<&BlockIndex>) -> Option<Block> {
        if self.can_bundle_block(previous_block_index) {
            Some(self.bundle_block(previous_block_index))
        } else {
            None
        }
    }

    /// Check to see if the `Mempool` has enough work to bundle a block
    ///
    /// * `previous_block` - `Option` of previous `Block`
    fn can_bundle_block(&self, previous_block_index: Option<&BlockIndex>) -> bool {
        match previous_block_index {
            Some((block_header, _)) => {
                let current_timestamp = create_timestamp();
                let work_needed = self
                    ._burnfee
                    .return_work_needed(current_timestamp, block_header.timestamp());

                println!(
                    "TS: {} -- WORK ---- {:?} -- {:?} --- TX COUNT {:?}",
                    format_timestamp(current_timestamp),
                    work_needed,
                    self.work_available,
                    self.transactions.len(),
                );

                // TODO -- add check for transactions in Mempool
                self.work_available >= work_needed
            }
            None => true,
        }
    }

    /// Create a new `Block` from the `Mempool`'s list of `Transaction`s
    ///
    /// * `previous_block` - `Option` of the previous block on the longest chain
    fn bundle_block(&mut self, previous_block_index: Option<&BlockIndex>) -> Block {
        let keypair = self._keypair.read().unwrap();
        let publickey = keypair.public_key();
        let mut block: Block;

        match previous_block_index {
            Some((previous_block_header, previous_block_hash)) => {
                block = Block::new(publickey.clone(), previous_block_hash.clone());

                // TODO -- include reclaimed fees here
                let treasury = previous_block_header.treasury();
                let coinbase = (treasury as f64 / GENESIS_PERIOD as f64).round() as u64;

                block.set_id(previous_block_header.id() + 1);
                block.set_coinbase(coinbase);
                block.set_treasury(treasury - coinbase);
                block.set_previous_block_hash(previous_block_hash.clone());
                block.set_difficulty(previous_block_header.difficulty());

                self._burnfee
                    .adjust_work_needed(block.timestamp(), previous_block_header.timestamp());
                block.set_burnfee(
                    self._burnfee
                        .return_work_needed(block.timestamp(), previous_block_header.timestamp()),
                );
            }
            None => {
                block = Block::new(publickey.clone(), [0; 32]);
            }
        }

        block.set_transactions(&mut self.transactions);

        return block;
        // TODO -- calculate difficulty and paysplit changes
        // https://github.com/orgs/SaitoTech/projects/5#card-61347666
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::keypair::Keypair;
    use std::sync::{Arc, RwLock};

    #[test]
    fn mempool_test() {
        assert_eq!(true, true);
        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let mempool = Mempool::new(keypair);

        assert_eq!(mempool.work_available, 0);
    }
    #[test]
    fn mempool_try_bundle_none_test() {
        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let mut mempool = Mempool::new(keypair);

        let new_block = mempool.try_bundle(None);

        match new_block {
            Some(block) => {
                assert_eq!(block.id(), 0);
                assert_eq!(*block.previous_block_hash(), [0; 32]);
            }
            None => {}
        }
    }
    #[test]
    fn mempool_try_bundle_some_test() {
        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let mut mempool = Mempool::new(keypair);

        let prev_block = Block::new(Keypair::new().public_key().clone(), [0; 32]);
        let prev_block_index = &(prev_block.header().clone(), prev_block.hash());
        let new_block = mempool.try_bundle(Some(prev_block_index));

        match new_block {
            Some(_) => {}
            None => {
                assert_eq!(true, true)
            }
        }
    }
}
