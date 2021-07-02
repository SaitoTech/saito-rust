use crate::crypto::SaitoPublicKey;
use serde::{Deserialize, Serialize};

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldenTicket {
    vote: u8,
    target: [u8; 32],
    random: [u8; 32],
    #[serde_as(as = "[_; 33]")]
    publickey: SaitoPublicKey,
}

impl GoldenTicket {
    pub fn new(vote: u8, target: [u8; 32], random: [u8; 32], publickey: SaitoPublicKey) -> Self {
        return Self {
            vote,
            target,
            random,
            publickey,
        };
    }
}

impl Default for GoldenTicket {
    fn default() -> Self {
        Self::new(0, [0; 32], [0; 32], [0; 33])
    }
}

