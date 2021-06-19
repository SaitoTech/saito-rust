use crate::{
    slip::Slip,
    path::Path,
    time::create_timestamp,
};
use secp256k1::Signature;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    id: u64,
    timestamp: u64,
    // compact signatures are 64 bytes; DER signatures are 68-72 bytes
    signature: Vec<u8>,
    inputs: Vec<Slip>,
    outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    //path: Path,
    transaction_type: TransactionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum TransactionType {
    Normal,
}


impl Transaction {

    pub fn new() -> Transaction {
        Transaction {
            id: 0,
	    timestamp: create_timestamp(),
            signature: vec![],
	    inputs: vec![],
	    outputs: vec![],
	    message: vec![],
            //path: Path::new(),
	    transaction_type: TransactionType::Normal,
        }
    }


    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.message = msg;
    }

    pub fn add_input(&mut self, input_slip: Slip) {
	self.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
	self.outputs.push(output_slip);
    }

    pub fn validate(&self, utxoset : &HashMap<Vec<u8>, u64>) -> bool {

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
    pub fn on_chain_reorganization(&self, utxoset : &mut HashMap<Vec<u8>, u64>, longest_chain : bool, block_id : u64) {

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


