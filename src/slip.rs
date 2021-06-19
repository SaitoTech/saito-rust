use secp256k1::PublicKey;
use std::hash::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ahash::AHashMap;

//#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Slip {

    publickey: [u8;33],
    uuid: [u8;32],
    amount: u64,
    sliptype: SlipType,
    slipkey: [u8;47],

}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}

    pub fn default_slipkey() -> [u8;47] {
        return [1;47];
    }

impl Slip {

    pub fn new() -> Self {
        Slip {
            publickey: [0;33],
            amount: 0,
            uuid: [0;32],
            sliptype: SlipType::Normal,
    	    slipkey: [0;47],
        }
    }

    pub fn set_publickey(&mut self, publickey: [u8;33]) {
        self.publickey = publickey;
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    pub fn set_uuid(&mut self, uuid: [u8;32]) {
        self.uuid = uuid;
    }

    pub fn set_slipkey(&mut self, slipkey: [u8;47]) {
        self.slipkey = slipkey;
    }

    pub fn set_sliptype(&mut self, sliptype: SlipType) {
        self.sliptype = sliptype;
    }

    pub fn to_be_bytes(&self) -> Vec<u8> {

	let st = self.sliptype as u32;
	let mut vbytes : Vec<u8> = vec![];
	        vbytes.extend(&self.publickey);
	        vbytes.extend(&self.uuid);
	        vbytes.extend(&self.amount.to_be_bytes());
	        vbytes.extend(&(self.sliptype as u32).to_be_bytes());

	return vbytes;

    }

    pub fn validate(&self, utxoset : &AHashMap<[u8;47], u64>) -> bool {

	//let slip_key = self.get_shashmap_slip_id();
	let slip_key = self.slipkey;

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

    pub fn on_chain_reorganization(&self, utxoset : &mut AHashMap<[u8;47], u64>, longest_chain : bool , slip_value : u64) {

	//let slip_key = self.get_shashmap_slip_id();
	let slip_key = self.slipkey;
        utxoset.entry(slip_key).or_insert(slip_value);

    }

    pub fn get_shashmap_slip_id(&self) -> Vec<u8> {

      let mut res:Vec<u8> = vec![];
      res.extend(&self.publickey);
      res.extend(&self.uuid);
      res.extend(&self.amount.to_be_bytes());
      return res;

    }

}




