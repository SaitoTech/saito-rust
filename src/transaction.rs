use crate::{
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;


#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub core: TransactionCore,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionCore {
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
            core: TransactionCore::new(),
        }
    }
}

impl TransactionCore {

    pub fn new() -> TransactionCore {
        TransactionCore {
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::Slip,
    };

    #[test]
    fn transaction_test() {

        let mut tx: Transaction::new();

        assert_eq!(tx.core.outputs(), &vec![]);
        assert_eq!(tx.core.inputs(), &vec![]);
        assert_eq!(tx.core.transaction_type(), &TransactionType::Normal);
        assert_eq!(tx.core.message(), &vec![]);

    }

}
