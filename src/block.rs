
#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    hash: [u8; 32],
    id: u64,
}

impl Block {

    pub fn new() -> Block {
        Block {
	    id: 0,
            hash: [0;32],
        }
    }

    pub fn validate(&self) -> bool {
	return true;
    }


}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn block_test() {
        let block = Block::new_mock();
        assert_eq!(block.id(), 0);
    }

}


