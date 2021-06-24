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

    pub fn add_block() {

    }

}


