use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionCore};
use std::str::FromStr;

#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    hash: [u8; 32],
    pub core: BlockCore,
    transactions: Vec<Transaction>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct BlockCore {
    id: u64,
    timestamp: u64,
    previous_block_hash: [u8; 32],
    merkle_root: [u8; 32],
    creator: PublicKey,
    burnfee: u64,
    difficulty: f32,
    treasury: u64,
    transaction_cores: Vec<TransactionCore>,
}

impl Block {

    pub fn new() -> Block {
        Block {
            hash: [0;32],
            core: BlockCore::new(),
            transactions: vec![],
        }
    }

    /// Returns the `Block` hash
    pub fn hash(&mut self) -> [u8; 32] {

      if self.hash == [0;32] {

        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = self.core.id.to_be_bytes();
        let ts_bytes: [u8; 8] = self.core.timestamp.to_be_bytes();
        let cr_bytes: Vec<u8> = self.core.creator.serialize().iter().cloned().collect();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);
        data.extend(&cr_bytes);

        self.hash = hash(&data);

      }

      self.hash

    }

    /// Returns the previous `Block` hash
    pub fn block_id(&self) -> u64 {
        self.core.id
    }

    /// Returns the previous `Block` hash
    pub fn burnfee(&self) -> u64 {
        self.core.burnfee
    }

    /// Returns the previous `Block` hash
    pub fn creator(&self) -> &PublicKey {
        &self.core.creator
    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> [u8;32] {
        self.core.previous_block_hash
    }


    pub fn set_previous_block_hash(&mut self, previous_block: &Block) {
	self.core.previous_block_hash = previous_block.hash();
    }




}

impl BlockCore {
    pub fn new() -> BlockCore {
        BlockCore {
            id: 0,
            timestamp: create_timestamp(),
            previous_block_hash: [0;32],
            merkle_root: [0;32],
            creator: PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap(),
            burnfee: 0,
            difficulty: 0.0,
            treasury: 0,
            transaction_cores: vec![],
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn block_test() {
        let block = Block::new_mock();
        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(*block.transactions(), vec![]);
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.coinbase(), 0);
    }

}


