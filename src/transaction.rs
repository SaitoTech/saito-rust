use crate::{slip::SaitoSlip, slip::SlipReference, time::create_timestamp};
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
pub struct RuntimeTransaction {
    block_id: u64,
    id: u64,
    pub signed_tx: SignedTransaction,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SignedTransaction {
    /// `secp256k1::Signature` verifying authenticity of `Transaction` data
    signature: Signature,
    pub body: TransactionBody,
}
/// Core data to be serialized/deserialized of `Transaction`
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionBody {
    /// UNIX timestamp when the `Transaction` was created
    timestamp: u64,
    /// A list of `SlipReference`s
    inputs: Vec<SlipReference>,
    /// A list of `SaitoSlip`s
    outputs: Vec<SaitoSlip>,
    /// A enum brodcast type determining how to process `Transaction` in consensus
    broadcast_type: TransactionBroadcastType,
    /// A list of `Hop` stipulating the history of `Transaction` routing
    path: Vec<Hop>,
    /// A byte array of miscellaneous information
    message: Vec<u8>,
}

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionBroadcastType {
    Normal,
}
impl TransactionBody {
    /// Creates new `TransactionBody`
    ///
    /// * `broadcast_type` - `TransactionBroadcastType` of the new `Transaction`
    pub fn new(broadcast_type: TransactionBroadcastType) -> TransactionBody {
        TransactionBody {
            timestamp: create_timestamp(),
            inputs: vec![],
            outputs: vec![],
            broadcast_type,
            path: vec![],
            message: vec![],
        }
    }

    /// Returns a timestamp when `Transaction` was created
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns list of `SaitoSlip` outputs
    pub fn outputs(&self) -> &Vec<SaitoSlip> {
        &self.outputs
    }

    /// Returns list of mutable `SaitoSlip` outputs
    pub fn outputs_mut(&mut self) -> &mut Vec<SaitoSlip> {
        &mut self.outputs
    }

    /// Returns list of `SaitoSlip` inputs
    pub fn inputs(&self) -> &Vec<SlipReference> {
        &self.inputs
    }

    /// Returns list of `SaitoSlip` inputs
    pub fn inputs_mut(&mut self) -> &mut Vec<SlipReference> {
        &mut self.inputs
    }

    /// Returns `TransactionBroadcastType` of the `Transaction`
    pub fn broadcast_type(&self) -> &TransactionBroadcastType {
        &self.broadcast_type
    }

    /// Returns the list of `Hop`s serving as a routing history of the `Transaction`
    pub fn path(&self) -> &Vec<Hop> {
        &self.path
    }

    /// Returns the message of the `Transaction`
    pub fn message(&self) -> &Vec<u8> {
        &self.message
    }

    /// Set the list of `SaitoSlip` outputs
    pub fn set_outputs(&mut self, slips: Vec<SaitoSlip>) {
        self.outputs = slips;
    }

    /// Add a new `SaitoSlip` to the list of `SaitoSlip` outputs
    pub fn add_output(&mut self, slip: SaitoSlip) {
        self.outputs.push(slip);
    }

    /// Set the list of `SaitoSlip` inputs
    pub fn set_inputs(&mut self, slips: Vec<SlipReference>) {
        self.inputs = slips;
    }

    /// Add a new `SaitoSlip` to the list of `SaitoSlip` inputs
    pub fn add_input(&mut self, slip: SlipReference) {
        self.inputs.push(slip);
    }

    /// Set the list of `Hop`s
    pub fn set_path(&mut self, path: Vec<Hop>) {
        self.path = path;
    }

    /// Add a new `Hop` to the list of `Hop`s
    pub fn add_hop_to_path(&mut self, path: Hop) {
        self.path.push(path);
    }

    /// Set the message
    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.message = msg;
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, transaction_body: TransactionBody) -> SignedTransaction {
        SignedTransaction {
            signature: signature,
            body: transaction_body,
        }
    }
    /// Set the `secp256k1::Signature`
    pub fn set_signature(&mut self, sig: Signature) {
        self.signature = sig;
    }
}
impl RuntimeTransaction {
    /// Creates new `Transaction`
    ///
    /// TODO
    pub fn new(tx_id: u64, signed_tx: SignedTransaction) -> RuntimeTransaction {
        RuntimeTransaction {
            block_id: 0,
            id: tx_id,
            signed_tx: signed_tx,
        }
    }
    /// Returns tx id, the ordinal of the tx in the serialized block
    pub fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{SaitoSlip, SlipBroadcastType, SlipReference},
    };

    #[test]
    fn transaction_test() {
        let mut tx = TransactionBody::new(TransactionBroadcastType::Normal);

        assert_eq!(tx.outputs(), &vec![]);
        assert_eq!(tx.inputs(), &vec![]);
        //assert_eq!(tx.signature(), &Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.broadcast_type(), &TransactionBroadcastType::Normal);
        assert_eq!(tx.path(), &vec![]);
        assert_eq!(tx.message(), &vec![]);

        let keypair = Keypair::new();
        let to_slip = SaitoSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        let mock_block_number: u64 = 1;
        let mock_tx_number: u64 = 1;
        let from_slip = SlipReference::new(mock_block_number, mock_tx_number, 0);

        let hop_message_bytes = Keypair::make_message_from_string("message_string");
        let signature = keypair.sign_message(&hop_message_bytes);
        let hop = Hop::new(keypair.public_key().clone(), signature);

        tx.add_output(to_slip);
        tx.add_input(from_slip);
        tx.add_hop_to_path(hop);

        assert_eq!(tx.outputs(), &vec![to_slip]);
        assert_eq!(tx.inputs(), &vec![from_slip]);
        assert_eq!(tx.path(), &vec![hop]);

        // tx.set_signature(signature.clone());
        // assert_eq!(tx.signature(), &signature);

        let message_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        tx.set_message(message_bytes.clone());
        assert_eq!(tx.message(), &message_bytes);
    }
}
