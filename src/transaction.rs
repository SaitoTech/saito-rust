use crate::{
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;

/// A record containging data of funds between transfered between public addresses. It
/// contains additional information as an optinal message field to transfer data around the network
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    /// `secp256k1::Signature` verifying authenticity of `TransactionCore` data
    signature: Signature,
    /// All data which is serialized and signed
    pub core: TransactionCore,
    /// Routing Path for the Transaction
    path: Path,
}

impl Transaction {
    /// Creates new `Transaction`
    ///
    pub fn new() -> Transaction {
        Transaction {
            signature: Signature::from_compact(&[0; 64]).unwrap(),
            core: TransactionCore::new(),
            path: Path::new(),
        }
    }

}


/// Core data to be serialized/deserialized of `Transaction`
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionCore {
    /// id of transaction
    id: u64,
    /// UNIX timestamp when the `Transaction` was created
    timestamp: u64,
    /// a vector of UTXO input `Slip`
    inputs: Vec<Slip>,
    /// A vector of UTXO output `Slip`
    outputs: Vec<Slip>,
    /// A enum transaction type determining how to process `Transaction` in consensus
    transaction_type: TransactionType,
    /// A byte array of miscellaneous information
    message: Vec<u8>,
}

impl TransactionCore {

    /// Creates new `TransactionCore`
    ///
    pub fn new() -> TransactionCore {
        TransactionCore {
            id: 0,
	    timestamp: create_timestamp(),
	    inputs: vec![],
	    outputs: vec![],
	    transaction_type: TransactionType::Normal,
	    message: vec![],
        }
    }

}

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
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
        let mut tx: Transaction = Transaction::new_mock();

        assert_eq!(tx.core.outputs(), &vec![]);
        assert_eq!(tx.core.inputs(), &vec![]);
        //assert_eq!(tx.signature(), &Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.core.transaction_type(), &TransactionType::Normal);
        //assert_eq!(tx.path(), &vec![]);
        assert_eq!(tx.core.message(), &vec![]);

        let keypair = Keypair::new();
        //let to_slip = Slip::new(keypair.publickey().clone(), SlipBroadcastType::Normal, 0);
        //let from_slip = SlipID::new(10, 10, 10);

        // let hop_message_bytes = Keypair::make_message_from_string("message_string");
        // let signature = keypair.sign_message(&hop_message_bytes);
        // let hop = Hop::new(keypair.publickey().clone(), signature);

        //tx.core.add_output(to_slip);
        //tx.core.add_input(from_slip);

        //assert_eq!(tx.core.outputs(), &vec![to_slip]);
        //assert_eq!(tx.core.inputs(), &vec![from_slip]);
        // assert_eq!(tx.path(), &vec![hop]);

        // let message_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        // assert_eq!(tx.message(), &message_bytes);
    }
}
