use crate::block::Block;
use crate::crypto::Sha256Hash;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ForkTree {
    fork_tree: HashMap<Sha256Hash, Block>,
}

impl ForkTree {
    /// Create new `ForkTree`
    pub fn new() -> Self {
        ForkTree {
            fork_tree: HashMap::new(),
        }
    }
    pub fn insert(&mut self, block_hash: Sha256Hash, block: Block) {
        self.fork_tree.insert(block_hash, block);
    }
    pub fn block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Block> {
        self.fork_tree.get(block_hash)
    }
    pub fn parent_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Block> {
        self.fork_tree.get(block_hash)
    }
    pub fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_key(block_hash)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_fork_tree() {
        assert!(true);
    }
}
