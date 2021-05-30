use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionCore};
use std::str::FromStr;

#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    /// Memoized hash of the block
    hash: Option<[u8; 32]>,
    /// BlockCore contains most of the critical block data maintained by consensus
    pub core: BlockCore,
    /// full transaction set
    transactions: Vec<Transaction>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct BlockCore {
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: [u8; 32],
    /// merkle root of transactions in Block
    merkle_root: [u8; 32],
    /// `Publickey` of the block creator
    creator: PublicKey,
    /// `BurnFee` containing the fees paid to produce the block
    burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// Total amount in network treasury
    treasury: u64,
    /// simplified transaction cores
    transaction_cores: Vec<TransactionCore>,
}


impl Block {
    pub fn new() -> Block {
        Block {
            hash: None,
            core: BlockCore::new(),
            transactions: vec![],
        }
    }


    /// Returns the `Block` hash
    pub fn hash(&mut self) -> Option<[u8; 32]> {

      if self.hash.is_none() {

        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = self.core.id.to_be_bytes();
        let ts_bytes: [u8; 8] = self.core.timestamp.to_be_bytes();
        let cr_bytes: Vec<u8> = self.core.creator.serialize().iter().cloned().collect();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);
        data.extend(&cr_bytes);

        self.hash = Some(hash(&data));

      }

      self.hash

    }


    /// Returns the previous `Block` hash
    pub fn block_id(&self) -> u64 {
        self.core.id
    }

    /// Returns the previous `Block` hash
    pub fn creator(&self) -> &PublicKey {
        &self.core.creator
    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> &[u8; 32] {
        &self.core.previous_block_hash
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
        
        let publickey: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(block.creator(), publickey);
        assert_eq!(*block.transactions(), vec![]);
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.coinbase(), 0);
    }

}


