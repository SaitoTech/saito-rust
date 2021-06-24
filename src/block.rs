use crate::transaction::Transaction;
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
    previous_block_hash: 	[u8;32], // sha256hash
    #[serde_as(as = "[_; 33]")]
    creator: 			[u8;33], // secp256k1 publickey
    merkle_root: 		[u8;32], // sha256hash
    #[serde_as(as = "[_; 64]")]
    signature:			[u8;64], // signature of block creator
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
    hash: [u8; 32],
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

    pub fn get_hash(&self) -> [u8;32] {
	self.hash
    }
}


