use crate::block::{Block, BlockHeader};

pub type BlockIndex = (BlockHeader, [u8; 32]);
/// Indexes of chain attribute
#[derive(Debug, Clone)]
pub struct BlockchainIndex {
    /// Vector of blocks
    blocks: Vec<BlockIndex>,
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
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            index: BlockchainIndex::new(),
        }
    }

    /// Returns the latest `Block` as part of the longest chain
    pub fn get_latest_block(&self) -> Option<&BlockIndex> {
        self.index.blocks.last()
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {
        let block_index: BlockIndex = (block.header().clone(), block.hash());
        self.index.blocks.push(block_index);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;

    #[test]
    fn blockchain_test() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.index.blocks, vec![]);
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
            Some((prev_block_header, _)) => {
                assert_eq!(&prev_block_header.clone(), block.header());
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
        let (block_header, _) = blockchain.index.blocks[0].clone();

        assert_eq!(block_header, *block.clone().header());
    }
}
