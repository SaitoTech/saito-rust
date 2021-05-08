use rand::rngs::OsRng;
pub use secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signature};
use std::fmt;
//use serde::{Serialize, Deserialize};
//use serde::ser::Serializer;
pub use base58::{ToBase58};
//use secp256k1::serde::Serialize;

#[derive(Debug)]
//#[derive(Serialize, Deserialize, Debug)]
pub struct Keypair {
    pub private_key: SecretKey,
    pub public_key: PublicKey,
}
impl fmt::Display for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pub: {} priv: {}", self.public_key, self.private_key)
    }
}
impl Keypair {
    pub fn new() -> Keypair {
        // secp256k1 docs are out of date and do not seem to include OsRng anymore.
        // rand 0.8.2 says to use rand_core::OsRng, but latest rand_core throws an
        // error "no `OsRng` in the root" despite that the docs say it should be there.
        // For now we are using rand 0.6.5 which includes it's own OsRng
        let secp = Secp256k1::new();
        let mut rng = OsRng::new().expect("OsRng");
        let (mut secret_key, mut public_key) = secp.generate_keypair(&mut rng);    
        while public_key.serialize().to_base58().len() != 44 {
            // sometimes secp256k1 address is too big to store in 44 base-58 digits
            let keypair_tuple = secp.generate_keypair(&mut rng);
            secret_key = keypair_tuple.0;
            public_key = keypair_tuple.1;
        }
        return Keypair {
            private_key: secret_key,
            public_key: public_key,
        };
    }
    
    pub fn new_from_private_key(private_key: &[u8]) -> Keypair {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(private_key).expect("32 bytes, within curve order");
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        return Keypair {
            private_key: secret_key,
            public_key: public_key,
        };
    }
    pub fn get_address(&self) -> String {
        return self.public_key.serialize().to_base58();
    }
    pub fn get_private_key(&self) -> String {
        return self.private_key.to_string();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    //use std::io::Write;
    use std::fmt::Write;
    #[test]
    fn test_new_from_private_key() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new_from_private_key(keypair1.private_key.as_ref());
        let mut s = String::with_capacity(32);
        let mut a = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        for byte in a.bytes() {
            write!(s, "{:02X}", byte);
        }
        let keypair = Keypair::new_from_private_key(s.as_bytes());
        println!("************************{}", keypair);
        // assert!(false);
        //e1hpHsuiRPbzXdCf7smXvAFCnqpvZXcjtxZLMxcATat1
    }
    fn test_new_get_address_get_private_key() {
        let keypair = Keypair::new();
        println!("{}", keypair);
        
        let serialized_pubkey = keypair.public_key.serialize();
        let foo = String::from_utf8_lossy(&serialized_pubkey).into_owned();
        // let s: String = serialized_pubkey.iter().cloned().collect();
        // let s = String::from(serialized_pubkey);
        //println!("****** {} ******", serialized_pubkey);
        println!("****** {} ******", foo);
        println!("****** {} ******", keypair.get_address());
        println!("****** {} ******", keypair.get_private_key());
        //println!("****** {} ******", keypair.toBase58().len());
        // println!("****** {} ******", keypair.getBase58PrivateKey());
        // println!("****** {} ******", keypair.getBase58PrivateKey().len());
        //println!("****** {} ******", foo);
        // let mut s = String::new();
        // write!(&mut s, "{:?}", serialized_pubkey); // uses fmt::Write::write_fmt
        // println!("****** {} ******", s);
        let mut st = String::new();
        write!(&mut st, "{}", keypair); // uses fmt::Write::write_fmt
        println!("****** {} ******", st);
        println!("****** {} ******", st.len());
        // let mut out = std::io::stdout();
        // out.write_all(serialized_pubkey);
        //out.flush()?;
        //keypair.public_key.len();
        assert_eq!(keypair.get_address().len(), 44);
        assert_eq!(keypair.get_private_key().len(), 64);
        println!("****** {} ******", keypair.get_private_key());
    }
}
