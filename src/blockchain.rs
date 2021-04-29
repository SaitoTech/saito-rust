use crate::block::Block;

fn new() {
    
}
fn getLatest() {
    
}
fn get_block_by_id(id: i32) -> Block {
    Block {}
}
fn get_block_by_hash(hash: &str) -> Block {
    Block {}
}
fn add_block(block: Block, parentId: i64) {
    
}
fn get_block_parent(block: Block) -> Block {
    Block {}
}
// Event: rollBackBlock(Block block)
// Event: rollForwardBlock(Block block)


#[cfg(test)]
mod test {
    #[test]
    fn test_new() {
        assert!(false);
    }
    #[test]
    fn test_get_block_by_id() {
        assert!(false);
    }
    #[test]
    fn test_get_block_by_hash() {
        assert!(false);
    }
    #[test]
    fn test_add_block() {
        assert!(false);
    }
    #[test]
    fn test_get_block_parent() {
        assert!(false);
    }
}
