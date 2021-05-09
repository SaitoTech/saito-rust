use rand::rngs::OsRng;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signature, All};
use sha2::{Sha256, Digest};
use std::fmt;
use base58::{ToBase58};

// We don't want to create a new secp256k1 object for every call to a method in keypair, instead
// we create a single static using lazy_static that all keypairs can reference when needed.
// TODO: Can this be replaced with this? https://docs.rs/secp256k1-plus/0.5.7/secp256k1/struct.SECP256K1.html
lazy_static! {
    static ref SECP: Secp256k1<All> = {
        Secp256k1::new()
    };
}

/// An secp256k1 keypair for signing and verifying messages
#[derive(Debug)]
pub struct Keypair {
    pub private_key: SecretKey,
}

impl Keypair {
    /// Create and return a keypair with a randomly generated private key.
    pub fn new() -> Keypair {
        // secp256k1 docs are out of date and do not seem to include OsRng anymore.
        // rand 0.8.2 says to use rand_core::OsRng, but latest rand_core throws an
        // error "no `OsRng` in the root" despite that the docs say it should be there.
        // For now we are using rand 0.6.5 which includes it's own OsRng
        //let secp = Secp256k1::new();
        let mut rng = OsRng::new().expect("OsRng");
        let (mut secret_key, mut public_key) = SECP.generate_keypair(&mut rng);    
        while public_key.serialize().to_base58().len() != 44 {
            // sometimes secp256k1 address is too big to store in 44 base-58 digits
            let keypair_tuple = SECP.generate_keypair(&mut rng);
            secret_key = keypair_tuple.0;
            public_key = keypair_tuple.1;
        }
        return Keypair {
            private_key: secret_key,
        };
    }
    /// Create and return a keypair with  the given hex u8 array as the private key
    pub fn new_from_private_key(private_key: &[u8]) -> Keypair {
        let secret_key = SecretKey::from_slice(private_key).expect("32 bytes, within curve order");
        
        return Keypair {
            private_key: secret_key,
        };
    }
    /// Create and return a keypair with  the given hex u8 array as the private key
    pub fn new_from_private_key_hex(private_key_hex: &str) -> Keypair {
        //let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(private_key_hex, &mut bytes as &mut [u8]);
        return Keypair::new_from_private_key(&bytes);
        
        // let secret_key = SecretKey::from_slice(private_key).expect("32 bytes, within curve order");
        // 
        // return Keypair {
        //     private_key: secret_key,
        // };
    }
    /// Get the public key of the keypair in base58(i.e. address) format
    pub fn get_address(&self) -> String {
        return PublicKey::from_secret_key(&SECP, &self.private_key).serialize().to_base58();
    }
    /// Get the public key of the keypair as secp256k1::key::PublicKey
    pub fn get_public_key(&self) -> PublicKey {
        return PublicKey::from_secret_key(&SECP, &self.private_key);
    }
    /// Get the private key as a hex-encoded string
    pub fn get_private_key(&self) -> String {
        return self.private_key.to_string();
    }
    /// Sign a byte message, must be length 32
    pub fn sign_message(&self, message_bytes: &[u8]) -> Signature {
        let msg = Message::from_slice(message_bytes).unwrap();
        return SECP.sign(&msg, &self.private_key);
        // -> Result<Signature, Error>
    }
    /// Hash and sign a string
    pub fn sign_string_message(&self, message_string: &str) -> Signature {
        let mut hasher = Sha256::new();
        hasher.update(message_string.as_bytes());
        let message_bytes = hasher.finalize();
        return self.sign_message(&message_bytes);
    }
}

impl fmt::Display for Keypair {
    /// formats a Keypair for println!
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pubkey = self.get_public_key();
        write!(f, "pubkey:{} privkey:{}", pubkey, self.private_key)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Write;
    use hex;
    #[test]
    fn test_signing() {
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::new_from_private_key_hex(mock_private_key);
        let mock_message: [u8;32] = [0;32];
        let mut sig_string = String::new();
        write!(&mut sig_string, "{:?}", keypair.sign_message(&[0;32]));
        assert_eq!(sig_string, "3045022100ec45d9852bab78d1b5492158a2ff30801e1c2133a9e2610f6e95df7ea7a87f5d02200cbf2d5c97e6118ffe5baac96ae8f5a5d23bd5fb072e3122846d4172b31dbd1b");
        let mut sig_string2 = String::new();
        write!(&mut sig_string2, "{:?}", keypair.sign_string_message(&String::from("hello world")));
        assert_eq!(sig_string2, "3045022100e45ad15a85e320d8f3c6721b50475ec9572bca4e4831c9cfd73ce8af39fd507c02202b9f0c729cb4a0030c852e836fdfce2301eccfe9a93de3c8579fd77acadc92fd");
    }
    #[test]
    fn test_new_from_private_key() {
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::new_from_private_key_hex(mock_private_key);
        let mut pubkey_string = String::new();
        write!(&mut pubkey_string, "{:?}", keypair.get_public_key());
        assert_eq!(keypair.get_private_key(), mock_private_key);
        assert_eq!(pubkey_string, "PublicKey(7280275e7c1b54f91a27a4b28291dab2b00b762a91292eb413065771fc90ee2552022d1fc27557465a8e86c147fff767b414495008b904dcdab490992add99a5)");
        assert_eq!(keypair.get_address(), "e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1");
    }
    #[test]
    fn test_new() {
        let keypair = Keypair::new();
        assert_eq!(keypair.get_address().len(), 44);
        assert_eq!(keypair.get_private_key().len(), 64);
    }
}
