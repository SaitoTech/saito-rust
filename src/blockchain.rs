use crate::block::{Block, BlockHeader};
use crate::crypto::SECP256K1Hash;
use crate::shashmap::Shashmap;

pub type BlockIndex = (BlockHeader, SECP256K1Hash);
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
    /// Hashmap of slips used by the network
    shashmap: Shashmap,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            index: BlockchainIndex::new(),
            shashmap: Shashmap::new(),
        }
    }

    /// Returns the latest `Block` as part of the longest chain
    pub fn get_latest_block_index(&self) -> Option<&BlockIndex> {
        self.index.blocks.last()
    }

    /// Append `Block` to the index of `Blockchain`
    ///
    /// * `block` - `Block` appended to index
    pub fn add_block(&mut self, block: Block) {
        for tx in block.transactions().iter() {
            self.shashmap.insert_new_transaction(tx);
        }

        let block_index: BlockIndex = (block.header().clone(), block.hash());
        println!("{:?}", block_index.clone());
        self.index.blocks.push(block_index);
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
        assert_eq!(blockchain.index.blocks, vec![]);
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

        blockchain.add_block(block.clone());

        match blockchain.get_latest_block_index() {
            Some((prev_block_header, _)) => {
                assert_eq!(&prev_block_header.clone(), block.header());
                assert!(true);
            }
            None => assert!(false),
        }
    }
    #[test]
    fn blockchain_add_block_test() {
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

        blockchain.add_block(block.clone());
        let (block_header, _) = blockchain.index.blocks[0].clone();

        assert_eq!(block_header, *block.clone().header());
        assert_eq!(blockchain.shashmap.slip_block_id(&slip), Some(&-1));
    }
}
