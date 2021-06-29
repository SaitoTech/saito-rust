use crate::{
    crypto::{
        generate_keys, sign, SaitoPrivateKey, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey,
    },
    slip::Slip,
};
use std::collections::HashMap;

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
#[derive(Clone, Debug)]
pub struct Wallet {
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
    slips: HashMap<SaitoUTXOSetKey, Slip>,
}

impl Wallet {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        let (publickey, privatekey) = generate_keys();
        Wallet {
            publickey,
            privatekey,
            slips: HashMap::new(),
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

    pub fn add_slip(&mut self, slip: Slip) {
        self.slips.insert(slip.get_utxoset_key(), slip);
    }

    pub fn remove_slip(&mut self, slip: &Slip) {
        self.slips.remove(&slip.get_utxoset_key());
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
