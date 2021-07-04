use std::convert::TryInto;

use crate::{
    crypto::{
        hash, sign, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature,
        SaitoUTXOSetKey,
    },
    slip::{Slip, SLIP_SIZE},
    time::create_timestamp,
};
use ahash::AHashMap;
use enum_variant_count_derive::TryFromByte;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub const TRANSACTION_SIZE: usize = 85;

/// TransactionType is a human-readable indicator of the type of
/// transaction such as a normal user-initiated transaction, a
/// golden ticket transaction, a VIP-transaction or a rebroadcast
/// transaction created by a block creator, etc.
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone, TryFromByte)]
pub enum TransactionType {
    Normal,
    Fee,
    GoldenTicket,
    Other,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {

    // the bulk of the consensus transaction data
    timestamp: u64,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
    #[serde_as(as = "[_; 64]")]
    signature: SaitoSignature,

    // hash used for merkle_root (does not include signature), and slip uuid
    hash_for_signature: SaitoHash,

    pub total_in: u64,
    pub total_out: u64,
    pub total_fees: u64,
    pub cumulative_fees: u64,

    pub routing_work_to_me: u64,
    pub routing_work_to_creator: u64,
}

impl Transaction {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            inputs: vec![],
            outputs: vec![],
            message: vec![],
            transaction_type: TransactionType::Normal,
            signature: [0; 64],
            hash_for_signature: [0; 32],
            total_in: 0,
            total_out: 0,
            total_fees: 0,
            cumulative_fees: 0,
            routing_work_to_me: 0,
            routing_work_to_creator: 0,
        }
    }

    pub fn add_input(&mut self, input_slip: Slip) {
        self.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
        self.outputs.push(output_slip);
    }

    pub fn is_fee_transaction(&self) -> bool {
        if self.transaction_type == TransactionType::Fee {
            return true;
        }
        return false;
    }

    pub fn is_golden_ticket(&self) -> bool {
        if self.transaction_type == TransactionType::GoldenTicket {
            return true;
        }
        return false;
    }

    pub fn get_total_fees(&self) -> u64 {
        self.total_fees
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_transaction_type(&self) -> TransactionType {
        self.transaction_type
    }

    pub fn get_inputs(&self) -> &Vec<Slip> {
        &self.inputs
    }

    pub fn get_mut_inputs(&mut self) -> &mut Vec<Slip> {
        &mut self.inputs
    }

    pub fn get_mut_outputs(&mut self) -> &mut Vec<Slip> {
        &mut self.outputs
    }

    pub fn get_outputs(&self) -> &Vec<Slip> {
        &self.outputs
    }

    pub fn get_message(&self) -> &Vec<u8> {
        &self.message
    }

    pub fn get_hash_for_signature(&self) -> SaitoHash {
        self.hash_for_signature
    }

    pub fn get_signature(&self) -> [u8; 64] {
        self.signature
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn set_transaction_type(&mut self, transaction_type: TransactionType) {
        self.transaction_type = transaction_type;
    }

    pub fn set_inputs(&mut self, inputs : Vec<Slip>) {
        self.inputs = inputs;
    }

    pub fn set_outputs(&mut self, outputs : Vec<Slip>) {
        self.outputs = outputs;
    }

    pub fn set_message(&mut self, message: Vec<u8>) {
        self.message = message;
    }

    pub fn set_signature(&mut self, sig: SaitoSignature) {
        self.signature = sig;
    }

    pub fn set_hash_for_signature(&mut self, hash: SaitoHash) {
        self.hash_for_signature = hash;
    }

    pub fn sign(&mut self, privatekey: SaitoPrivateKey) {
        //
        // we set slip ordinals when signing
        //
        let mut slip_ordinal = 0;
        println!("signing tx with {} outputs: ", self.get_outputs().len());
        for output in self.get_mut_outputs() {
            println!("updating slip ordinal: {}", slip_ordinal);
            output.set_slip_ordinal(slip_ordinal);
            slip_ordinal += 1;
        }

        let hash_for_signature = hash(&self.serialize_for_signature());
        self.set_signature(sign(&hash_for_signature, privatekey));
        self.set_hash_for_signature(hash_for_signature);
    }

    pub fn serialize_for_signature(&self) -> Vec<u8> {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.timestamp.to_be_bytes());
        for input in &self.inputs {
            vbytes.extend(&input.serialize_for_signature());
        }
        for output in &self.outputs {
            vbytes.extend(&output.serialize_for_signature());
        }
        vbytes.extend(&(self.transaction_type as u32).to_be_bytes());
        vbytes.extend(&self.message);

        vbytes
    }
    /// Deserialize from bytes to a Transaction.
    /// [len of inputs - 4 bytes - u32]
    /// [len of outputs - 4 bytes - u32]
    /// [len of message - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    pub fn deserialize_from_net(bytes: Vec<u8>) -> Transaction {
        let inputs_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let outputs_len: u32 = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let message_len: usize = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let signature: SaitoSignature = bytes[12..76].try_into().unwrap();

        let timestamp: u64 = u64::from_be_bytes(bytes[76..84].try_into().unwrap());
        let transaction_type: TransactionType = TransactionType::try_from(bytes[84]).unwrap();
        let start_of_inputs = TRANSACTION_SIZE;
        let start_of_outputs = start_of_inputs + inputs_len as usize * SLIP_SIZE;
        let start_of_message = start_of_outputs + outputs_len as usize * SLIP_SIZE;
        let mut inputs: Vec<Slip> = vec![];
        for n in 0..inputs_len {
            let start_of_data: usize = start_of_inputs as usize + n as usize * SLIP_SIZE;
            let end_of_data: usize = start_of_data + SLIP_SIZE;
            let input = Slip::deserialize_from_net(bytes[start_of_data..end_of_data].to_vec());
            inputs.push(input);
        }
        let mut outputs: Vec<Slip> = vec![];
        for n in 0..outputs_len {
            let start_of_data: usize = start_of_outputs as usize + n as usize * SLIP_SIZE;
            let end_of_data: usize = start_of_data + SLIP_SIZE;
            let output = Slip::deserialize_from_net(bytes[start_of_data..end_of_data].to_vec());
            outputs.push(output);
        }
        let message = bytes[start_of_message..start_of_message + message_len]
            .try_into()
            .unwrap();
        let mut transaction = Transaction::new();
	transaction.set_timestamp(timestamp);
	transaction.set_inputs(inputs);
	transaction.set_outputs(outputs);
	transaction.set_message(message);
	transaction.set_transaction_type(transaction_type);
	transaction.set_signature(signature);
	transaction
    }

    /// Serialize a Transaction for transport or disk.
    /// [len of inputs - 4 bytes - u32]
    /// [len of outputs - 4 bytes - u32]
    /// [len of message - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.inputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.outputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.message.len() as u32).to_be_bytes());
        vbytes.extend(&self.signature);
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&(self.transaction_type as u8).to_be_bytes());
        for input in &self.inputs {
            vbytes.extend(&input.serialize_for_net());
        }
        for output in &self.outputs {
            vbytes.extend(&output.serialize_for_net());
        }
        vbytes.extend(&self.message);
        vbytes
    }
    /// Serialize a Transaction for transport or disk.
    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        longest_chain: bool,
        block_id: u64,
    ) {
        if longest_chain {
            for input in self.get_inputs() {
                input.on_chain_reorganization(utxoset, longest_chain, block_id);
            }
            for output in self.get_outputs() {
                output.on_chain_reorganization(utxoset, longest_chain, 1);
            }
        } else {
            for input in self.get_inputs() {
                input.on_chain_reorganization(utxoset, longest_chain, 1);
            }
            for output in self.get_outputs() {
                output.on_chain_reorganization(utxoset, longest_chain, 0);
            }
        }
    }

    //
    // we have to calculate cumulative fees sequentially.
    //
    pub fn pre_validation_calculations_cumulative_fees(&mut self, cumulative_fees: u64) -> u64 {
        self.cumulative_fees = cumulative_fees + self.total_fees;
        return self.cumulative_fees;
    }
    pub fn pre_validation_calculations_parallelizable(&mut self) -> bool {
        //
        // and save the hash_for_signature so we can use it later...
        //
        let hash_for_signature: SaitoHash = hash(&self.serialize_for_signature());
        self.set_hash_for_signature(hash_for_signature);

        //
        // calculate nolan in / out, fees
        //
        let mut nolan_in: u64 = 0;
        let mut nolan_out: u64 = 0;
        for input in &self.inputs {
            nolan_in += input.get_amount();
        }
        for output in &self.outputs {
            nolan_out += output.get_amount();
        }
        self.total_in = nolan_in;
        self.total_out = nolan_out;
        self.total_fees = 0;

        //
        // note that this is not validation code, permitting this. we may have
        // some transactions that do insert NOLAN, such as during testing of
        // monetary policy. All sanity checks need to be in the validate()
        // function.
        //
        if nolan_in > nolan_out {
            self.total_fees = nolan_in - nolan_out;
        }

        true
    }
    pub fn validate(&self) -> bool {
        //
        // VALIDATE signature valid
        //
        let hash_for_signature: SaitoHash = self.get_hash_for_signature();
        let sig: SaitoSignature = self.get_signature();

        if self.inputs.is_empty() {
            panic!("transaction must have at least 1 input");
        }
        let publickey: SaitoPublicKey = self.inputs[0].get_publickey();

        if !verify(&hash_for_signature, sig, publickey) {
            println!("message verifies not");
            return false;
        }

        //
        // VALIDATE min one sender and receiver
        //
        if self.get_inputs().len() < 1 {
            println!("ERROR 582039: less than 1 input in transaction");
            return false;
        }
        if self.get_outputs().len() < 1 {
            println!("ERROR 582039: less than 1 output in transaction");
            return false;
        }

        //
        // VALIDATE no negative payments
        //
        // Rust Types prevent these variables being < 0
        //        if nolan_in < 0 {
        //            println!("ERROR 672939: negative payment in transaction from slip");
        //            return false;
        //        }
        //        if nolan_out < 0 {
        //            println!("ERROR 672940: negative payment in transaction to slip");
        //            return false;
        //        }
        //
        // we make an exception for fee transactions, which may be pulling revenue from the
        // treasury in some amount.
        if self.total_out > self.total_in && self.get_transaction_type() != TransactionType::Fee {
            println!("ERROR 672941: transaction spends more than it has available");
            return false;
        }

        //
        // VALIDATE UTXO inputs
        //
        for input in &self.inputs {
            if !input.validate() {
                return false;
            }
        }
        true
    }
}


#[cfg(test)]
mod tests {
    use crate::slip::Slip;

    use super::*;

    #[test]
    fn transaction_new_test() {
        let tx = Transaction::new();
        assert_eq!(tx.timestamp, 0);
        assert_eq!(tx.inputs, vec![]);
        assert_eq!(tx.outputs, vec![]);
        assert_eq!(tx.message, Vec::<u8>::new());
        assert_eq!(tx.transaction_type, TransactionType::Normal);
        assert_eq!(tx.signature, [0; 64]);
    }

    #[test]
    fn serialize_for_net_test() {
        let mock_input = Slip::new();
        let mock_output = Slip::new();
        let mut mock_tx = Transaction::new();
	mock_tx.set_timestamp(create_timestamp());
	mock_tx.add_input(mock_input);
	mock_tx.add_output(mock_output);
	mock_tx.set_message(vec![104, 101, 108, 108, 111]);
        mock_tx.set_transaction_type(TransactionType::Normal);
	mock_tx.set_signature([1; 64]);
        let serialized_tx = mock_tx.serialize_for_net();
        let deserialized_tx = Transaction::deserialize_from_net(serialized_tx);
        assert_eq!(mock_tx, deserialized_tx);
    }
}
