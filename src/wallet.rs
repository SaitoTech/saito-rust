use crate::crypto::{Keypair, SaitoPublicKey, SaitoSignature};

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
#[derive(Clone, Debug)]
pub struct Wallet {
    keypair: Keypair,
}

impl Wallet {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Wallet {
            keypair: Keypair::new(),
        }
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.keypair.get_publickey()
    }

    pub fn sign(&self, message_bytes: &[u8]) -> SaitoSignature {
        self.keypair.sign(message_bytes)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
