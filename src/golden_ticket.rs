use serde::{Serialize, Deserialize};
use crate::{
    crypto::PublicKey
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldenTicket {
    target: [u8; 32],
    vote: u8,
    random: [u8; 32],
    publickey: PublicKey
}

impl GoldenTicket {
    pub fn new(vote: u8, target: [u8; 32], random: [u8; 32], publickey: PublicKey) -> Self {
        return GoldenTicket{vote, target, random, publickey};
    }

    pub fn calculate_difficulty (&self, previous_difficulty: f32) -> f32 {
        return match self.vote {
            1 => previous_difficulty + 0.01,
            _ => previous_difficulty - 0.01
        }
    }

    pub fn calculate_paysplit (&self, previous_paysplit: f32) -> f32 {
        return match self.vote {
            1 => previous_paysplit + 0.01,
            _ => previous_paysplit - 0.01
        }
    }
}
