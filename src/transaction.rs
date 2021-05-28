use crate::{slip::Slip, time::create_timestamp};
use secp256k1::{PublicKey, Signature};

/// A single record used in the history of transactions being routed around the network
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Hop {
    /// An `secp256k1::PublicKey` of the router
    pub address: PublicKey,
    /// An `secp256k1::Signature` proving routing work
    pub signature: Signature,
}

impl Hop {
    /// Creates a new `Hop`
    ///
    /// * `address` - `secp256k1::PublicKey` address of router
    /// * `signature` - `secp256k1::Signature` verifying work done by routers
    pub fn new(address: PublicKey, signature: Signature) -> Hop {
        return Hop { address, signature };
    }
}

/// A record containging data of funds between transfered between public addresses. It
/// contains additional information as an optinal message field to transfer data around the network
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    /// `secp256k1::Signature` verifying authenticity of `Transaction` data
    signature: Signature,
    /// All data which is serialized and signed
    body: TransactionBody,
}

/// Core data to be serialized/deserialized of `Transaction`
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionBody {
    /// UNIX timestamp when the `Transaction` was created
    timestamp: u64,
    /// A list of `Slip` inputs
    inputs: Vec<Slip>,
    /// A list of `Slip` outputs
    outputs: Vec<Slip>,
    /// A enum brodcast type determining how to process `Transaction` in consensus
    broadcast_type: TransactionType,
    /// A list of `Hop` stipulating the history of `Transaction` routing
    path: Vec<Hop>,
    /// A byte array of miscellaneous information
    message: Vec<u8>,
}

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}

impl Transaction {
    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionType` of the new `Transaction`
    pub fn new(broadcast_type: TransactionType) -> Transaction {
        return Transaction {
            signature: Signature::from_compact(&[0; 64]).unwrap(),
            body: TransactionBody {
                timestamp: create_timestamp(),
                inputs: vec![],
                outputs: vec![],
                broadcast_type,
                path: vec![],
                message: vec![],
            },
        };
    }

    /// Returns a timestamp when `Transaction` was created
    pub fn timestamp(&self) -> u64 {
        self.body.timestamp
    }

    /// Returns list of `Slip` outputs
    pub fn outputs(&self) -> &Vec<Slip> {
        &self.body.outputs
    }

    /// Returns list of mutable `Slip` outputs
    pub fn outputs_mut(&mut self) -> &mut Vec<Slip> {
        &mut self.body.outputs
    }

    /// Returns list of `Slip` inputs
    pub fn inputs(&self) -> &Vec<Slip> {
        &self.body.inputs
    }

    /// Returns list of `Slip` inputs
    pub fn inputs_mut(&mut self) -> &mut Vec<Slip> {
        &mut self.body.inputs
    }

    /// Returns `secp256k1::Signature` verifying the validity of data on a transaction
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Returns `TransactionType` of the `Transaction`
    pub fn broadcast_type(&self) -> &TransactionType {
        &self.body.broadcast_type
    }

    /// Returns the list of `Hop`s serving as a routing history of the `Transaction`
    pub fn path(&self) -> &Vec<Hop> {
        &self.body.path
    }

    /// Returns the message of the `Transaction`
    pub fn message(&self) -> &Vec<u8> {
        &self.body.message
    }

    /// Set the list of `Slip` outputs
    pub fn set_outputs(&mut self, slips: Vec<Slip>) {
        self.body.outputs = slips;
    }

    /// Add a new `Slip` to the list of `Slip` outputs
    pub fn add_output(&mut self, slip: Slip) {
        self.body.outputs.push(slip);
    }

    /// Set the list of `Slip` inputs
    pub fn set_inputs(&mut self, slips: Vec<Slip>) {
        self.body.inputs = slips;
    }

    /// Add a new `Slip` to the list of `Slip` inputs
    pub fn add_input(&mut self, slip: Slip) {
        self.body.inputs.push(slip);
    }

    /// Set the `secp256k1::Signature`
    pub fn set_signature(&mut self, sig: Signature) {
        self.signature = sig;
    }

    /// Set the list of `Hop`s
    pub fn set_path(&mut self, path: Vec<Hop>) {
        self.body.path = path;
    }

    /// Add a new `Hop` to the list of `Hop`s
    pub fn add_hop_to_path(&mut self, path: Hop) {
        self.body.path.push(path);
    }

    /// Set the message
    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.body.message = msg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{Slip, SlipBroadcastType},
    };

    #[test]
    fn transaction_test() {
        let mut tx = Transaction::new(TransactionType::Normal);

        assert_eq!(tx.outputs(), &vec![]);
        assert_eq!(tx.inputs(), &vec![]);
        assert_eq!(tx.signature(), &Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.broadcast_type(), &TransactionType::Normal);
        assert_eq!(tx.path(), &vec![]);
        assert_eq!(tx.message(), &vec![]);

        let keypair = Keypair::new();
        let to_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        let from_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        let hop_message_bytes = Keypair::make_message_from_string("message_string");
        let signature = keypair.sign_message(&hop_message_bytes);
        let hop = Hop::new(keypair.public_key().clone(), signature);

        tx.add_output(to_slip);
        tx.add_input(from_slip);
        tx.add_hop_to_path(hop);

        assert_eq!(tx.outputs(), &vec![to_slip]);
        assert_eq!(tx.inputs(), &vec![from_slip]);
        assert_eq!(tx.path(), &vec![hop]);

        tx.set_signature(signature.clone());
        assert_eq!(tx.signature(), &signature);

        let message_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        tx.set_message(message_bytes.clone());
        assert_eq!(tx.message(), &message_bytes);
    }
}
