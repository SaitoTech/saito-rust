use crate::{
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;


#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    id: u64,
    timestamp: u64,
    signature: Signature,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    message: Vec<u8>,
    path: Path,
    transaction_type: TransactionType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


impl Transaction {
    pub fn new() -> Transaction {
        Transaction {
            id: 0,
	    timestamp: create_timestamp(),
            signature: Signature::from_compact(&[0; 64]).unwrap(),
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
            path: Path::new(),
	    transaction_type: TransactionType::Normal,
        }
    }

}

