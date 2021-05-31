use crate::{
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}

/// A record containging data of funds between transfered between public addresses. It
/// contains additional information as an optinal message field to transfer data around the network
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    /// `secp256k1::Signature` verifying authenticity of `TransactionCore` data
    signature: Signature,
    /// A list of `Hop` stipulating the history of `Transaction` routing
    path: Vec<Hop>,
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
    message: Vec<u8>,
}

impl Transaction {
    ///
    pub fn new_mock() -> Transaction {
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
        // TODO add inputs, outputs, and message here
        Transaction {
            signature: signature,
            path: path,
            core: TransactionCore {
                timestamp: timestamp,
                inputs: inputs,
                outputs: outputs,
                broadcast_type: broadcast_type,
                message: message,
            },
        }
    }

    pub fn sign(core: TransactionCore) -> Transaction {
        Transaction {
            signature: Signature::from_compact(&[0; 64]).unwrap(),
            path: vec![],
            core: core,
        }
    }
    pub fn add_signature(core: TransactionCore, signature: Signature) -> Transaction {
        Transaction {
            signature: signature,
            path: vec![],
            core: core,
        }
    }
    /// Returns `secp256k1::Signature` verifying the validity of data on a transaction
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Add a new `Hop` to the list of `Hop`s
    pub fn add_hop_to_path(&mut self, path: Hop) {
        self.path.push(path);
    }
    /// validates sig
    pub fn sig_is_valid(&self) -> bool {
        // TODO check with keypair if things are valid
        true
    }
    /// validates slip against utxoset
    pub fn are_slips_valid(&self) -> bool {
        // self.inputs.iter()
        // result = result && self.is_slip_spendable(slip_id);
        // result = result && self.is_slip_id_valid(slip_id, output_slip);
        result
    }
    /// Returns true if the slip has been seen in the blockchain
    fn is_slip_spendable(&self, _slip_id: &SlipID) -> bool {
        // TODO check with utxoset to see if slip is spendable
        true
    }
    // Returns true if the OutputSlip found in the utxoset matches the OutputSlip
    fn is_slip_id_valid(&self, _slip_id: &SlipID, _slip_as_output: &OutputSlip) -> bool {
        // TODO loop through all sigs in utxo set and make sure they have to correct receiver and amount
        true
    }


}

impl TransactionCore {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{OutputSlip, SlipID, SlipType},
    };

    #[test]
    fn transaction_test() {
        let mut tx: Transaction = Transaction::new_mock();

        assert_eq!(tx.core.outputs(), &vec![]);
        assert_eq!(tx.core.inputs(), &vec![]);
        //assert_eq!(tx.signature(), &Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.core.broadcast_type(), &TransactionType::Normal);
        //assert_eq!(tx.path(), &vec![]);
        assert_eq!(tx.core.message(), &vec![]);

        let keypair = Keypair::new();
        let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 0);
        let from_slip = SlipID::new_mock();

        // let hop_message_bytes = Keypair::make_message_from_string("message_string");
        // let signature = keypair.sign_message(&hop_message_bytes);
        // let hop = Hop::new(keypair.public_key().clone(), signature);

        tx.core.add_output(to_slip);
        tx.core.add_input(from_slip);

        assert_eq!(tx.core.outputs(), &vec![to_slip]);
        assert_eq!(tx.core.inputs(), &vec![from_slip]);
        // assert_eq!(tx.path(), &vec![hop]);

        // let message_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        // assert_eq!(tx.message(), &message_bytes);
    }
}
