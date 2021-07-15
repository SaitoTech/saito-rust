use crate::crypto::{sign, SaitoHash, SaitoPublicKey, SaitoSignature};
use crate::wallet::Wallet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::convert::{TryFrom, TryInto};

//
// TODO - we can reduce the size by eliminating the FROM record if
// the portions of the code that check the routing work start with
// the sender of the transaction. This would be a good optimization
//
pub const HOP_SIZE: usize = 130;

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


    pub fn deserialize_from_net(bytes: Vec<u8>) -> Hop {

        let from: SaitoPublicKey = bytes[..33].try_into().unwrap();
        let to: SaitoPublicKey = bytes[33..66].try_into().unwrap();
        let sig: SaitoSignature = bytes[66..130].try_into().unwrap();

println!("from: {:?}", from);
println!("to: {:?}", to);

	let mut hop = Hop::new();
        hop.set_from(from);
        hop.set_to(to);
        hop.set_sig(sig);

        hop
    }

    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.get_from());
        vbytes.extend(&self.get_to());
        vbytes.extend(&self.get_sig());
        vbytes
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
