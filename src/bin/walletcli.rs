use saito_rust::{
    crypto::{sign_blob, SaitoPrivateKey, SaitoPublicKey},
    storage::Storage,
};
use secp256k1::{PublicKey, SecretKey, SECP256K1};

struct MockWallet {
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
}
impl MockWallet {
    pub fn new() -> MockWallet {
        let mut bytes = [0u8; 32];
        let result = hex::decode_to_slice(
            String::from("bbca91074466508f81ad202576cdbae8a475a87e7136a6027cdcb4bc2053c0be")
                .as_bytes(),
            &mut bytes as &mut [u8],
        );
        if result.is_err() {
            panic!("Couldn't decode");
        }

        let secret_key = SecretKey::from_slice(&bytes).unwrap();
        let public_key = PublicKey::from_secret_key(&SECP256K1, &secret_key);
        let mut secret_bytes = [0; 32];
        secret_bytes.clone_from_slice(&secret_key[0..32]);
        MockWallet {
            publickey: public_key.serialize(),
            privatekey: secret_bytes,
        }
    }
    pub fn get_privatekey(&self) -> SaitoPrivateKey {
        self.privatekey
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.publickey
    }
}

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    // TODO replace this with the normal wallet once the keypair is being persisted.
    // It's very tricky to use this to acheive anything useful since the keys change
    // each time it's run, so for now we just use this mock wallet with fixed keys.
    let wallet = MockWallet::new();

    if args.len() == 1 {
        panic!("Must send command and args!");
    } else {
        match args[1].as_str() {
            "print" | "p" => {
                println!(" public key : {}", hex::encode(wallet.get_publickey()));
                println!("private key : {}", hex::encode(wallet.get_privatekey()));
            }
            "sign" | "s" => {
                let mut blob = Storage::read(&args[2]).unwrap();
                let signed_blob = sign_blob(&mut blob, wallet.get_privatekey());
                Storage::write(signed_blob.clone(), &args[3]);
            }
            _ => {}
        }
    }
    Ok(())
}
