use crate::block::Block;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ForkTree {
    fork_tree: HashMap<[u8; 32], Block>,
}

impl ForkTree {
    /// Create new `ForkTree`
    pub fn new() -> Self {
        ForkTree {
            fork_tree: HashMap::new(),
        }
    }
    pub fn insert(&self, _block_hash: [u8; 32], _block: Block) {
        
    }
    pub fn get_parent(block_hash: [u8; 32]) -> [u8; 32] {
        [0; 32]
    }
    pub fn contains_block_hash(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_fork_tree() {
        assert!(true);
    }
}
