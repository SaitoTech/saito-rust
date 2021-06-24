use crate::{
    time::create_timestamp,
    slip::Slip,
};
use serde::{Deserialize, Serialize};


//
// TransactionType is a human-readable indicator of the type of 
// transaction such as a normal user-initiated transaction, a 
// golden ticket transaction, a VIP-transaction or a rebroadcast
// transaction created by a block creator, etc.
//
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


//
// TransactionCore is a self-contained object containing only the core
// information about the transaction that exists regardless of how it 
// was routed or produced. It exists to simplify transaction serialization
// and deserialization until we have custom functions.
//
// This is a private variable. Access to variables within the 
// TransactionCore should be handled through getters and setters in the 
// block which surrounds it.
//
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionCore {
    id: u64,
    timestamp: u64,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
}
impl TransactionCore {
    pub fn new() -> TransactionCore {
        TransactionCore {
            id: 0,
	    timestamp: create_timestamp(),
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
	    transaction_type: TransactionType::Normal,
        }
    }
}


#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    core: TransactionCore,
    #[serde_as(as = "[_; 64]")]
    signature: [u8;64],	// compact signatures are 64 bytes; DER signatures are 68-72 bytes
}

impl Transaction {
    pub fn new() -> Transaction {
        Transaction {
	    core: TransactionCore::new(),
            signature: [0;64],
        }
    }
}



