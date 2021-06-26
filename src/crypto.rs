use base58::ToBase58;
use blake3::{join::RayonJoin, Hasher};
pub use secp256k1::{Message, PublicKey, SecretKey, Signature, SECP256K1};

pub type SaitoHash = [u8; 32];
pub type SaitoPublicKey = [u8; 33];
pub type SaitoPrivateKey = [u8; 32]; // 256-bit key
pub type SaitoSignature = [u8; 64];

pub const PARALLEL_HASH_BYTE_THRESHOLD: usize = 128_000;


pub fn generate_keys() -> (SaitoPublicKey, SaitoPrivateKey) {
    let (mut secret_key, mut public_key) = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
    while public_key.serialize().to_base58().len() != 44 {
        // sometimes secp256k1 address is too big to store in 44 base-58 digits
        let keypair_tuple = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
        secret_key = keypair_tuple.0;
        public_key = keypair_tuple.1;
    }
    let mut secret_bytes = [0u8; 32];
    for i in 0..32 { secret_bytes[i] = secret_key[i]; }
    return (public_key.serialize() , secret_bytes);
}

pub fn hash(data: &Vec<u8>) -> SaitoHash {
    let mut hasher = Hasher::new();
    // Hashing in parallel can be faster if large enough
    // TODO: Blake3 has benchmarked 128 kb as the cutoff,
    // the benchmark should be redone for Saito's needs
    if data.len() > PARALLEL_HASH_BYTE_THRESHOLD {
        hasher.update(data);
    } else {
        hasher.update_with_join::<RayonJoin>(data);
    }
    hasher.finalize().into()
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

