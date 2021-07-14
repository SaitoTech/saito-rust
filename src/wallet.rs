use crate::block::Block;
use crate::crypto::{
    generate_keys, hash, sign, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature,
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
                if output.get_amount() > 0 {
                    self.add_slip(block, tx, output, block.get_lc());
                }
            }
        }
    }

    pub fn add_slip(&mut self, block: &Block, transaction: &Transaction, slip: &Slip, lc: bool) {
        let mut wallet_slip = WalletSlip::new();

        wallet_slip.set_uuid(transaction.get_hash_for_signature());
        wallet_slip.set_utxokey(slip.get_utxoset_key());
        wallet_slip.set_amount(slip.get_amount());
        wallet_slip.set_slip_ordinal(slip.get_slip_ordinal());
        wallet_slip.set_block_id(block.get_id());
        wallet_slip.set_block_hash(block.get_hash());
        wallet_slip.set_lc(lc);

        self.slips.push(wallet_slip);
    }

    pub fn delete_slip(&mut self, slip: &Slip) {
        self.slips.retain(|x| {
            x.get_uuid() != slip.get_uuid() || x.get_slip_ordinal() != slip.get_slip_ordinal()
        });
    }

    pub fn get_privatekey(&self) -> SaitoPrivateKey {
        self.privatekey
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.publickey
    }

    pub fn get_available_balance(&self) -> u64 {
        let mut available_balance: u64 = 0;
        for slip in &self.slips {
            if !slip.get_spent() {
                available_balance += slip.get_amount();
            }
        }
        return available_balance;
    }

    pub fn generate_slips(&mut self, nolan_requested: u64) -> (Vec<Slip>, Vec<Slip>) {
        let mut inputs: Vec<Slip> = vec![];
        let mut outputs: Vec<Slip> = vec![];
        let mut nolan_in: u64 = 0;
        let mut nolan_out: u64 = 0;
        let my_publickey = self.get_publickey();

        //
        // grab inputs
        //
        for slip in &mut self.slips {
            if !slip.get_spent() {
                if nolan_in < nolan_requested {
                    nolan_in += slip.get_amount();

                    let mut input = Slip::new();
                    input.set_publickey(my_publickey);
                    input.set_amount(slip.get_amount());
                    input.set_uuid(slip.get_uuid());
                    inputs.push(input);

                    slip.set_spent(true);
                }
            }
        }

        //
        // create outputs
        //
        if nolan_in > nolan_requested {
            nolan_out = nolan_in - nolan_requested;
        }

        // change address
        let mut output = Slip::new();
        output.set_publickey(my_publickey);
        output.set_amount(nolan_out);
        outputs.push(output);

        //
        // ensure not empty
        //
        if inputs.len() == 0 {
            let mut input = Slip::new();
            input.set_publickey(my_publickey);
            input.set_amount(0);
            input.set_uuid([0; 32]);
            inputs.push(input);
        }
        if outputs.len() == 0 {
            let mut output = Slip::new();
            output.set_publickey(my_publickey);
            output.set_amount(0);
            output.set_uuid([0; 32]);
            outputs.push(output);
        }

        return (inputs, outputs);
    }

    pub fn sign(&self, message_bytes: &[u8]) -> SaitoSignature {
        sign(message_bytes, self.privatekey)
    }

    pub async fn create_golden_ticket_transaction(
        &mut self,
        golden_ticket: GoldenTicket,
    ) -> Transaction {
        let mut transaction = Transaction::new();

        // for now we'll use bincode to de/serialize
        transaction.set_transaction_type(TransactionType::GoldenTicket);
        transaction.set_message(golden_ticket.serialize_for_transaction());

        let mut input1 = Slip::new();
        input1.set_publickey(self.get_publickey());
        input1.set_amount(0);
        input1.set_uuid([0; 32]);

        let mut output1 = Slip::new();
        output1.set_publickey(self.get_publickey());
        output1.set_amount(0);
        output1.set_uuid([0; 32]);

        transaction.add_input(input1);
        transaction.add_output(output1);

        let hash_for_signature: SaitoHash = hash(&transaction.serialize_for_signature());
        transaction.set_hash_for_signature(hash_for_signature);

        transaction.sign(self.get_privatekey());

        transaction
    }
}

/// The `WalletSlip` stores the essential information needed to track which
/// slips are spendable and managing them as they move onto and off of the
/// longest-chain.
///
/// Please note that the wallet in this Saito Rust client is intended primarily
/// to hold the public/privatekey and that slip-spending and tracking code is
/// not coded in a way intended to be robust against chain-reorganizations but
/// rather for testing of basic functions like transaction creation. Slips that
/// are spent on one fork are not recaptured on chains, for instance, and once
/// a slip is spent it is marked as spent.
///
#[derive(Clone, Debug)]
pub struct WalletSlip {
    uuid: SaitoHash,
    utxokey: SaitoUTXOSetKey,
    amount: u64,
    block_id: u64,
    block_hash: SaitoHash,
    lc: bool,
    slip_ordinal: u8,
    spent: bool,
}

impl WalletSlip {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        WalletSlip {
            uuid: [0; 32],
            utxokey: [0; 74],
            amount: 0,
            block_id: 0,
            block_hash: [0; 32],
            lc: true,
            slip_ordinal: 0,
            spent: false,
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

    pub fn get_spent(&self) -> bool {
        self.spent
    }

    pub fn set_spent(&mut self, spent: bool) {
        self.spent = spent;
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
