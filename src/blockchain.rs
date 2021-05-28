use crate::block::{Block, BlockHeader};
use crate::utxoset::UTXOSet;
use std::collections::HashMap;

/// We want to keep information on the block header, the SHA256hash,
/// and whether or not the block is on the longest chain
pub type BlockIndex = (BlockHeader, [u8; 32]);
/// Indexes of chain attribute
#[derive(Debug, Clone)]
pub struct BlockchainIndex {
    /// Map of Blocks
    block_map: HashMap<[u8; 32], BlockIndex>,
    // HashMap of `Block`s that are on the longest chain
    longest_chain_map: HashMap<[u8; 32], bool>,
    /// The block has of the latest `Block` on the longest chain
    last_block_hash: Option<[u8; 32]>,
    /// The `Block` at the end of the epoch
    genesis_block_hash: Option<[u8; 32]>,
}

impl BlockchainIndex {
    /// Creates new `BlockahinIndex`
    pub fn new() -> Self {
        BlockchainIndex {
            block_map: HashMap::new(),
            longest_chain_map: HashMap::new(),
            last_block_hash: None,
            genesis_block_hash: None,
        }
    }
}

pub struct ChainFork {
    blocks: Vec<BlockIndex>,
    _cumulative_burnfee: u64,
}

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// A set of indices used to track the state of the `Blockchain`
    index: BlockchainIndex,
    /// Hashmap of slips used by the network
    utxoset: UTXOSet,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            index: BlockchainIndex::new(),
            utxoset: UTXOSet::new(),
        }
    }

    /// Returns the latest `Block` as part of the longest chain
    pub fn get_latest_block_index(&self) -> Option<&BlockIndex> {
        match self.index.last_block_hash {
            Some(last_block_hash) => self.index.block_map.get(&last_block_hash),
            None => None,
        }
    }

    pub fn get_genesis_block_index(&self) -> Option<&BlockIndex> {
        match self.index.genesis_block_hash {
            Some(genesis_block_hash) => self.index.block_map.get(&genesis_block_hash),
            None => None,
        }
    }

    /// Append `Block` to the index of `Blockchain`
    ///
    /// * `block` - `Block` appended to index
    pub fn try_add_block(&mut self, block: Block) -> crate::Result<()> {
        // TODO -- we want to lock writes to the blockchain while we're adding a block
        // To do that, we should implement a RwLock around the `Blockchain` index to verify
        // that there's only one thread that is writing to the chain at once time

        if let Some((genesis_block_header, _)) = self.get_genesis_block_index() {
            // check if our candidate blocks is greater than the genesis block
            if self.genesis_block_check(&genesis_block_header, &block) {
                // Add the block to the index, but not the longest chain just yet
                self.add_block_index(&block);

                // If our candidate block previous block_hash is equal to last_block_hash,
                // then the candidate block is building on the longest chain
                if &block.header().previous_block_hash() == &(self.index.last_block_hash.unwrap()) {
                    // Add block to longest chain and add successfully
                    self.index.longest_chain_map.insert((&block).hash(), true);
                    self.index.last_block_hash = Some((&block).hash());
                    self.spend_transactions(&block)?;
                    self.add_block(block)?;
                } else {
                    //
                    // The candidate block is forking;
                    // the node attempts to find the shared ancestor of the candidate block
                    // with the longest chain.
                    //
                    // The function also returns the new_chain and old_chain,
                    // which are both `ChainFork`s. `ChainFork` is just a container
                    // that collects `Blocks` for each chain, and calculates the cumulative burnfee
                    // for each chain. This makes comparing the chains very simple, as well as
                    // simplifying winding and unwinding the chain if the chain needs to be reorganized
                    if let Some((shared_ancestor_index, new_chain, old_chain)) =
                        self.fork_chains((&block).hash())
                    {
                        // `fork_chains` returns the shared ancestor, and both chains
                        // so that they can be compared and decided which one is longer
                        if self.is_longer_chain(&new_chain, &old_chain) {
                            //
                            // New chain is longer than old, so there's a chain reorganization
                            self.reorganize_chain(shared_ancestor_index, new_chain, old_chain)?;
                        } else {
                            // New chain is not longer than old, kick into add_block
                            // but we don't spend the transactions however
                            self.add_block(block)?;
                        }
                    } else {
                        //
                        // TODO -- handle edge case where we can't find a shared_ancestor.
                        //
                    }
                }
            }
        } else {
            // This is our first block, which is automatically added to chain
            self.add_block_index(&block);
            self.index.longest_chain_map.insert((&block).hash(), true);
            self.index.last_block_hash = Some((&block).hash());
            self.spend_transactions(&block)?;
            self.add_block(block)?;
        }

        Ok(())
    }

    fn add_block_index(&mut self, block: &Block) {
        let block_index: BlockIndex = ((&block).header().clone(), (&block).hash());
        self.index
            .block_map
            .insert((&block).hash(), block_index.clone());
    }

    fn genesis_block_check(&self, genesis_block_header: &BlockHeader, block: &Block) -> bool {
        block.id() > genesis_block_header.id()
            && block.timestamp() > genesis_block_header.timestamp()
    }

    fn fork_chains(&self, block_hash: [u8; 32]) -> Option<(BlockIndex, ChainFork, ChainFork)> {
        let mut new_chain = ChainFork {
            blocks: vec![],
            _cumulative_burnfee: 0,
        };
        let mut old_chain = ChainFork {
            blocks: vec![],
            _cumulative_burnfee: 0,
        };

        let new_block_index = self.index.block_map.get(&block_hash).unwrap();
        let latest_block_index = self
            .index
            .block_map
            .get(&(self.index.last_block_hash.unwrap()))
            .unwrap();

        new_chain.blocks.push(new_block_index.clone());
        old_chain.blocks.push(latest_block_index.clone());

        let (new_block_header, _) = new_block_index;
        let (old_block_header, _) = latest_block_index;

        if let Some(shared_ancestor_index) = self.find_shared_ancestor(
            new_block_header.previous_block_hash(),
            old_block_header.previous_block_hash(),
            &new_chain,
            &old_chain,
        ) {
            // Return our chains with shared_ancestor_index
            Some((shared_ancestor_index, new_chain, old_chain))
        } else {
            // We couldn't find a shared_ancestor, return None
            None
        }
    }

    fn find_shared_ancestor(
        &self,
        _new_hash: [u8; 32],
        _old_hash: [u8; 32],
        _new_chain: &ChainFork,
        _old_chain: &ChainFork,
    ) -> Option<BlockIndex> {
        None
    }

    fn is_longer_chain(&self, _new_chain: &ChainFork, _old_chain: &ChainFork) -> bool {
        // TODO -- implement chain check
        false
    }

    fn reorganize_chain(
        &self,
        _shared_ancestor_index: BlockIndex,
        _new_chain: ChainFork,
        _old_chain: ChainFork,
    ) -> crate::Result<()> {
        Ok(())
    }

    fn spend_transactions(&mut self, block: &Block) -> crate::Result<()> {
        for tx in block.transactions().iter() {
            self.utxoset.insert_new_transaction(tx);
        }

        Ok(())
    }

    fn add_block(&mut self, _block: Block) -> crate::Result<()> {
        // TODO -- add storage and write block to disk
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;
    use crate::slip::{Slip, SlipBroadcastType};
    use crate::transaction::{Transaction, TransactionBroadcastType};

    #[test]
    fn blockchain_test() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.index.block_map, HashMap::new());
    }
    #[test]
    fn blockchain_get_latest_block_index_none_test() {
        let blockchain = Blockchain::new();
        match blockchain.get_latest_block_index() {
            None => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn blockchain_get_latest_block_index_some_test() {
        let mut blockchain = Blockchain::new();
        let block = Block::new(Keypair::new().public_key().clone(), [0; 32]);

        blockchain.try_add_block(block.clone()).unwrap();

        match blockchain.get_latest_block_index() {
            Some((prev_block_header, _)) => {
                assert_eq!(&prev_block_header.clone(), block.header());
                assert!(true);
            }
            None => assert!(false),
        }
    }
    #[test]
    fn blockchain_try_add_block_test() {
        let keypair = Keypair::new();
        let mut blockchain = Blockchain::new();
        let mut block = Block::new(keypair.public_key().clone(), [0; 32]);
        let mut transaction = Transaction::new(TransactionBroadcastType::Normal);
        let slip = Slip::new(
            *keypair.public_key(),
            SlipBroadcastType::Normal,
            2_0000_0000,
        );
        transaction.add_output(slip);
        block.add_transaction(transaction);

        blockchain.try_add_block(block.clone()).unwrap();
        let (block_header, _) = blockchain
            .index
            .block_map
            .get(&(block.clone().hash()))
            .unwrap();

        assert_eq!(block_header, block.clone().header());
        assert_eq!(blockchain.utxoset.slip_block_id(&slip), Some(&-1));
    }
}
