use crate::block::Block;
use std::collections::HashMap;

/// Indexes of chain attribute
#[derive(Debug, Clone)]
pub struct BlockchainIndex {
    /// Vector of blocks
    blocks: Vec<Block>,
}

impl BlockchainIndex {
    /// Creates new `BlockahinIndex`
    pub fn new() -> Self {
        BlockchainIndex { blocks: vec![] }
    }
}

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// Index of `Block`s
    index: BlockchainIndex,
    /// Hash map of all `Block`s received, both on and off the longest chain
    block_hash_longest_chain_hashmap: HashMap<[u8; 32], u8>,
    /// Hash map of the ids of `Block` hashes
    block_hash_block_id_hashmap: HashMap<[u8; 32], u64>,

    /// Position of the longest chain
    longest_chain_position: usize,
    /// Flag indicating if the lognest chain is set
    longest_chain_is_set: bool,

    /// The start of the blockchain UNIx timestamp
    genesis_timestamp: u64,
    /// The latest `Block` id in the genesis period
    genesis_block_id: u64,
    /// The amount of `Block`s kept on chain
    genesis_period: u64,

    /// Last `Block` hash
    last_block_hash: [u8; 32],
    /// Last `Block` id
    last_block_id: u64,
    /// Last `Block` timestamp
    last_timestamp: u64,
    /// Last `Block` burnfee
    last_burnfee: f64,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            index: BlockchainIndex::new(),
            block_hash_longest_chain_hashmap: HashMap::new(),
            block_hash_block_id_hashmap: HashMap::new(),

            longest_chain_position: 0,
            longest_chain_is_set: false,

            genesis_timestamp: 0,
            genesis_block_id: 0,
            genesis_period: 0,

            last_block_hash: [0; 32],
            last_block_id: 0,
            last_timestamp: 0,
            last_burnfee: 0.0,
        }
    }

    /// Returns the latest `Block` as part of the longest chain
    pub fn get_latest_block(&self) -> Option<&Block> {
        match !self.longest_chain_is_set {
            true => None,
            false => Some(&self.index.blocks[self.longest_chain_position]),
        }
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {
        // ignore blocks that are pre-genesis
        if block.timestamp() < self.genesis_timestamp || block.id() < self.genesis_block_id {
            println!("not adding block to blockchain -- block precedes genesis");
            return;
        }

        let pos: usize = self.index.blocks.len();
        self.block_hash_block_id_hashmap
            .insert(block.block_hash(), block.id());
        self.block_hash_longest_chain_hashmap
            .insert(block.block_hash(), 1);

        self.index.blocks.push(block);

        self.last_block_hash = self.index.blocks[pos].block_hash();
        self.last_timestamp = self.index.blocks[pos].timestamp();
        self.last_block_id = self.index.blocks[pos].id();

        self.longest_chain_position = pos;
        self.longest_chain_is_set = true;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;
    use std::collections::HashMap;

    #[test]
    fn blockchain_test() {
        let blockchain = Blockchain::new();

        assert_eq!(blockchain.index.blocks, vec![]);
        assert_eq!(blockchain.block_hash_longest_chain_hashmap, HashMap::new());
        assert_eq!(blockchain.block_hash_block_id_hashmap, HashMap::new());
        assert_eq!(blockchain.longest_chain_position, 0);
        assert_eq!(blockchain.longest_chain_is_set, false);

        assert_eq!(blockchain.genesis_timestamp, 0);
        assert_eq!(blockchain.genesis_block_id, 0);
        assert_eq!(blockchain.genesis_period, 0);

        assert_eq!(blockchain.last_block_hash, [0; 32]);
        assert_eq!(blockchain.last_block_id, 0);
        assert_eq!(blockchain.last_timestamp, 0);
        assert_eq!(blockchain.last_burnfee, 0.0);
    }
    #[test]
    fn blockchain_get_latest_block_none_test() {
        let blockchain = Blockchain::new();
        match blockchain.get_latest_block() {
            None => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn blockchain_get_latest_block_some_test() {
        let mut blockchain = Blockchain::new();
        let block = Block::new(Keypair::new().public_key().clone(), [0; 32]);

        blockchain.add_block(block.clone());

        match blockchain.get_latest_block() {
            Some(prev_block) => {
                assert_eq!(prev_block.clone(), block);
                assert!(true);
            }
            None => assert!(false),
        }
    }
    #[test]
    fn blockchain_add_block_test() {
        let mut blockchain = Blockchain::new();
        let block = Block::new(Keypair::new().public_key().clone(), [0; 32]);

        blockchain.add_block(block.clone());

        assert_eq!(blockchain.index.blocks[0], block.clone());

        assert_eq!(blockchain.last_block_hash, block.block_hash());
        assert_eq!(blockchain.last_timestamp, block.timestamp());
        assert_eq!(blockchain.last_block_id, block.id());

        assert!(blockchain
            .block_hash_block_id_hashmap
            .contains_key(&block.block_hash()));
        assert!(blockchain
            .block_hash_longest_chain_hashmap
            .contains_key(&block.block_hash()));

        assert_eq!(blockchain.longest_chain_position, 0);
        assert_eq!(blockchain.longest_chain_is_set, true);
    }
}
