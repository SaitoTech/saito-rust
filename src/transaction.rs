use crate::{
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    id: u64,
    timestamp: u64,
    // compact signatures are 64 bytes; DER signatures are 68-72 bytes
    signature: Vec<u8>,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    message: Vec<u8>,
    //path: Path,
    transaction_type: TransactionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


impl Transaction {

    pub fn new() -> Transaction {
        Transaction {
            id: 0,
	    timestamp: create_timestamp(),
            signature: vec![],
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
            //path: Path::new(),
	    transaction_type: TransactionType::Normal,
        }
    }


    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.message = msg;
    }

}


