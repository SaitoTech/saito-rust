use crate::block::Block;
use crate::blockring::BlockRing;
use crate::crypto::SaitoHash;

use ahash::AHashMap;

#[derive(Debug)]
pub struct Blockchain {
    blockring: BlockRing,
    blocks: AHashMap<SaitoHash, Block>,
    last_block_hash: Option<SaitoHash>,
}



impl Blockchain {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Blockchain {
    	    blockring: BlockRing::new(),
            blocks: AHashMap::new(),
            last_block_hash: None,
        }
    }

    pub fn add_block(&mut self, block: Block) {
        println!(
            "Received block in blockchain.add_block: {:?}",
            block.get_hash()
        );

        println!("Validates? {:?}", block.validate());
    }

    pub fn get_latest_block(&self) -> Option<&Block> {
        match self.last_block_hash {
            Some(hash) => self.blocks.get(&hash),
            None => None,
        }
    }

    pub fn get_latest_block_hash(&self) -> SaitoHash {
        match self.last_block_hash {
            Some(hash) => self.blocks.get(&hash).unwrap().get_hash(),
            None => [0; 32],
        }
    }

    pub fn get_latest_block_id(&self) -> u64 {
        match self.last_block_hash {
            Some(hash) => self.blocks.get(&hash).unwrap().get_id(),
            None => 1,
        }
    }
}
