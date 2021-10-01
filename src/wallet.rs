use crate::storage::{Persistable, Storage};
use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use macros::Persistable;
use std::path::Path;

use crate::block::Block;
use crate::crypto::{
    generate_keypair_from_privatekey, generate_keys, hash, sign, SaitoHash, SaitoPrivateKey,
    SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey,
};
use crate::golden_ticket::GoldenTicket;
use crate::slip::{Slip, SlipType};
use crate::staking::Staking;
use crate::transaction::{Transaction, TransactionType};
use serde::{Deserialize, Serialize};

// create an alias for convenience
type Aes128Cbc = Cbc<Aes128, Pkcs7>;

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
#[derive(Clone, Debug)]
pub struct Wallet {
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
    slips: Vec<WalletSlip>,
    staked_slips: Vec<WalletSlip>,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Persistable, Default)]
#[persist_with_name(get_file_name)]
pub struct EncryptedWallet {
    encrypted_privatekey: Vec<u8>,
    #[serde(skip)]
    file_path: String,
}
impl EncryptedWallet {
    pub fn get_file_name(&self) -> String {
        self.file_path.clone()
    }
}
impl Wallet {
    pub fn new() -> Wallet {
        let (publickey, privatekey) = generate_keys();
        Wallet {
            publickey,
            privatekey,
            slips: vec![],
            staked_slips: vec![],
        }
    }

    pub fn load_keys(&mut self, key_path: &str, password: Option<&str>) {
        let mut filename = String::from("data/");
        filename.push_str(key_path);
        let path = Path::new(&filename);

        let decrypted_buffer: Vec<u8>;
        if !path.exists() {
            println!("Creating key file at path: {}", key_path);
            decrypted_buffer = Wallet::create_key_file(&key_path, &password);
        } else {
            println!("Reading key file from path: {}", key_path);
            decrypted_buffer = Wallet::read_key_file(&key_path, &password);
        }

        let (publickey, privatekey) = generate_keypair_from_privatekey(&decrypted_buffer);

        self.set_publickey(publickey);
        self.set_privatekey(privatekey);
    }

    fn create_key_file(key_file_path: &str, opts_password: &Option<&str>) -> Vec<u8> {
        let password: String;
        if opts_password.is_none() {
            password = rpassword::prompt_password_stdout("Password: ").unwrap();
        } else {
            password = String::from(opts_password.as_deref().unwrap());
        }

        let (key, iv) = Wallet::create_primitives_from_password(&password);
        let cipher = Aes128Cbc::new_from_slices(&key, &iv).unwrap();
        let (_publickey, privatekey) = generate_keys();

        let ciphertext = cipher.encrypt_vec(&privatekey);
        let encrypted_wallet = EncryptedWallet {
            encrypted_privatekey: ciphertext,
            file_path: key_file_path.to_string().clone(),
        };
        encrypted_wallet.save();

        privatekey.to_vec()
    }

    fn read_key_file(key_file_path: &str, opts_password: &Option<&str>) -> Vec<u8> {
        let password: String;
        if opts_password.is_none() {
            password = rpassword::prompt_password_stdout("Password: ").unwrap();
        } else {
            password = String::from(opts_password.as_deref().unwrap());
        }

        let encrypted_wallet = EncryptedWallet::load(&key_file_path);

        Wallet::decrypt_key_file(encrypted_wallet.encrypted_privatekey, &password)
    }

    fn decrypt_key_file(ciphertext: Vec<u8>, password: &str) -> Vec<u8> {
        let (key, iv) = Wallet::create_primitives_from_password(password);
        let cipher = Aes128Cbc::new_from_slices(&key, &iv).unwrap();
        cipher.decrypt_vec(&ciphertext).unwrap()
    }

    fn create_primitives_from_password(password: &str) -> ([u8; 16], [u8; 16]) {
        let hash = hash(&password.as_bytes().to_vec());
        let mut key: [u8; 16] = [0; 16];
        let mut iv: [u8; 16] = [0; 16];
        key.clone_from_slice(&hash[0..16]);
        iv.clone_from_slice(&hash[16..32]);
        (key, iv)
    }

    pub fn on_chain_reorganization(&mut self, block: &Block, lc: bool) {
        if lc {
            println!("wallet on-chain-reorg");

            for tx in block.get_transactions() {
                for input in tx.get_inputs() {
                    if input.get_amount() > 0 && input.get_publickey() == self.get_publickey() {
                        println!("this type of tx: {:?}", input.get_slip_type());
                        if input.get_slip_type() == SlipType::StakerDeposit
                            || input.get_slip_type() == SlipType::StakerOutput
                            || input.get_slip_type() == SlipType::StakerWithdrawalStaking
                            || input.get_slip_type() == SlipType::StakerWithdrawalPending
                        {
                            println!(
                                "REMOVING STAKER INPUT FROM WALLET: {:?}",
                                input.get_utxoset_key()
                            );
                            self.delete_staked_slip(input);
                        } else {
                            println!("REMOVING non-staking INPUT");
                            self.delete_slip(input);
                        }
                    }
                }
                for output in tx.get_outputs() {
                    if output.get_amount() > 0 && output.get_publickey() == self.get_publickey() {
                        println!(
                            "ADDING OUTPUT: {:?} / {}",
                            output.get_utxoset_key(),
                            output.get_amount()
                        );
                        self.add_slip(block, tx, output, true);
                    }
                }
            }
        } else {
            for tx in block.get_transactions() {
                for input in tx.get_inputs() {
                    if input.get_amount() > 0 && input.get_publickey() == self.get_publickey() {
                        self.add_slip(block, tx, input, true);
                    }
                }
                for output in tx.get_outputs() {
                    if output.get_amount() > 0 && output.get_publickey() == self.get_publickey() {
                        self.delete_slip(output);
                    }
                }
            }
        }
    }

    //
    // removes all slips in block when pruned / deleted
    //
    pub fn delete_block(&mut self, block: &Block) {
        for tx in block.get_transactions() {
            for input in tx.get_inputs() {
                self.delete_slip(input);
            }
            for output in tx.get_outputs() {
                if output.get_amount() > 0 {
                    self.delete_slip(output);
                }
            }
        }
    }

    pub fn add_slip(&mut self, block: &Block, transaction: &Transaction, slip: &Slip, lc: bool) {
        let mut wallet_slip = WalletSlip::new();

        wallet_slip.set_uuid(transaction.get_hash_for_signature().unwrap());
        wallet_slip.set_utxokey(slip.get_utxoset_key());
        wallet_slip.set_amount(slip.get_amount());
        wallet_slip.set_slip_ordinal(slip.get_slip_ordinal());
        wallet_slip.set_block_id(block.get_id());
        wallet_slip.set_block_hash(block.get_hash());
        wallet_slip.set_lc(lc);

        if slip.get_slip_type() == SlipType::StakerDeposit
            || slip.get_slip_type() == SlipType::StakerOutput
        {
            self.staked_slips.push(wallet_slip);
        } else {
            self.slips.push(wallet_slip);
        }
    }

    pub fn delete_staked_slip(&mut self, slip: &Slip) {
        self.staked_slips.retain(|x| {
            x.get_uuid() != slip.get_uuid() || x.get_slip_ordinal() != slip.get_slip_ordinal()
        });
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

    pub fn set_privatekey(&mut self, privatekey: SaitoPrivateKey) {
        self.privatekey = privatekey;
    }

    pub fn set_publickey(&mut self, publickey: SaitoPublicKey) {
        self.publickey = publickey;
    }

    pub fn get_available_balance(&self) -> u64 {
        let mut available_balance: u64 = 0;
        for slip in &self.slips {
            if !slip.get_spent() {
                available_balance += slip.get_amount();
            }
        }
        available_balance
    }

    // the nolan_requested is omitted from the slips created - only the change
    // address is provided as an output. so make sure that any function calling
    // this manually creates the output for its desired payment
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
                    input.set_slip_ordinal(slip.get_slip_ordinal());
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

        //
        // add change address
        //
        let mut output = Slip::new();
        output.set_publickey(my_publickey);
        output.set_amount(nolan_out);
        outputs.push(output);

        //
        // ensure not empty
        //
        if inputs.is_empty() {
            let mut input = Slip::new();
            input.set_publickey(my_publickey);
            input.set_amount(0);
            input.set_uuid([0; 32]);
            inputs.push(input);
        }
        if outputs.is_empty() {
            let mut output = Slip::new();
            output.set_publickey(my_publickey);
            output.set_amount(0);
            output.set_uuid([0; 32]);
            outputs.push(output);
        }

        (inputs, outputs)
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

    //
    // creates a transaction that will deposit tokens into the staking system in the
    // amount specified, if possible. the transaction will be invalid if there is not
    // enough UTXO in the wallet to make the payment.
    //
    pub async fn create_staking_deposit_transaction(
        &mut self,
        total_requested: u64,
    ) -> Transaction {
        let mut transaction = Transaction::new();

        transaction.set_transaction_type(TransactionType::StakerDeposit);

        let (mut input_slips, mut output_slips) = self.generate_slips(total_requested);
        let input_len = input_slips.len();
        let output_len = output_slips.len();

        // add the staking deposit
        let mut output = Slip::new();
        output.set_publickey(self.get_publickey());
        output.set_amount(total_requested);
        output.set_slip_type(SlipType::StakerDeposit);
        transaction.add_output(output);

        for _i in 0..input_len {
            transaction.add_input(input_slips.remove(0));
        }
        for _i in 0..output_len {
            transaction.add_output(output_slips.remove(0));
        }

        let hash_for_signature: SaitoHash = hash(&transaction.serialize_for_signature());
        transaction.set_hash_for_signature(hash_for_signature);
        transaction.sign(self.get_privatekey());

        transaction
    }

    //
    // creates a staking withdrawal transaction if possible that removes a slip from
    // the staking table. this function is primarily used for testing and as a reference
    // for how these transactions should be formatted, so we will just withdraw the first
    // staking slip.
    //
    pub async fn create_staking_withdrawal_transaction(
        &mut self,
        staking: &Staking,
    ) -> Transaction {
        let mut transaction = Transaction::new();
        transaction.set_transaction_type(TransactionType::StakerWithdrawal);

        if self.staked_slips.is_empty() {
            return transaction;
        }

        let slip = self.staked_slips[0].clone();

        let mut input = Slip::new();
        input.set_publickey(self.get_publickey());
        input.set_amount(slip.get_amount());
        input.set_uuid(slip.get_uuid());
        input.set_slip_ordinal(slip.get_slip_ordinal());
        input.set_slip_type(SlipType::StakerWithdrawalStaking);

        if staking.validate_slip_in_stakers(input.clone()) {
            println!("this slip is in stakers");
            input.set_slip_type(SlipType::StakerWithdrawalStaking);
        }
        if staking.validate_slip_in_pending(input.clone()) {
            println!("this slip is in pending");
            input.set_slip_type(SlipType::StakerWithdrawalPending);
        }

        let mut output = input.clone();
        output.set_slip_type(SlipType::Normal);

        // just convert to a normal transaction
        transaction.add_input(input);
        transaction.add_output(output);

        let hash_for_signature: SaitoHash = hash(&transaction.serialize_for_signature());
        transaction.set_hash_for_signature(hash_for_signature);
        transaction.sign(self.get_privatekey());

        // and remember it is spent!
        self.staked_slips[0].set_spent(true);

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
    #[allow(clippy::new_without_default)]
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
