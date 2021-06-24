use crate::block::Block;
use crate::crypto::{SaitoHash};
use ahash::AHashMap;





#[derive(Debug)]
pub struct Blockchain {
    blocks: AHashMap<SaitoHash, Block>,
}

impl Blockchain {

    pub fn new() -> Self {
        Blockchain {
	    blocks: AHashMap::new(),
        }
    }

    pub fn add_block(&mut self, block : Block) {

	println!("Received block in blockchain.add_block: {:?}", block.get_hash());


    }

    pub fn get_latest_block_id(&self) -> u64 {
        1
    }

    pub fn get_latest_block_hash(&self) -> SaitoHash {
        [0; 32]
    }

}


