pub use secp256k1::{Message, PublicKey, Signature, SECP256K1};
use sha2::{Digest, Sha256};
use std::convert::TryInto;

//
// forward-looking flexibility
//
pub type SaitoHash = [u8; 32];
pub type SaitoPublicKey = [u8; 33];
pub type SaitoSignature = [u8; 64];


pub fn hash(data: &Vec<u8>) -> SaitoHash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
    //let results : [u8;32] = hasher.finalize().as_slice().try_into().unwrap()
    //hasher.finalize().try_into().expect("hash is wrong size")
}

pub fn verify(msg: &[u8], sig: SaitoSignature, publickey: SaitoPublicKey) -> bool {
    let m = Message::from_slice(msg).unwrap();
    let p = PublicKey::from_slice(&publickey).unwrap();
    let s = Signature::from_compact(&sig).unwrap();
    SECP256K1.verify(&m, &s, &p).is_ok()
}



