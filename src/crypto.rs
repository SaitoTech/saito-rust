use base58::ToBase58;
use blake3::Hasher;
use ring::digest::{Algorithm, SHA256 as sha256};
pub use secp256k1::{Message, PublicKey, SecretKey, Signature, SECP256K1};
pub static SHA256: &Algorithm = &sha256;
pub use merkle::MerkleTree;

pub type SaitoHash = [u8; 32];
pub type SaitoUTXOSetKey = [u8; 74];
pub type SaitoPublicKey = [u8; 33];
pub type SaitoPrivateKey = [u8; 32]; // 256-bit key
pub type SaitoSignature = [u8; 64];

pub const PARALLEL_HASH_BYTE_THRESHOLD: usize = 128_000;

pub fn generate_keys() -> (SaitoPublicKey, SaitoPrivateKey) {
    let (mut secret_key, mut public_key) =
        SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
    while public_key.serialize().to_base58().len() != 44 {
        // sometimes secp256k1 address is too big to store in 44 base-58 digits
        let keypair_tuple = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
        secret_key = keypair_tuple.0;
        public_key = keypair_tuple.1;
    }
    let mut secret_bytes = [0u8; 32];
    for i in 0..32 {
        secret_bytes[i] = secret_key[i];
    }
    (public_key.serialize(), secret_bytes)
}
/// Create and return a keypair with  the given hex u8 array as the private key
pub fn generate_keypair_from_privatekey(slice: &[u8]) -> (SaitoPublicKey, SaitoPrivateKey) {
    let secret_key = SecretKey::from_slice(slice).unwrap();
    let public_key = PublicKey::from_secret_key(&SECP256K1, &secret_key);
    let mut secret_bytes = [0u8; 32];
    for i in 0..32 {
        secret_bytes[i] = secret_key[i];
    }
    (public_key.serialize(), secret_bytes)
}
pub fn sign_blob(vbytes: &mut Vec<u8>, privatekey: SaitoPrivateKey) -> &mut Vec<u8> {
    let sig = sign(&hash(vbytes.as_ref()), privatekey);
    vbytes.extend(&sig);
    vbytes
}

pub fn generate_random_bytes(len: u64) -> Vec<u8> {
    if len == 0 {
        let x: Vec<u8> = vec![];
        return x;
    }
    (0..len).map(|_| rand::random::<u8>()).collect()
}

pub fn hash(data: &Vec<u8>) -> SaitoHash {
    let mut hasher = Hasher::new();
    // Hashing in parallel can be faster if large enough
    // TODO: Blake3 has benchmarked 128 kb as the cutoff,
    // the benchmark should be redone for Saito's needs
    if data.len() > PARALLEL_HASH_BYTE_THRESHOLD {
        hasher.update(data);
    } else {
        hasher.update_rayon(data);
    }
    hasher.finalize().into()
}

pub fn sign(message_bytes: &[u8], privatekey: SaitoPrivateKey) -> SaitoSignature {
    let msg = Message::from_slice(message_bytes).unwrap();
    let secret = SecretKey::from_slice(&privatekey).unwrap();
    let sig = SECP256K1.sign(&msg, &secret);
    sig.serialize_compact()
}

pub fn verify(msg: &[u8], sig: SaitoSignature, publickey: SaitoPublicKey) -> bool {
    let m = Message::from_slice(msg).unwrap();
    let p = PublicKey::from_slice(&publickey).unwrap();
    let s = Signature::from_compact(&sig).unwrap();
    SECP256K1.verify(&m, &s, &p).is_ok()
}
