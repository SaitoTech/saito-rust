use secp256k1::PublicKey;
use std::hash::Hash;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct Slip {

    publickey: Vec<u8>,
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
            publickey: vec![],
            amount: 0,
            uuid: [0;32],
            sliptype: SlipType::Normal,
        }
    }

}




