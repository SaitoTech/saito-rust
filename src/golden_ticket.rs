use crate::crypto::{SaitoPublicKey, SaitoHash};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldenTicket {
    vote: u8,
    target: SaitoHash,
    random: SaitoHash,
    #[serde_as(as = "[_; 33]")]
    publickey: SaitoPublicKey,
}

impl GoldenTicket {

    pub fn new(vote: u8, target: SaitoHash, random: SaitoHash, publickey: SaitoPublicKey) -> Self {
        return Self {
            vote,
            target,
            random,
            publickey,
        };
    }

    pub fn get_vote(&self) -> u8 {
        self.vote
    }

    pub fn get_target(&self) -> SaitoHash {
        self.target
    }

    pub fn get_random(&self) -> SaitoHash {
        self.random
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.publickey
    }

    pub fn serialize_for_transaction(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.vote.to_be_bytes());
        vbytes.extend(&self.target);
        vbytes.extend(&self.random);
        vbytes.extend(&self.publickey);
        vbytes
    }

    pub fn deserialize_for_transaction(bytes: Vec<u8>) -> GoldenTicket {
        let vote: u8 = u8::from_be_bytes(bytes[0..1].try_into().unwrap());
        let target: SaitoHash = bytes[1..33].try_into().unwrap();
        let random: SaitoHash = bytes[33..65].try_into().unwrap();
        let publickey: SaitoPublicKey = bytes[65..98].try_into().unwrap();
        GoldenTicket::new(vote, target, random, publickey)
    }


}

impl Default for GoldenTicket {
    fn default() -> Self {
        Self::new(0, [0; 32], [0; 32], [0; 33])
    }
}

