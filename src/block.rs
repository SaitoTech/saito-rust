use crate::transaction::Transaction;
use crate::crypto::{SaitoHash,SaitoPublicKey,SaitoSignature,hash};
use serde::{Deserialize, Serialize};

//
// BlockCore is a self-contained object containing only the minimum
// information needed about a block. It exists to simplify block
// serialization and deserialization until we have custom functions
// and to .
//
// This is a private variable. Access to variables within the BlockCore
// should be handled through getters and setters in the block which
// surrounds it.
//
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockCore {
    id: 			u64,
    timestamp: 			u64,
    previous_block_hash: 	SaitoHash,
    #[serde_as(as = "[_; 33]")]
    creator: 			SaitoPublicKey, // public key of block creator
    merkle_root: 		SaitoHash, // merkle root of txs
    #[serde_as(as = "[_; 64]")]
    signature:			SaitoSignature, // signature of block creator
    treasury: 			u64,
    burnfee:			u64,
    difficulty:			u64,
}
impl BlockCore {

    pub fn new() -> BlockCore {
        BlockCore {
	    id: 0,
	    timestamp: 0,
	    previous_block_hash: [0;32],
	    creator: [0;33],
	    merkle_root: [0;32],
	    signature: [0;64],
	    treasury: 0,
	    burnfee: 0,
	    difficulty: 0,
        }
    }
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {

    //
    // Consensus Level Variables
    //
    core:  BlockCore,

    //
    // Transactions
    //
    transactions: Vec<Transaction>,

    //
    // Self-Calculated / Validated
    //
    hash: SaitoHash,
    lc: bool,

}

impl Block {

    pub fn new() -> Block {
        Block {
	    core: BlockCore::new(),
	    transactions: vec![],
            hash: [0;32],
	    lc: false,
        }
    }

    pub fn get_hash(&self) -> SaitoHash {
	self.hash
    }

    pub fn get_id(&self) -> u64 {
        1
    }

    pub fn set_id(&mut self, id : u64) {
        self.core.id = id;
    }

    pub fn set_timestamp(&mut self, timestamp : u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_previous_block_hash(&mut self , previous_block_hash : SaitoHash) {
        self.core.previous_block_hash = previous_block_hash;
    }

    // TODO
    //
    // hash is nor being serialized from the right data - requires
    // merkle_root as an input into the hash, and that is not yet 
    // supported. this is a stub that uses the timestamp and the
    // id -- it exists so each block will still have a unique hash
    // for blockchain functions.
    //
    pub fn set_hash(&mut self) -> SaitoHash {

        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = self.core.id.to_be_bytes();
        let ts_bytes: [u8; 8] = self.core.timestamp.to_be_bytes();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);

        self.hash = hash(&data);
 
	self.hash     
 
    }


}


//
// TODO
//
// temporary data-serialization of blocks so that we can save
// to disk. These should only be called through the serialization
// functions within the block class, so that all access is 
// compartmentalized and we can move to custom serialization
//
impl From<Vec<u8>> for Block {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for Block {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}


