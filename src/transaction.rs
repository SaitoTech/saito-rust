use crate::{
    crypto::{hash},
    keypair::Keypair,
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ahash::AHashMap;


//#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    id: u64,
    timestamp: u64,
    // compact signatures are 64 bytes; DER signatures are 68-72 bytes
    signature: [u8;64],
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    //#[serde(with = "serde_bytes")]
    message: Vec<u8>,
    //path: Path,
    transaction_type: TransactionType,
}

#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


impl Transaction {

    pub fn new() -> Transaction {
        Transaction {
            id: 0,
	    timestamp: create_timestamp(),
            signature: [0;64],
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
            //path: Path::new(),
	    transaction_type: TransactionType::Normal,
        }
    }


    pub fn add_input(&mut self, input_slip: Slip) {
	self.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
	self.outputs.push(output_slip);
    }

    pub fn get_signature(&mut self) -> [u8;64] {
       self.signature
    }

    pub fn get_signature_source(&self) -> [u8;32] {

	let mut vbytes : Vec<u8> = vec![];
	        vbytes.extend(&self.timestamp.to_be_bytes());
		for input in &self.inputs { vbytes.extend(&input.to_be_bytes()); }
		for output in &self.outputs { vbytes.extend(&output.to_be_bytes()); }
	        vbytes.extend(&(self.transaction_type as u32).to_be_bytes());
		vbytes.extend(&self.message);

        let message_hash = hash(&vbytes);
	return message_hash;

    }

    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.message = msg;
    }

    pub fn set_signature(&mut self, signature: [u8;64]) {
       self.signature = signature;
    }

    pub fn sign_transaction(&mut self, keypair: &Keypair) {
        let message_hash = self.get_signature_source();
        let signature = keypair.sign_message(&message_hash[..]);
	self.set_signature(signature.serialize_compact());
    }


    pub fn validate(&self, utxoset : &AHashMap<[u8;47], u64>) -> bool {

	//
	// UTXO validate inputs
	//
	for input in &self.inputs {
  	    if !input.validate(utxoset) {
		return false;
	    }
	}
	return true;

    }

    // TODO -- shashmap values are nonsensical 0 unspendable, 1 spendable, block_id = when spent -- just testing speeds here
    pub fn on_chain_reorganization(&self, utxoset : &mut AHashMap<[u8;47], u64>, longest_chain : bool, block_id : u64) {

	if longest_chain {
	    for input in &self.inputs {
    	        input.on_chain_reorganization(utxoset, longest_chain, block_id);
	    }
	    for output in &self.outputs {
    	        output.on_chain_reorganization(utxoset, longest_chain, 1);
	    }
	} else {
	    for input in &self.inputs {
    	        input.on_chain_reorganization(utxoset, longest_chain, 1);
	    }
	    for output in &self.outputs {
    	        output.on_chain_reorganization(utxoset, longest_chain, 0);
	    }


	}
    }

}


