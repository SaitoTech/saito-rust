use crate::{
    big_array::BigArray,
    block::Block,
    crypto::{
        hash, sign, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature,
        SaitoUTXOSetKey,
    },
    slip::Slip,
    time::create_timestamp,
};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

/// TransactionType is a human-readable indicator of the type of
/// transaction such as a normal user-initiated transaction, a
/// golden ticket transaction, a VIP-transaction or a rebroadcast
/// transaction created by a block creator, etc.
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}

/// TransactionCore is a self-contained object containing only the core
/// information about the transaction that exists regardless of how it
/// was routed or produced. It exists to simplify transaction serialization
/// and deserialization until we have custom functions.
///
/// This is a private variable. Access to variables within the
/// TransactionCore should be handled through getters and setters in the
/// block which surrounds it.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionCore {
    timestamp: u64,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
    #[serde(with = "BigArray")]
    signature: SaitoSignature, // compact signatures are 64 bytes; DER signatures are 68-72 bytes
}

impl TransactionCore {
    pub fn new(
        timestamp: u64,
        inputs: Vec<Slip>,
        outputs: Vec<Slip>,
        message: Vec<u8>,
        transaction_type: TransactionType,
        signature: SaitoSignature,
    ) -> Self {
        Self {
            timestamp,
            inputs,
            outputs,
            message,
            transaction_type,
            signature,
        }
    }
}

impl Default for TransactionCore {
    fn default() -> Self {
        Self::new(
            create_timestamp(),
            vec![],
            vec![],
            vec![],
            TransactionType::Normal,
            [0; 64],
        )
    }
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    // the bulk of the consensus transaction data
    core: TransactionCore,
    // hash used for merkle_root (does not include signature), and slip uuid
    hash_for_signature: SaitoHash,
}

impl Transaction {
    pub fn new(core: TransactionCore) -> Self {
        Self {
            core,
            hash_for_signature: [0; 32],
        }
    }

    pub fn add_input(&mut self, input_slip: Slip) {
        self.core.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
        self.core.outputs.push(output_slip);
    }

    pub fn calculate_work(&mut self, block : Block) {

    }

    pub fn get_timestamp(&self) -> u64 {
        self.core.timestamp
    }

    pub fn get_transaction_type(&self) -> TransactionType {
        self.core.transaction_type
    }

    pub fn get_inputs(&self) -> &Vec<Slip> {
        &self.core.inputs
    }

    pub fn get_outputs(&self) -> &Vec<Slip> {
        &self.core.outputs
    }

    pub fn get_message(&self) -> &Vec<u8> {
        &self.core.message
    }

    pub fn get_hash_for_signature(&self) -> SaitoHash {
        self.hash_for_signature
    }

    pub fn get_signature(&self) -> [u8; 64] {
        self.core.signature
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_transaction_type(&mut self, transaction_type: TransactionType) {
        self.core.transaction_type = transaction_type;
    }

    pub fn set_message(&mut self, message: Vec<u8>) {
        self.core.message = message;
    }

    pub fn set_signature(&mut self, sig: SaitoSignature) {
        self.core.signature = sig;
    }

    pub fn set_hash_for_signature(&mut self, hash: SaitoHash) {
        self.hash_for_signature = hash;
    }

    pub fn sign(&mut self, privatekey: SaitoPrivateKey) {
        let hash_for_signature = hash(&self.serialize_for_signature());
        self.set_signature(sign(&hash_for_signature, privatekey));
        self.set_hash_for_signature(hash_for_signature);
    }

    pub fn serialize_for_signature(&self) -> Vec<u8> {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.timestamp.to_be_bytes());
        for input in &self.core.inputs {
            vbytes.extend(&input.serialize_for_signature());
        }
        for output in &self.core.outputs {
            vbytes.extend(&output.serialize_for_signature());
        }
        vbytes.extend(&(self.core.transaction_type as u32).to_be_bytes());
        vbytes.extend(&self.core.message);

        vbytes
    }

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

    pub fn validate_pre_calculations(&mut self) -> bool {

	//
	// and save the hash_for_signature so we can use it later...
	//
        let hash_for_signature: SaitoHash = hash(&self.serialize_for_signature());
	self.set_hash_for_signature(hash_for_signature);

	true
    }
    pub fn validate(&self) -> bool {

        //
        // VALIDATE signature valid
        //
        let hash_for_signature: SaitoHash = self.get_hash_for_signature();
        let sig: SaitoSignature = self.get_signature();
        let mut publickey: SaitoPublicKey = [0; 33];
        if self.core.inputs.len() > 0 {
            publickey = self.core.inputs[0].get_publickey();
        }

        if !verify(&hash_for_signature, sig, publickey) {
            println!("message verifies not");
            return false;
        }


	//
	// VALIDATE min one sender and receiver
	//
        if self.get_inputs().len() < 1 {
	    println!("ERROR 582039: less than 1 input in transaction");
	    return false
	}
        if self.get_outputs().len() < 1 {
	    println!("ERROR 582039: less than 1 output in transaction");
	    return false 
	}


	//
	// VALIDATE no negative payments
	//
        let mut nolan_in: u64 = 0;
        let mut nolan_out: u64 = 0;
        for input in &self.core.inputs { nolan_in += input.get_amount(); }
        for output in &self.core.outputs { nolan_out += output.get_amount(); }
	if nolan_in < 0 {
	    println!("ERROR 672939: negative payment in transaction from slip");
	    return false;
	}
	if nolan_out < 0 {
	    println!("ERROR 672940: negative payment in transaction to slip");
	    return false;
	}
	if nolan_out > nolan_in {
	    println!("ERROR 672941: transaction spends more than it has available");
	    return false;
	}


        //
        // VALIDATE UTXO inputs
        //
        for input in &self.core.inputs {
            if !input.validate() {
                return false;
            }
        }

        return true;
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new(TransactionCore::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_core_default_test() {
        let timestamp = create_timestamp();
        let tx_core = TransactionCore::default();
        assert_eq!(tx_core.timestamp, timestamp);
        assert_eq!(tx_core.inputs, vec![]);
        assert_eq!(tx_core.outputs, vec![]);
        assert_eq!(tx_core.message, Vec::<u8>::new());
        assert_eq!(tx_core.transaction_type, TransactionType::Normal);
    }

    #[test]
    fn transaction_core_new_test() {
        let timestamp = create_timestamp();
        let tx_core = TransactionCore::new(
            timestamp,
            vec![],
            vec![],
            vec![],
            TransactionType::Normal,
            [0; 64],
        );
        assert_eq!(tx_core.timestamp, timestamp);
        assert_eq!(tx_core.inputs, vec![]);
        assert_eq!(tx_core.outputs, vec![]);
        assert_eq!(tx_core.message, Vec::<u8>::new());
        assert_eq!(tx_core.transaction_type, TransactionType::Normal);
        assert_eq!(tx_core.signature, [0; 64]);
    }

    #[test]
    fn transaction_default_test() {
        let timestamp = create_timestamp();
        let tx = Transaction::default();
        assert_eq!(tx.core.timestamp, timestamp);
        assert_eq!(tx.core.inputs, vec![]);
        assert_eq!(tx.core.outputs, vec![]);
        assert_eq!(tx.core.message, Vec::<u8>::new());
        assert_eq!(tx.core.transaction_type, TransactionType::Normal);
        assert_eq!(tx.core.signature, [0; 64]);
    }

    #[test]
    fn transaction_new_test() {
        let timestamp = create_timestamp();
        let tx_core = TransactionCore::default();
        let tx = Transaction::new(tx_core);
        assert_eq!(tx.core.timestamp, timestamp);
        assert_eq!(tx.core.inputs, vec![]);
        assert_eq!(tx.core.outputs, vec![]);
        assert_eq!(tx.core.message, Vec::<u8>::new());
        assert_eq!(tx.core.transaction_type, TransactionType::Normal);
        assert_eq!(tx.core.signature, [0; 64]);
    }
}
