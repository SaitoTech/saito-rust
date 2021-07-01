use crate::crypto::{generate_keys, sign, SaitoPrivateKey, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey};


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
        let (publickey, privatekey) = generate_keys();
        Wallet {
            publickey,
            privatekey,
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


/// The `WalletSlip` stores the essential information needed to track which
/// slips are spendable and managing them as they move onto and off of the 
/// longest-chain.
pub struct WalletSlip {
    uuid: SaitoHash,
    utxokey: SaitoUTXOSetKey,
    amount: u64,
    block_id: u64,
    block_hash: SaitoHash,
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
