use crate::{
    time::create_timestamp,
    slip::Slip,
};
use crate::crypto::{SaitoSignature, SaitoPublicKey, SaitoPrivateKey, hash, sign};
use serde::{Deserialize, Serialize};


//
// TransactionType is a human-readable indicator of the type of 
// transaction such as a normal user-initiated transaction, a 
// golden ticket transaction, a VIP-transaction or a rebroadcast
// transaction created by a block creator, etc.
//
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


//
// TransactionCore is a self-contained object containing only the core
// information about the transaction that exists regardless of how it 
// was routed or produced. It exists to simplify transaction serialization
// and deserialization until we have custom functions.
//
// This is a private variable. Access to variables within the 
// TransactionCore should be handled through getters and setters in the 
// block which surrounds it.
//
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TransactionCore {
    timestamp: u64,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
    #[serde_as(as = "[_; 64]")]
    signature: SaitoSignature,
}
impl TransactionCore {
    pub fn new() -> TransactionCore {
        TransactionCore {
	    timestamp: create_timestamp(),
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
	    transaction_type: TransactionType::Normal,
	    signature: [0;64],
        }
    }
}


#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    core: TransactionCore,
}

impl Transaction {
    pub fn new() -> Transaction {
        Transaction {
	    core: TransactionCore::new(),
        }
    }


    pub fn add_input(&mut self, input_slip: Slip) {
        self.core.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
        self.core.outputs.push(output_slip);
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

    pub fn get_signature(&self) -> [u8;64] {
        self.core.signature
    }

    pub fn set_timestamp(&mut self, timestamp : u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_transaction_type(&mut self, transaction_type : TransactionType) {
        self.core.transaction_type = transaction_type;
    }

    pub fn set_message(&mut self, message : Vec<u8>) {
        self.core.message = message;
    }

    // TODO - this is just a stub
    pub fn set_signature(&mut self, sig : SaitoSignature) {
	self.core.signature = sig;
    }

    pub fn sign(&mut self, privatekey : SaitoPrivateKey) {

      let vbytes = self.serialize_for_transaction_signature();
      let hash = hash(&vbytes);
      let sig = sign(&hash, privatekey);
      self.set_signature(sig); 

    }
    pub fn serialize_for_transaction_signature(&self) -> Vec<u8> {

        //
        // create array of bytes -- should be faster than bincode
        //
        let mut vbytes : Vec<u8> = vec![];
                vbytes.extend(&self.core.timestamp.to_be_bytes());
                for input in &self.core.inputs { vbytes.extend(&input.serialize_for_transaction_signature()); }
                for output in &self.core.outputs { vbytes.extend(&output.serialize_for_transaction_signature()); }
                vbytes.extend(&(self.core.transaction_type as u32).to_be_bytes());
                vbytes.extend(&self.core.message);

	return vbytes;

    }


}



