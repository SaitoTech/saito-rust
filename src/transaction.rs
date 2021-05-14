use crate::{slip::Slip, time::create_timestamp};
use secp256k1::{PublicKey, Signature};

/// A single record used in the history of transactions being routed around the network
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Hop {
    pub to: PublicKey,
    sig: Signature,
}

impl Hop {
    /// Creates a new `Hop`
    ///
    /// * `to` - `secp256k1::PublicKey` address of where the transaction is headed
    /// * `sign` - `secp256k1::Signature` verifying work done by routers
    pub fn new(to: PublicKey, sig: Signature) -> Hop {
        return Hop { to, sig };
    }
}

/// A record containging data of funds between transfered between public addresses. It
/// contains additional information as an optinal message field to transfer data around the network
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    body: TransactionBody,
}

/// Core data to be serialized/deserialized of `Transaction`
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionBody {
    ts: u64,
    pub to: Vec<Slip>,
    pub from: Vec<Slip>,
    sig: Signature,
    pub broadcast_type: TransactionBroadcastType,
    path: Vec<Hop>,
    pub msg: Vec<u8>,
}

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionBroadcastType {
    Normal,
}

impl Transaction {
    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionBroadcastType` of the new `Transaction`
    pub fn new(broadcast_type: TransactionBroadcastType) -> Transaction {
        return Transaction {
            body: TransactionBody {
                ts: create_timestamp(),
                to: vec![],
                from: vec![],
                sig: Signature::from_compact(&[0; 64]).unwrap(),
                broadcast_type,
                path: vec![],
                msg: vec![],
            },
        };
    }

    /// Returns a timestamp when `Transaction` was created
    pub fn timestamp(&self) -> u64 {
        self.body.ts
    }

    /// Returns list of `Slip` outputs
    pub fn to_slips(&self) -> &Vec<Slip> {
        &self.body.to
    }

    /// Returns list of `Slip` outputs
    pub fn to_slips_mut(&mut self) -> &mut Vec<Slip> {
        &mut self.body.to
    }

    /// Returns list of `Slip` inputs
    pub fn from_slips(&self) -> &Vec<Slip> {
        &self.body.from
    }

    /// Returns list of `Slip` inputs
    pub fn from_slips_mut(&mut self) -> &mut Vec<Slip> {
        &mut self.body.from
    }

    /// Returns `secp256k1::Signature` verifying the validity of data on a transaction
    pub fn signature(&self) -> &Signature {
        &self.body.sig
    }

    /// Returns `TransactionBroadcastType` of the `Transaction`
    pub fn broadcast_type(&self) -> &TransactionBroadcastType {
        &self.body.broadcast_type
    }

    /// Returns the list of `Hop`s serving as a routing history of the `Transaction`
    pub fn path(&self) -> &Vec<Hop> {
        &self.body.path
    }

    /// Returns the message of the `Transaction`
    pub fn message(&self) -> &Vec<u8> {
        &self.body.msg
    }

    /// Set the list of `Slip` outputs
    pub fn set_to_slips(&mut self, slips: Vec<Slip>) {
        self.body.to = slips;
    }

    /// Add a new `Slip` to the list of `Slip` outputs
    pub fn add_to_slip(&mut self, slip: Slip) {
        self.body.to.push(slip);
    }

    /// Set the list of `Slip` inputs
    pub fn set_from_slips(&mut self, slips: Vec<Slip>) {
        self.body.from = slips;
    }

    /// Add a new `Slip` to the list of `Slip` inputs
    pub fn add_from_slip(&mut self, slip: Slip) {
        self.body.from.push(slip);
    }

    /// Set the `secp256k1::Signature`
    pub fn set_signature(&mut self, sig: Signature) {
        self.body.sig = sig;
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
        self.body.msg = msg;
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
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        assert_eq!(tx.to_slips(), &vec![]);
        assert_eq!(tx.from_slips(), &vec![]);
        assert_eq!(tx.signature(), &Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.broadcast_type(), &TransactionBroadcastType::Normal);
        assert_eq!(tx.path(), &vec![]);
        assert_eq!(tx.message(), &vec![]);

        let keypair = Keypair::new();
        let to_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        let from_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);

        let hop_message_bytes = Keypair::make_message_from_string("message_string");
        let signature = keypair.sign_message(&hop_message_bytes);
        let hop = Hop::new(keypair.public_key().clone(), signature);

        tx.add_to_slip(to_slip);
        tx.add_from_slip(from_slip);
        tx.add_hop_to_path(hop);

        assert_eq!(tx.to_slips(), &vec![to_slip]);
        assert_eq!(tx.from_slips(), &vec![from_slip]);
        assert_eq!(tx.path(), &vec![hop]);

        tx.set_signature(signature.clone());
        assert_eq!(tx.signature(), &signature);

        let message_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        tx.set_message(message_bytes.clone());
        assert_eq!(tx.message(), &message_bytes);
    }
}
