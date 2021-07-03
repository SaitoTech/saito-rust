use crate::block::Block;
use crate::crypto::{
    generate_keys, sign, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature,
    SaitoUTXOSetKey,
};
use crate::golden_ticket::GoldenTicket;
use crate::slip::Slip;
use crate::transaction::{Transaction, TransactionType};

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
            slips: vec![],
        }
    }

    pub fn add_block(&mut self, block: &Block) {
        for tx in &block.transactions {
            for input in tx.get_inputs() {
                self.delete_slip(input);
            }
            for output in tx.get_outputs() {
                self.add_slip(block, tx, output, block.get_lc());
            }
        }
    }

    pub fn add_slip(&mut self, block: &Block, transaction: &Transaction, slip: &Slip, lc: bool) {
        let mut wallet_slip = WalletSlip::new();

        wallet_slip.set_uuid(transaction.get_hash_for_signature());
        wallet_slip.set_utxokey(slip.get_utxoset_key());
        wallet_slip.set_amount(slip.get_amount());
        wallet_slip.set_block_id(block.get_id());
        wallet_slip.set_block_hash(block.get_hash());
        wallet_slip.set_lc(lc);

        self.slips.push(WalletSlip::new());
    }

    pub fn delete_slip(&mut self, slip: &Slip) {
        self.slips.retain(|x| {
            x.get_uuid() != slip.get_uuid() && x.get_slip_ordinal() != slip.get_slip_ordinal()
        });
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



    pub async fn create_golden_ticket_transaction(&mut self, golden_ticket : GoldenTicket) -> Transaction {

        let mut transaction = Transaction::default();

        // for now we'll use bincode to de/serialize
        transaction.set_transaction_type(TransactionType::GoldenTicket);
        transaction.set_message(bincode::serialize(&golden_ticket).unwrap());

        let mut input1 = Slip::default();
        input1.set_publickey(self.get_publickey());
        input1.set_amount(0);
        input1.set_uuid([0; 32]);

        let mut output1 = Slip::default();
        output1.set_publickey(self.get_publickey());
        output1.set_amount(0);
        output1.set_uuid([0; 32]);

        transaction.add_input(input1);
        transaction.add_output(output1);
        transaction.sign(self.get_privatekey());

        transaction
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
    slip_ordinal: u8,
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
            slip_ordinal: 0,
        }
    }

    pub fn get_uuid(&self) -> SaitoHash {
        self.uuid
    }

    pub fn get_utxokey(&self) -> &SaitoUTXOSetKey {
        &self.utxokey
    }

    pub fn get_amount(&self) -> u64 {
        self.amount
    }

    pub fn get_block_id(&self) -> u64 {
        self.block_id
    }

    pub fn get_block_hash(&self) -> SaitoHash {
        self.block_hash
    }

    pub fn get_lc(&self) -> bool {
        self.lc
    }

    pub fn get_slip_ordinal(&self) -> u8 {
        self.slip_ordinal
    }

    pub fn set_uuid(&mut self, hash: SaitoHash) {
        self.uuid = hash;
    }

    pub fn set_utxokey(&mut self, utxokey: SaitoUTXOSetKey) {
        self.utxokey = utxokey;
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    pub fn set_block_id(&mut self, id: u64) {
        self.block_id = id;
    }

    pub fn set_block_hash(&mut self, hash: SaitoHash) {
        self.block_hash = hash;
    }

    pub fn set_lc(&mut self, lc: bool) {
        self.lc = lc;
    }

    pub fn set_slip_ordinal(&mut self, slip_ordinal: u8) {
        self.slip_ordinal = slip_ordinal;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
