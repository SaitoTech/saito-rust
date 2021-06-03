use std::convert::TryInto;

use base58::FromBase58;
pub use secp256k1::{Message, PublicKey, Signature, SECP256K1};
use sha2::{Digest, Sha256};

pub type Sha256Hash = [u8; 32];

pub fn hash(data: &Vec<u8>) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}

/// Hash the message string with sha256 for signing by secp256k1 and return as byte array
pub fn make_message_from_string(message_string: &str) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(message_string.as_bytes());
    let hashvalue = hasher.finalize();

    hashvalue.as_slice().try_into().unwrap()
}

/// Hash the message byte array with sha256 for signing by secp256k1 and return as byte array
pub fn make_message_from_bytes(message_bytes: &[u8]) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(message_bytes);
    let hashvalue = hasher.finalize();

    hashvalue.as_slice().try_into().unwrap()
}

/// Verify a message signed by secp256k1. Message is a plain string. Sig and pubkey should be base58 encoded.
pub fn verify_string_message(message: &str, sig: &str, public_key: &str) -> bool {
    let message = Message::from_slice(&make_message_from_string(message)).unwrap();
    let sig = Signature::from_der(&String::from(sig).from_base58().unwrap()).unwrap();
    let public_key =
        PublicKey::from_slice(&String::from(public_key).from_base58().unwrap()).unwrap();
    verify_message(message, sig, public_key)
}

pub fn verify_message_signature(hash: &Sha256Hash, sig: &Signature, public_key: &PublicKey) -> bool {
    let msg = Message::from_slice(hash).unwrap();
    SECP256K1.verify(&msg, sig, public_key).is_ok()
}

pub fn verify_message(msg: Message, sig: Signature, public_key: PublicKey) -> bool {
    SECP256K1.verify(&msg, &sig, &public_key).is_ok()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_make_message_from_string() {
        make_message_from_string("foobarbaz");
        make_message_from_string("1231231231");
        make_message_from_string("");
        assert!(true);
    }
}
