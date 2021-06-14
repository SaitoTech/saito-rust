use crate::crypto::make_message_from_string;
use base58::ToBase58;
use secp256k1::{Message, PublicKey, SecretKey, Signature, SECP256K1};
use std::fmt;
use std::fmt::Write;

/// An secp256k1 keypair for signing and verifying messages
#[derive(Debug, PartialEq)]
pub struct Keypair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl Keypair {
    /// Create and return a keypair with a randomly generated private key.
    pub fn new() -> Keypair {
        let (mut secret_key, mut public_key) =
            SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
        while public_key.serialize().to_base58().len() != 44 {
            // sometimes secp256k1 address is too big to store in 44 base-58 digits
            let keypair_tuple = SECP256K1.generate_keypair(&mut secp256k1::rand::thread_rng());
            secret_key = keypair_tuple.0;
            public_key = keypair_tuple.1;
        }

        Keypair {
            secret_key: secret_key,
            public_key: public_key,
        }
    }

    /// Create and return a keypair with  the given hex u8 array as the private key
    pub fn from_secret_slice(slice: &[u8]) -> Result<Keypair, secp256k1::Error> {
        let secret_key = SecretKey::from_slice(slice)?;
        let public_key = PublicKey::from_secret_key(&SECP256K1, &secret_key);

        Ok(Keypair {
            secret_key: secret_key,
            public_key: public_key,
        })
    }

    /// Create and return a keypair with  the given hex u8 array as the private key
    pub fn from_secret_hex(secret_hex: &str) -> Result<Keypair, Box<dyn std::error::Error>> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(secret_hex, &mut bytes as &mut [u8])?;
        Ok(Keypair::from_secret_slice(&bytes)?)
    }

    /// Get the public key of the keypair in base58(i.e. address) format
    pub fn address(&self) -> String {
        self.public_key.serialize().to_base58()
    }

    /// Get the public key of the keypair as secp256k1::key::PublicKey
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Get the private key as a hex-encoded string
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Hash and sign a message string
    pub fn sign_string_message(&self, message_string: &str) -> Result<String, std::fmt::Error> {
        let message_bytes = make_message_from_string(message_string);
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
        SECP256K1.sign(&msg, &self.secret_key)
    }
}

impl fmt::Display for Keypair {
    /// formats a Keypair for println!
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pubkey = self.public_key();
        write!(f, "pubkey:{} privkey:{}", pubkey, self.secret_key)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::verify_string_message;
    use hex;

    #[test]
    fn keypair_signing_test() {
        let mock_secret_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::from_secret_hex(mock_secret_key).unwrap();
        let sig_string2 = keypair
            .sign_string_message(&String::from("hello world"))
            .unwrap();
        assert_eq!(sig_string2, "3045022100e45ad15a85e320d8f3c6721b50475ec9572bca4e4831c9cfd73ce8af39fd507c02202b9f0c729cb4a0030c852e836fdfce2301eccfe9a93de3c8579fd77acadc92fd");
        let mut sig_bytes = [0u8; 71];
        assert!(hex::decode_to_slice(sig_string2, &mut sig_bytes as &mut [u8]).is_ok());
        let result = verify_string_message(
            "hello world",
            &sig_bytes.to_base58(),
            "e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1",
        );
        assert!(result);
    }

    #[test]
    fn keypair_new_from_secret_key_test() {
        let mock_secret_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::from_secret_hex(mock_secret_key).unwrap();
        let mut pubkey_string = String::new();
        assert!(write!(&mut pubkey_string, "{:?}", keypair.public_key()).is_ok());
        assert_eq!(keypair.secret_key().to_string(), mock_secret_key);
        assert_eq!(pubkey_string, "PublicKey(7280275e7c1b54f91a27a4b28291dab2b00b762a91292eb413065771fc90ee2552022d1fc27557465a8e86c147fff767b414495008b904dcdab490992add99a5)");
        assert_eq!(
            keypair.address(),
            "e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1"
        );

        assert!(Keypair::from_secret_hex("randomtext").is_err());
        assert!(Keypair::from_secret_hex("").is_err());
    }

    #[test]
    fn keypair_new_test() {
        let keypair = Keypair::new();
        assert_eq!(keypair.address().len(), 44);
        assert_eq!(keypair.secret_key().to_string().len(), 64);
    }
}
