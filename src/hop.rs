use crate::crypto::{sign, SaitoHash, SaitoPublicKey, SaitoSignature};
use crate::wallet::Wallet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Hop {
    #[serde_as(as = "[_; 33]")]
    from: SaitoPublicKey,
    #[serde_as(as = "[_; 33]")]
    to: SaitoPublicKey,
    #[serde_as(as = "[_; 64]")]
    sig: SaitoSignature,
}

impl Hop {
    pub fn new() -> Self {
        Hop {
            from: [0; 33],
            to: [0; 33],
            sig: [0; 64],
        }
    }

    pub async fn generate_hop(
        wallet_lock: Arc<RwLock<Wallet>>,
        to_publickey: SaitoPublicKey,
        hash_to_sign: SaitoHash,
    ) -> Hop {
        let wallet = wallet_lock.read().await;
        let mut hop = Hop::new();

        hop.set_from(wallet.get_publickey());
        hop.set_to(to_publickey);
        hop.set_sig(sign(&hash_to_sign, wallet.get_privatekey()));

        hop
    }

    pub fn get_from(&self) -> SaitoPublicKey {
        self.from
    }

    pub fn get_to(&self) -> SaitoPublicKey {
        self.to
    }

    pub fn get_sig(&self) -> SaitoSignature {
        self.sig
    }

    pub fn set_from(&mut self, from: SaitoPublicKey) {
        self.from = from
    }

    pub fn set_to(&mut self, to: SaitoPublicKey) {
        self.to = to
    }

    pub fn set_sig(&mut self, sig: SaitoSignature) {
        self.sig = sig
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn hop_new_test() {
        let hop = Hop::new();
        assert_eq!(1, 1);
    }
}
