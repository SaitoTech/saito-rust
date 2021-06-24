pub use secp256k1::{Message, SecretKey, PublicKey, Signature, SECP256K1};
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use base58::ToBase58;
use hex::{decode_to_slice};
//
// forward-looking flexibility
//
pub type SaitoHash = [u8; 32];
pub type SaitoPublicKey = [u8; 33];
pub type SaitoPrivateKey = [u8; 32]; // 256-bit key
pub type SaitoSignature = [u8; 64];


//
// The Keypair is a crypto-class object that holds the secp256k1 private
// and publickey. Not all external functions hold both private and public
// keys. On the blockchain most publickeys are stored in byte-array format
// and never touch a keypair.
//
pub struct Keypair {
  privatekey: SecretKey,
  publickey: PublicKey,
}
impl Keypair {

    pub fn new() -> Keypair {
        let (mut secret_key, mut public_key) = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
        while public_key.serialize().to_base58().len() != 44 {
            // sometimes secp256k1 address is too big to store in 44 base-58 digits
            let keypair_tuple = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
            secret_key = keypair_tuple.0;
            public_key = keypair_tuple.1;
        }

        Keypair {
	    publickey: public_key,
	    privatekey: secret_key,
        }
    }

    pub fn get_privatekey(&self) -> SaitoPrivateKey {
        let mut secret_bytes = [0u8; 32];
	for i in 0..32 { secret_bytes[i] = self.privatekey[i]; }
        return secret_bytes;
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
	self.publickey.serialize()
    }

}

pub fn hash(data: &Vec<u8>) -> SaitoHash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}

pub fn sign(message_bytes: &[u8], privatekey: SaitoPrivateKey) -> SaitoSignature {
    let msg = Message::from_slice(message_bytes).unwrap();
    let secret = SecretKey::from_slice(&privatekey).unwrap();
    let sig = SECP256K1.sign(&msg, &secret);
    return sig.serialize_compact();
}

pub fn verify(msg: &[u8], sig: SaitoSignature, publickey: SaitoPublicKey) -> bool {
    let m = Message::from_slice(msg).unwrap();
    let p = PublicKey::from_slice(&publickey).unwrap();
    let s = Signature::from_compact(&sig).unwrap();
    SECP256K1.verify(&m, &s, &p).is_ok()
}



