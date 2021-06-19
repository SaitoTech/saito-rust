use secp256k1::PublicKey;
use std::hash::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct Slip {

    publickey: [u8;32],  // 33 bytes, but deserialize not implemented for that so skipping for expedience
    uuid: [u8;32],
    amount: u64,
    sliptype: SlipType,

}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}

impl Slip {

    pub fn new() -> Self {
        Slip {
            publickey: [0;32], // 33 bytes
            amount: 0,
            uuid: [0;32],
            sliptype: SlipType::Normal,
        }
    }

    pub fn set_publickey(&mut self, publickey: [u8;32]) {
        self.publickey = publickey;
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    pub fn set_uuid(&mut self, uuid: [u8;32]) {
        self.uuid = uuid;
    }

    pub fn set_sliptype(&mut self, sliptype: SlipType) {
        self.sliptype = sliptype;
    }

    pub fn validate(&self, utxoset : &HashMap<Vec<u8>, u64>) -> bool {

	let slip_key = self.get_shashmap_slip_id();

        match utxoset.get(&slip_key) {
            Some(slip_value) => {
//		println!("Slip Value in UTXOSET: {:?}", slip_value)
	    },
            None => {
//		println!("No Slip Found in UTXOSET");
            }
        }

        return true;

    }

    pub fn on_chain_reorganization(&self, utxoset : &mut HashMap<Vec<u8>, u64>, longest_chain : bool , slip_value : u64) {

	let slip_key = self.get_shashmap_slip_id();
        utxoset.entry(slip_key).or_insert(slip_value);

    }

    pub fn get_shashmap_slip_id(&self) -> Vec<u8> {

      let mut res:Vec<u8> = [self.publickey, self.uuid].concat();
      res.extend(&self.amount.to_be_bytes());
      return res;

    }

}




