use std::convert::TryInto;

use sha2::{Digest, Sha256};
pub use secp256k1::{PublicKey, Signature};

pub fn hash(data: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}
