use crate::{
    crypto::{hash_bytes, Sha256Hash},
    keypair::Keypair,
    slip::{OutputSlip, SlipID},
    time::create_timestamp,
};
use secp256k1::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

/// A single record used in the history of transactions being routed around the network
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
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

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum TransactionType {
    Normal,
    GoldenTicket,
}

/// A record containging data of funds between transfered between public addresses. It
/// contains additional information as an optinal message field to transfer data around the network
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    /// `secp256k1::Signature` verifying authenticity of `TransactionCore` data
    signature: Signature,
    /// A list of `Hop` stipulating the history of `Transaction` routing
    path: Vec<Hop>,
    /// hash of the tx
    hash: Sha256Hash,
    /// All data which is serialized and signed
    pub core: TransactionCore,
}

/// Core data to be serialized/deserialized of `Transaction`
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionCore {
    /// UNIX timestamp when the `Transaction` was created
    timestamp: u64,
    /// A list of `SlipID` inputs
    inputs: Vec<SlipID>,
    /// A list of `OutputSlip` outputs
    outputs: Vec<OutputSlip>,
    /// A enum brodcast type determining how to process `Transaction` in consensus
    broadcast_type: TransactionType,
    /// A byte array of miscellaneous information
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionSignatureCore {
    /// UNIX timestamp when the `Transaction` was created
    timestamp: u64,
    /// A list of `SlipID` inputs
    inputs: Vec<SlipID>,
    /// A list of `OutputSlip` outputs
    outputs: Vec<OutputSlip>,
    /// A enum brodcast type determining how to process `Transaction` in consensus
    broadcast_type: TransactionType,
}

impl Transaction {
    ///
    pub fn default() -> Transaction {
        Transaction::new(
            Signature::from_compact(&[0; 64]).unwrap(),
            vec![],
            create_timestamp(),
            vec![],
            vec![],
            TransactionType::Normal,
            vec![],
        )
    }

    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionType` of the new `Transaction`
    pub fn new(
        signature: Signature,
        path: Vec<Hop>,
        timestamp: u64,
        inputs: Vec<SlipID>,
        outputs: Vec<OutputSlip>,
        broadcast_type: TransactionType,
        message: Vec<u8>,
    ) -> Transaction {
        // For the constructors of Transaction, Rust tries to be helpful and disallows
        // us from createing a core and then getting a hash from it using a shared borrow and
        // then moving it to the Transaction. To circumvent this, we instead let the tx own
        // the core and then get a shared borrow of it through the core() getter and use that
        // to generate and assign the hash.
        let mut tx = Transaction {
            signature: signature,
            path: path,
            hash: [0; 32],
            core: TransactionCore {
                timestamp: timestamp,
                inputs: inputs,
                outputs: outputs,
                broadcast_type: broadcast_type,
                message: message,
            },
        };
        tx.set_hash(hash_bytes(&tx.core().sig_core().into()));
        tx
    }

    /// Construct a new tx from the Transaction Core and sign it
    pub fn sign(core: TransactionCore) -> Transaction {
        let mut tx = Transaction {
            signature: Signature::from_compact(&[0; 64]).unwrap(),
            path: vec![],
            hash: [0; 32],
            core: core,
        };
        tx.set_hash(hash_bytes(&tx.core().sig_core().into()));
        tx
    }

    /// Construct a new tx with signature from the Transaction Core
    pub fn add_signature(core: TransactionCore, signature: Signature) -> Transaction {
        let mut tx = Transaction {
            signature: signature,
            path: vec![],
            hash: [0; 32],
            core: core,
        };
        tx.set_hash(hash_bytes(&tx.core().sig_core().into()));
        tx
    }

    /// Create signature and new `Transaction`
    pub fn create_signature(core: TransactionCore, keypair: &Keypair) -> Transaction {
        let message_bytes: Vec<u8> = core.sig_core().into();
        let message_hash = hash_bytes(&message_bytes);
        let signature = keypair.sign_message(&message_hash[..]);
        Transaction::add_signature(core, signature)
    }
    ///
    pub fn core(&self) -> &TransactionCore {
        &self.core
    }
    /// Returns `secp256k1::Signature` verifying the validity of data on a transaction
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn hash(&self) -> Sha256Hash {
        self.hash
    }

    fn set_hash(&mut self, hash: Sha256Hash) {
        self.hash = hash;
    }

    pub fn path(&self) -> &Vec<Hop> {
        &self.path
    }

    /// Add a new `Hop` to the list of `Hop`s
    pub fn add_hop_to_path(&mut self, path: Hop) {
        self.path.push(path);
    }
}

impl From<Vec<u8>> for Transaction {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for Transaction {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

impl From<Vec<u8>> for TransactionCore {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for TransactionCore {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

impl Into<Vec<u8>> for &TransactionCore {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

impl Into<Vec<u8>> for TransactionSignatureCore {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

impl TransactionCore {
    /// Default `TransactionCore` creation
    pub fn default() -> Self {
        TransactionCore {
            timestamp: create_timestamp(),
            inputs: vec![],
            outputs: vec![],
            broadcast_type: TransactionType::Normal,
            message: vec![],
        }
    }

    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionType` of the new `Transaction`
    pub fn new(
        timestamp: u64,
        inputs: Vec<SlipID>,
        outputs: Vec<OutputSlip>,
        broadcast_type: TransactionType,
        message: Vec<u8>,
    ) -> Self {
        TransactionCore {
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
            broadcast_type: broadcast_type,
            message: message,
        }
    }

    /// Returns a timestamp when `Transaction` was created
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns list of `OutputSlip` outputs
    pub fn outputs(&self) -> &Vec<OutputSlip> {
        &self.outputs
    }

    /// Returns list of mutable `OutputSlip` outputs
    pub fn outputs_mut(&mut self) -> &mut Vec<OutputSlip> {
        &mut self.outputs
    }

    /// Add a new `OutputSlip` to the list of `Slip` outputs
    pub fn add_output(&mut self, slip: OutputSlip) {
        self.outputs.push(slip);
    }

    /// Returns list of `SlipID` inputs
    pub fn inputs(&self) -> &Vec<SlipID> {
        &self.inputs
    }

    /// Returns list of `SlipID` inputs
    pub fn inputs_mut(&mut self) -> &mut Vec<SlipID> {
        &mut self.inputs
    }

    /// Add a new `SlipID` to the list of `SlipID` inputs
    pub fn add_input(&mut self, slip: SlipID) {
        self.inputs.push(slip);
    }

    /// Returns `TransactionType` of the `Transaction`
    pub fn broadcast_type(&self) -> &TransactionType {
        &self.broadcast_type
    }

    /// Returns the message of the `Transaction`
    pub fn message(&self) -> &Vec<u8> {
        &self.message
    }

    pub fn sig_core(&self) -> TransactionSignatureCore {
        TransactionSignatureCore {
            timestamp: self.timestamp(),
            inputs: self.inputs().clone(),
            outputs: self.outputs().clone(),
            broadcast_type: self.broadcast_type().clone(),
        }
    }

    pub fn set_type(&mut self, tx_type: TransactionType) {
        self.broadcast_type = tx_type;
    }

    pub fn set_message(&mut self, message: Vec<u8>) {
        self.message = message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::{hash_bytes, verify_bytes_message},
        keypair::Keypair,
        slip::{OutputSlip, SlipID, SlipType},
    };

    #[test]
    fn transaction_test() {
        let mut tx: Transaction = Transaction::default();

        assert_eq!(tx.core.outputs(), &vec![]);
        assert_eq!(tx.core.inputs(), &vec![]);
        assert_eq!(tx.core.broadcast_type(), &TransactionType::Normal);
        let message: Vec<u8> = vec![];
        assert_eq!(tx.core.message(), &message);

        let keypair = Keypair::new();
        let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 0);
        let from_slip = SlipID::default();

        tx.core.add_output(to_slip);
        tx.core.add_input(from_slip);

        assert_eq!(tx.core.outputs(), &vec![to_slip]);
        assert_eq!(tx.core.inputs(), &vec![from_slip]);
    }

    #[ignore]
    #[test]
    fn transaction_signature_test() {
        let keypair = Keypair::new();

        let mut tx_core = TransactionCore::default();
        tx_core.set_message(vec![255, 255, 255, 255, 255, 255, 255]);

        let tx = Transaction::create_signature(tx_core, &keypair);
        let serialize_tx: Vec<u8> = tx.core.clone().into();
        let hash = hash_bytes(&serialize_tx);

        assert!(verify_bytes_message(
            &hash,
            &tx.signature(),
            keypair.public_key()
        ));
    }
}
