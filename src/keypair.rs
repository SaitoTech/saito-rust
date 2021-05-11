use base58::{FromBase58, ToBase58};
use rand::rngs::OsRng;
use secp256k1::{All, Message, PublicKey, Secp256k1, SecretKey, Signature};
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::fmt;
use std::fmt::Write;

// We don't want to create a new secp256k1 object for every call to a method in keypair, instead
// we create a single static using lazy_static that all keypairs can reference when needed.
// TODO: Can this be replaced with this? https://docs.rs/secp256k1-plus/0.5.7/secp256k1/struct.SECP256K1.html
lazy_static! {
    static ref SECP: Secp256k1<All> = Secp256k1::new();
}

/// An secp256k1 keypair for signing and verifying messages
#[derive(Debug, PartialEq)]
pub struct Keypair {
    pub private_key: SecretKey,
}

impl Keypair {
    /// Create and return a keypair with a randomly generated private key.
    pub fn new() -> Result<Keypair, rand::Error> {
        // secp256k1 docs are out of date and do not seem to include OsRng anymore.
        // rand 0.8.2 says to use rand_core::OsRng, but latest rand_core throws an
        // error "no `OsRng` in the root" despite that the docs say it should be there.
        // For now we are using rand 0.6.5 which includes it's own OsRng
        let mut rng = OsRng::new()?;
        let (mut secret_key, mut public_key) = SECP.generate_keypair(&mut rng);
        while public_key.serialize().to_base58().len() != 44 {
            // sometimes secp256k1 address is too big to store in 44 base-58 digits
            let keypair_tuple = SECP.generate_keypair(&mut rng);
            secret_key = keypair_tuple.0;
            public_key = keypair_tuple.1;
        }

        Ok(Keypair {
            private_key: secret_key,
        })
    }

    /// Create and return a keypair with  the given hex u8 array as the private key
    fn new_from_private_key(private_key: &[u8]) -> Result<Keypair, secp256k1::Error> {
        let secret_key = SecretKey::from_slice(private_key)?;

        Ok(Keypair {
            private_key: secret_key,
        })
    }

    /// Create and return a keypair with  the given hex u8 array as the private key
    pub fn new_from_private_key_hex(
        private_key_hex: &str,
    ) -> Result<Keypair, Box<dyn std::error::Error>> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(private_key_hex, &mut bytes as &mut [u8])?;
        Ok(Keypair::new_from_private_key(&bytes)?)
    }

    /// Get the public key of the keypair in base58(i.e. address) format
    pub fn get_address(&self) -> String {
        PublicKey::from_secret_key(&SECP, &self.private_key)
            .serialize()
            .to_base58()
    }

    /// Get the public key of the keypair as secp256k1::key::PublicKey
    pub fn get_public_key(&self) -> PublicKey {
        PublicKey::from_secret_key(&SECP, &self.private_key)
    }

    /// Get the private key as a hex-encoded string
    pub fn get_private_key(&self) -> String {
        self.private_key.to_string()
    }

    /// Hash the message string with sha256 for signing by secp256k1 and return as byte array
    /// TODO: Make sure this handles utf correctly. We probably want to ensure that the message
    /// is actually just ascii encoded...
    pub fn make_message_from_string(message_string: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(message_string.as_bytes());
        let hashvalue = hasher.finalize();

        hashvalue.as_slice().try_into().unwrap()
    }

    /// Hash and sign a message string
    pub fn sign_string_message(&self, message_string: &str) -> Result<String, std::fmt::Error> {
        let message_bytes = Keypair::make_message_from_string(message_string);
        let bytes = self.sign_message(&message_bytes);
        let mut string_out = String::new();

        if let Err(e) = write!(&mut string_out, "{:?}", bytes) {
            return Err(e);
        }

        Ok(string_out)
    }

    /// Hash and sign message bytes
    pub fn sign_message(&self, message_bytes: &[u8]) -> Signature {
        let msg = Message::from_slice(message_bytes).unwrap();
        SECP.sign(&msg, &self.private_key)
    }

    /// Verify a message signed by secp256k1. Message is a plain string. Sig and pubkey should be base58 encoded.
    pub fn verify_string_message(message: &str, sig: &str, public_key: &str) -> bool {
        let message = Message::from_slice(&Keypair::make_message_from_string(message)).unwrap();
        let sig = Signature::from_der(&String::from(sig).from_base58().unwrap()).unwrap();
        let public_key =
            PublicKey::from_slice(&String::from(public_key).from_base58().unwrap()).unwrap();
        Keypair::verify_message(message, sig, public_key)
    }
    fn verify_message(msg: Message, sig: Signature, public_key: PublicKey) -> bool {
        SECP.verify(&msg, &sig, &public_key).is_ok()
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
    use hex;

    #[test]
    fn test_make_message_from_string() {
        Keypair::make_message_from_string("foobarbaz");
        Keypair::make_message_from_string("1231231231");
        Keypair::make_message_from_string("");
    }

    #[test]
    fn test_signing() {
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::new_from_private_key_hex(mock_private_key).unwrap();
        let sig_string2 = keypair
            .sign_string_message(&String::from("hello world"))
            .unwrap();
        assert_eq!(sig_string2, "3045022100e45ad15a85e320d8f3c6721b50475ec9572bca4e4831c9cfd73ce8af39fd507c02202b9f0c729cb4a0030c852e836fdfce2301eccfe9a93de3c8579fd77acadc92fd");
        let mut sig_bytes = [0u8; 71];
        assert!(hex::decode_to_slice(sig_string2, &mut sig_bytes as &mut [u8]).is_ok());
        let result = Keypair::verify_string_message(
            "hello world",
            &sig_bytes.to_base58(),
            "e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1",
        );
        assert!(result);
    }

    #[test]
    fn test_new_from_private_key() {
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::new_from_private_key_hex(mock_private_key).unwrap();
        let mut pubkey_string = String::new();
        assert!(write!(&mut pubkey_string, "{:?}", keypair.get_public_key()).is_ok());
        assert_eq!(keypair.get_private_key(), mock_private_key);
        assert_eq!(pubkey_string, "PublicKey(7280275e7c1b54f91a27a4b28291dab2b00b762a91292eb413065771fc90ee2552022d1fc27557465a8e86c147fff767b414495008b904dcdab490992add99a5)");
        assert_eq!(
            keypair.get_address(),
            "e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1"
        );

        assert!(Keypair::new_from_private_key_hex("randomtext").is_err());
        assert!(Keypair::new_from_private_key_hex("").is_err());
    }

    #[test]
    fn test_new() {
        let keypair = Keypair::new().unwrap();
        assert_eq!(keypair.get_address().len(), 44);
        assert_eq!(keypair.get_private_key().len(), 64);
    }
}
