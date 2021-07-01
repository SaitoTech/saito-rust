use crate::crypto::{generate_keys, sign, SaitoPrivateKey, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey, SaitoHash};
use crate::transaction::Transaction;
use crate::block::Block;
use crate::slip::Slip;

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
#[derive(Clone, Debug)]
pub struct Wallet {
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
    slips: Vec<WalletSlip>,
}

impl Wallet {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        let (publickey, privatekey) = generate_keys();
        Wallet {
            publickey,
            privatekey,
	    slips : vec![],
        }
    }

    pub fn add_slip(&mut self, block : Block , transaction : Transaction, slip : Slip, lc : bool) {

	let mut wallet_slip = WalletSlip::new();

	wallet_slip.set_uuid(transaction.get_hash_for_signature());
	wallet_slip.set_utxokey(slip.get_utxoset_key());
	wallet_slip.set_amount(slip.get_amount());
	wallet_slip.set_block_id(block.get_id());
	wallet_slip.set_block_hash(block.get_hash());
	wallet_slip.set_lc(lc);

        self.slips.push(WalletSlip::new());
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
#[derive(Clone, Debug)]
pub struct WalletSlip {
    uuid: SaitoHash,
    utxokey: SaitoUTXOSetKey,
    amount: u64,
    block_id: u64,
    block_hash: SaitoHash,
    lc: bool,
}
impl WalletSlip {

    pub fn new() -> Self {
        WalletSlip {
	    uuid: [0; 32],
	    utxokey: vec![],
	    amount: 0,
	    block_id: 0,
	    block_hash: [0; 32],
	    lc: true,
	}
    }

    pub fn set_uuid(&mut self, hash : SaitoHash) {
	self.uuid = hash;
    }

    pub fn set_utxokey(&mut self, utxokey : SaitoUTXOSetKey) {
	self.utxokey = utxokey;
    }

    pub fn set_amount(&mut self, amount : u64) {
	self.amount = amount;
    }

    pub fn set_block_id(&mut self, id : u64) {
	self.block_id = id;
    }

    pub fn set_block_hash(&mut self, hash : SaitoHash) {
	self.block_hash = hash;
    }

    pub fn set_lc(&mut self, lc : bool) {
	self.lc = lc;
    }

}


#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
