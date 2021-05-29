use crate::block::{Block, BlockCore};
use crate::shashmap::Shashmap;


/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {

    /// Vector of blocks
    blocks: Vec<BlockIndex>,

    /// Hashmap of slips used by the network
    shashmap: Shashmap,

}

impl Blockchain {

    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            blocks: vec![],
            shashmap: Shashmap::new(),
        }
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block_index);
    }

    /// Return latest `Block` in blockchain
    pub fn get_latest_block(&mut self) -> Option<Block> {
        pub block = Block::new();
	block
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;
    use crate::slip::{Slip, SlipType};
    use crate::transaction::{Transaction, TransactionBroadcastType};

    #[test]
    fn blockchain_test() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.blocks, vec![]);
    }
}

