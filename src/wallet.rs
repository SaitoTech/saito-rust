use crate::crypto::{sign, generate_keys, SaitoPrivateKey, SaitoPublicKey, SaitoSignature};

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
#[derive(Clone, Debug)]
pub struct Wallet {
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
}

impl Wallet {

    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
	let (publickey , privatekey) = generate_keys();
        Wallet {
            publickey: publickey,
            privatekey: privatekey,
        }
    }

    pub fn get_privatekey(&self) -> SaitoPrivateKey {
        self.privatekey
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.publickey
    }

    pub fn sign(&self, message_bytes: &[u8]) -> SaitoSignature {
        sign(message_bytes, self.privatekey)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
