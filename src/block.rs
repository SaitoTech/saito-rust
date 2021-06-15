use crate::{
  transaction::Transaction,
  crypto::{hash, PublicKey},
};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {

    //
    // Consensus Variables
    //
    id: u64,
    timestamp: u64,
    previous_block_hash: [u8; 32],
    merkle_root: [u8; 32],
    creator: [u8; 32],
    treasury: u64,
    burnfee: u64,
    difficulty: u64,

    //
    //
    //
    transactions: Vec<Transaction>,

    //
    // Emergent Consensus Variables
    //
    hash: [u8; 32],
    lc: bool,


}

impl Block {

    pub fn new() -> Block {
        Block {

	    id: 0,
	    timestamp: 0,
	    previous_block_hash: [0;32],
	    merkle_root: [0;32],
	    creator: [0;32],
	    treasury: 0,
	    burnfee: 0,
	    difficulty: 0,

	    transactions: vec![],

            hash: [0;32],
	    lc: false,

        }
    }



    /// Converts our blockhash from a byte array into a hex string
    pub fn hash_as_hex(&self) -> String {
        hex::encode(self.hash)
    }


    pub fn get_hash(&self) -> [u8; 32] {
	return self.hash;
    }

    pub fn get_previous_block_hash(&self) -> [u8; 32] {
	return self.previous_block_hash;
    }

    pub fn get_id(&self) -> u64 {
	return self.id;
    }

    pub fn get_lc(&self) -> bool {
	return self.lc;
    }


    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }


    pub fn set_id(&mut self, id: u64) {
      self.id = id;
    }

    pub fn set_lc(&mut self, lc: bool) {
      self.lc = lc;
    }

    pub fn set_previous_block_hash(&mut self, previous_block_hash: [u8;32]) {
      self.previous_block_hash = previous_block_hash;
    }

    pub fn set_timestamp(&mut self, ts: u64) {
      self.timestamp = ts;
    }

    /// generates the block hash gives its current datae
    pub fn set_hash(&mut self) -> [u8; 32] {

      if self.hash == [0;32] {

        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = self.id.to_be_bytes();
        let ts_bytes: [u8; 8] = self.timestamp.to_be_bytes();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);

        self.hash = hash(&data);

      }

      self.hash

    }


    pub fn validate(&self) -> bool {
	return true;
    }


}


