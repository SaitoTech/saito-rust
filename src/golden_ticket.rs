use crate::crypto::{hash, SaitoHash, SaitoPublicKey};
use bigint::uint::U256;
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

    // TODO - review exact solution generated and mechanism to determine validity
    pub fn generate_solution(random_bytes: SaitoHash, publickey: SaitoPublicKey) -> SaitoHash {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&random_bytes);
        vbytes.extend(&publickey);

        hash(&vbytes)
    }

    // TODO - review exact algorithm in use here
    pub fn is_valid_solution(target: SaitoHash, solution: SaitoHash, difficulty: u64) -> bool {
        let difficulty_order = (difficulty as f64 / 1_0000_0000_f64).round() as usize;
        let difficulty_grain = difficulty % 1_0000_0000;

        let random_solution = U256::from_big_endian(&solution[..difficulty_order]);
        let target_solution = U256::from_big_endian(&target[..difficulty_order]);
        let difficulty_grain = U256::from(difficulty_grain * 16);

        random_solution >= target_solution
            && (random_solution - target_solution) <= difficulty_grain
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
