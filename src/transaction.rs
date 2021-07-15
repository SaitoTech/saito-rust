use std::convert::TryInto;

use crate::{
    blockchain::UtxoSet,
    crypto::{
        generate_random_bytes, hash, sign, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey,
        SaitoSignature, SaitoUTXOSetKey,
    },
    hop::Hop,
    slip::{Slip, SlipType, SLIP_SIZE},
    wallet::Wallet,
};
use ahash::AHashMap;
use bigint::uint::U256;
use enum_variant_count_derive::TryFromByte;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const TRANSACTION_SIZE: usize = 85;

/// TransactionType is a human-readable indicator of the type of
/// transaction such as a normal user-initiated transaction, a
/// golden ticket transaction, a VIP-transaction or a rebroadcast
/// transaction created by a block creator, etc.
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone, TryFromByte)]
pub enum TransactionType {
    Normal,
    Fee,
    GoldenTicket,
    ATR,
    Vip,
    Other,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Transaction {
    // the bulk of the consensus transaction data
    timestamp: u64,
    pub inputs: Vec<Slip>,
    pub outputs: Vec<Slip>,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    transaction_type: TransactionType,
    #[serde_as(as = "[_; 64]")]
    signature: SaitoSignature,
    path: Vec<Hop>,

    // hash used for merkle_root (does not include signature), and slip uuid
    hash_for_signature: Option<SaitoHash>,

    pub total_in: u64,
    pub total_out: u64,
    pub total_fees: u64,
    pub cumulative_fees: u64,

    pub routing_work_for_me: u64,
    pub routing_work_for_creator: u64,
}

impl Transaction {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            inputs: vec![],
            outputs: vec![],
            message: vec![],
            transaction_type: TransactionType::Normal,
            signature: [0; 64],
            hash_for_signature: None,
            path: vec![],
            total_in: 0,
            total_out: 0,
            total_fees: 0,
            cumulative_fees: 0,
            routing_work_for_me: 0,
            routing_work_for_creator: 0,
        }
    }

    pub async fn add_hop_to_path(
        &mut self,
        wallet_lock: Arc<RwLock<Wallet>>,
        to_publickey: SaitoPublicKey,
    ) {
        //
        // msg is transaction signature and next peer
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.get_signature());
        vbytes.extend(&to_publickey);
        let hash_to_sign = hash(&vbytes);

        let hop = Hop::generate_hop(wallet_lock.clone(), to_publickey, hash_to_sign).await;

        //
        // add to path
        //
        self.path.push(hop);
    }

    pub fn validate_routing_path(&self) -> bool {
        for i in 0..self.path.len() {
            //
            // msg is transaction signature and next peer
            //
            let mut vbytes: Vec<u8> = vec![];
            vbytes.extend(&self.get_signature());
            vbytes.extend(&self.path[i].get_to());

            // check sig is valid
            if !verify(
                &hash(&vbytes),
                self.path[i].get_sig(),
                self.path[i].get_from(),
            ) {
                return false;
            }

            // check path is continuous
            if i > 0 {
                if self.path[i].get_from() != self.path[i - 1].get_to() {
                    return false;
                }
            }
        }

        return true;
    }

    //
    // this function exists largely for testing. It attempts to attach the requested fee
    // to the transaction if possible. If not possible it reverts back to a transaction
    // with 1 zero-fee input and 1 zero-fee output.
    //
    pub async fn generate_transaction(
        wallet_lock: Arc<RwLock<Wallet>>,
        to_publickey: SaitoPublicKey,
        with_payment: u64,
        with_fee: u64,
    ) -> Transaction {
        let mut wallet = wallet_lock.write().await;
        let wallet_publickey = wallet.get_publickey();

        let available_balance = wallet.get_available_balance();
        let total_requested = with_payment + with_fee;
        //println!("in generate transaction ab: {} and pr: {} and fr: {}", available_balance, with_payment, with_fee);

        if available_balance >= total_requested {
            let mut transaction = Transaction::new();
            let (mut input_slips, mut output_slips) = wallet.generate_slips(total_requested);
            let input_len = input_slips.len();
            let output_len = output_slips.len();

            for _i in 0..input_len {
                transaction.add_input(input_slips.remove(0));
            }
            for _i in 0..output_len {
                transaction.add_output(output_slips.remove(0));
            }

            // add the payment
            let mut output = Slip::new();
            output.set_publickey(to_publickey);
            output.set_amount(with_payment);
            transaction.add_output(output);

            //println!("inputs are: {}", transaction.get_inputs().len());

            return transaction;
        } else {
            if available_balance > with_payment {
                let mut transaction = Transaction::new();
                let (mut input_slips, mut output_slips) = wallet.generate_slips(total_requested);
                let input_len = input_slips.len();
                let output_len = output_slips.len();

                for _i in 0..input_len {
                    transaction.add_input(input_slips.remove(0));
                }
                for _i in 0..output_len {
                    transaction.add_output(output_slips.remove(0));
                }

                // add the payment
                let mut output = Slip::new();
                output.set_publickey(to_publickey);
                output.set_amount(with_payment);
                transaction.add_output(output);

                return transaction;
            }

            if available_balance > with_fee {
                let mut transaction = Transaction::new();
                let (mut input_slips, mut output_slips) = wallet.generate_slips(total_requested);
                let input_len = input_slips.len();
                let output_len = output_slips.len();

                for _i in 0..input_len {
                    transaction.add_input(input_slips.remove(0));
                }
                for _i in 0..output_len {
                    transaction.add_output(output_slips.remove(0));
                }

                return transaction;
            }

            //
            // we have neither enough for the payment OR the fee, so
            // we just create a transaction that has no payment AND no
            // attached fee.
            //
            let mut transaction = Transaction::new();

            let mut input1 = Slip::new();
            input1.set_publickey(to_publickey);
            input1.set_amount(0);
            let random_uuid = hash(&generate_random_bytes(32));
            input1.set_uuid(random_uuid);

            let mut output1 = Slip::new();
            output1.set_publickey(wallet_publickey);
            output1.set_amount(0);
            output1.set_uuid([0; 32]);

            transaction.add_input(input1);
            transaction.add_output(output1);

            return transaction;
        }
    }

    //
    // generate a transaction that has more outputs than inputs. this is primarily being used for testing
    // but it will be used to populate the first block with the VIP transactions in the future. VIP
    // transactions are provided . they are distinct from normal transactions in that they do not have
    // fees assessed for rebroadcasting. thank you to everyone who has supported Saito and help make
    // this blockchain a reality. you deserve it.
    //
    pub async fn generate_vip_transaction(
        _wallet_lock: Arc<RwLock<Wallet>>,
        from_publickey: SaitoPublicKey,
        to_publickey: SaitoPublicKey,
        with_fee: u64,
    ) -> Transaction {
        let mut transaction = Transaction::new();
        transaction.set_transaction_type(TransactionType::Vip);

        let mut input = Slip::new();
        input.set_publickey(from_publickey);
        input.set_amount(0);
        input.set_slip_type(SlipType::VipInput);

        let mut output = Slip::new();
        output.set_publickey(to_publickey);
        output.set_amount(with_fee);
        output.set_slip_type(SlipType::VipOutput);

        transaction.add_input(input);
        transaction.add_output(output);

        println!("VIP going to: {:?}", to_publickey);

        transaction
    }

    //
    //
    // generate ATR transaction using source transaction and output slip
    //
    // this assumes that the
    //
    pub fn generate_rebroadcast_transaction(
        transaction_to_rebroadcast: &Transaction,
        output_slip_to_rebroadcast: &Slip,
        with_fee: u64,
    ) -> Transaction {
        let mut transaction = Transaction::new();
        let mut output_payment = output_slip_to_rebroadcast.get_amount() - with_fee;
        if output_payment < 0 {
            output_payment = 0;
        }

        transaction.set_transaction_type(TransactionType::ATR);

        let mut input = Slip::new();
        input.set_publickey(output_slip_to_rebroadcast.get_publickey());
        input.set_amount(output_slip_to_rebroadcast.get_amount());
        input.set_slip_type(output_slip_to_rebroadcast.get_slip_type());

        let mut output = Slip::new();
        output.set_publickey(output_slip_to_rebroadcast.get_publickey());
        output.set_amount(output_payment);
        output.set_slip_type(SlipType::ATR);
        output.set_uuid(output_slip_to_rebroadcast.get_uuid());

        if input.get_slip_type() == SlipType::ATR {
            //
            // this is not our first rebroadcast, which means we just
            // need to copy the rebroadcast message field when making
            // this transaction.
            //
            transaction.set_message(transaction_to_rebroadcast.get_message().to_vec());
        } else {
            //
            // this is our first rebroadcast, so we need to set the
            // message field in the transaction to the content of the
            // original transaction being rebroadcast.
            //
            transaction.set_message(transaction_to_rebroadcast.serialize_for_net().to_vec());
        }

        transaction.add_input(input);
        transaction.add_output(output);

        //
        // signature is the ORIGINAL signature. this transaction
        // will fail its signature check and then get analysed as
        // a rebroadcast transaction because of its transaction type.
        //
        transaction.set_signature(transaction_to_rebroadcast.get_signature());

        transaction
    }

    pub fn get_routing_work_for_publickey(&self, publickey: SaitoPublicKey) -> u64 {
        // there is not routing path
        if self.path.len() == 0 {
            return 0;
        }

        // we are not the last routing node
        let last_hop = &self.path[self.path.len() - 1];
        if last_hop.get_to() != publickey {
            return 0;
        }

        let total_fees = self.get_total_fees();
        let mut routing_work_available_to_publickey = total_fees;

        //
        // first hop gets ALL the routing work, so we start
        // halving from the 2nd hop in the routing path
        //
        for _i in 1..self.path.len() {
            // return nothing if the path is broken
            if self.path[_i].get_to() != self.path[_i - 1].get_from() {
                return 0;
            }

            // otherwise halve the work
            let half_of_routing_work: u64 = routing_work_available_to_publickey / 2;
            routing_work_available_to_publickey -= half_of_routing_work;
        }

        return routing_work_available_to_publickey;
    }

    pub fn add_input(&mut self, input_slip: Slip) {
        self.inputs.push(input_slip);
    }

    pub fn add_output(&mut self, output_slip: Slip) {
        self.outputs.push(output_slip);
    }

    pub fn is_fee_transaction(&self) -> bool {
        self.transaction_type == TransactionType::Fee
    }

    pub fn is_golden_ticket(&self) -> bool {
        self.transaction_type == TransactionType::GoldenTicket
    }

    pub fn get_path(&self) -> &Vec<Hop> {
        &self.path
    }

    pub fn get_total_fees(&self) -> u64 {
        self.total_fees
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_transaction_type(&self) -> TransactionType {
        self.transaction_type
    }

    pub fn get_inputs(&self) -> &Vec<Slip> {
        &self.inputs
    }

    pub fn get_mut_inputs(&mut self) -> &mut Vec<Slip> {
        &mut self.inputs
    }

    pub fn get_mut_outputs(&mut self) -> &mut Vec<Slip> {
        &mut self.outputs
    }

    pub fn get_outputs(&self) -> &Vec<Slip> {
        &self.outputs
    }

    pub fn get_message(&self) -> &Vec<u8> {
        &self.message
    }

    pub fn get_hash_for_signature(&self) -> Option<SaitoHash> {
        self.hash_for_signature
    }

    pub fn get_signature(&self) -> [u8; 64] {
        self.signature
    }

    pub fn get_winning_routing_node(&self, random_hash: SaitoHash) -> SaitoPublicKey {
        //
        // if there are no routing paths, we return the sender of
        // the payment, as they're got all of the routing work by
        // definition. this is the edge-case where sending a tx
        // can make you money.
        //
        if self.path.len() == 0 {
            if self.inputs.len() > 0 {
                return self.inputs[0].get_publickey();
            } else {
                return [0; 33];
            }
        }

        //
        // if we have a routing path, we calculate the total amount
        // of routing work that it is possible for this transaction
        // to contain (2x the fee).
        //
        let mut aggregate_routing_work: u64 = self.get_total_fees();
        let mut routing_work_this_hop: u64 = aggregate_routing_work;
        let mut work_by_hop: Vec<u64> = vec![];
        work_by_hop.push(aggregate_routing_work);

        for _i in 1..self.path.len() {
            let new_routing_work_this_hop: u64 = routing_work_this_hop / 2;
            aggregate_routing_work += new_routing_work_this_hop;
            routing_work_this_hop = new_routing_work_this_hop;
            work_by_hop.push(aggregate_routing_work);
        }

        println!("work by hop: {:?}", work_by_hop);

        //
        // find winning routing node
        //
        let x = U256::from_big_endian(&random_hash);
        let z = U256::from_big_endian(&aggregate_routing_work.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);
        let winning_routing_work_in_nolan = zy.low_u64();

        println!("wrwin: {}", winning_routing_work_in_nolan);

        for i in 0..work_by_hop.len() {
            if winning_routing_work_in_nolan <= work_by_hop[i] {
                return self.path[i].get_to();
            }
        }

        //
        // we should never reach this
        //
        return [0; 33];
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn set_transaction_type(&mut self, transaction_type: TransactionType) {
        self.transaction_type = transaction_type;
    }

    pub fn set_inputs(&mut self, inputs: Vec<Slip>) {
        self.inputs = inputs;
    }

    pub fn set_outputs(&mut self, outputs: Vec<Slip>) {
        self.outputs = outputs;
    }

    pub fn set_message(&mut self, message: Vec<u8>) {
        self.message = message;
    }

    pub fn set_signature(&mut self, sig: SaitoSignature) {
        self.signature = sig;
    }

    pub fn set_hash_for_signature(&mut self, hash: SaitoHash) {
        self.hash_for_signature = Some(hash);
    }

    pub fn sign(&mut self, privatekey: SaitoPrivateKey) {
        //
        // we set slip ordinals when signing
        //
        for (i, output) in self.get_mut_outputs().iter_mut().enumerate() {
            output.set_slip_ordinal(i as u8);
        }

        let hash_for_signature = hash(&self.serialize_for_signature());
        self.set_signature(sign(&hash_for_signature, privatekey));
        self.set_hash_for_signature(hash_for_signature);
    }

    pub fn serialize_for_signature(&self) -> Vec<u8> {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.timestamp.to_be_bytes());
        for input in &self.inputs {
            vbytes.extend(&input.serialize_input_for_signature());
        }
        for output in &self.outputs {
            vbytes.extend(&output.serialize_output_for_signature());
        }
        vbytes.extend(&(self.transaction_type as u32).to_be_bytes());
        vbytes.extend(&self.message);

        vbytes
    }
    /// Deserialize from bytes to a Transaction.
    /// [len of inputs - 4 bytes - u32]
    /// [len of outputs - 4 bytes - u32]
    /// [len of message - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    pub fn deserialize_from_net(bytes: Vec<u8>) -> Transaction {
        let inputs_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let outputs_len: u32 = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let message_len: usize = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let signature: SaitoSignature = bytes[12..76].try_into().unwrap();

        let timestamp: u64 = u64::from_be_bytes(bytes[76..84].try_into().unwrap());
        let transaction_type: TransactionType = TransactionType::try_from(bytes[84]).unwrap();
        let start_of_inputs = TRANSACTION_SIZE;
        let start_of_outputs = start_of_inputs + inputs_len as usize * SLIP_SIZE;
        let start_of_message = start_of_outputs + outputs_len as usize * SLIP_SIZE;
        let mut inputs: Vec<Slip> = vec![];
        for n in 0..inputs_len {
            let start_of_data: usize = start_of_inputs as usize + n as usize * SLIP_SIZE;
            let end_of_data: usize = start_of_data + SLIP_SIZE;
            let input = Slip::deserialize_from_net(bytes[start_of_data..end_of_data].to_vec());
            inputs.push(input);
        }
        let mut outputs: Vec<Slip> = vec![];
        for n in 0..outputs_len {
            let start_of_data: usize = start_of_outputs as usize + n as usize * SLIP_SIZE;
            let end_of_data: usize = start_of_data + SLIP_SIZE;
            let output = Slip::deserialize_from_net(bytes[start_of_data..end_of_data].to_vec());
            outputs.push(output);
        }
        let message = bytes[start_of_message..start_of_message + message_len]
            .try_into()
            .unwrap();
        let mut transaction = Transaction::new();
        transaction.set_timestamp(timestamp);
        transaction.set_inputs(inputs);
        transaction.set_outputs(outputs);
        transaction.set_message(message);
        transaction.set_transaction_type(transaction_type);
        transaction.set_signature(signature);
        transaction
    }

    /// Serialize a Transaction for transport or disk.
    /// [len of inputs - 4 bytes - u32]
    /// [len of outputs - 4 bytes - u32]
    /// [len of message - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.inputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.outputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.message.len() as u32).to_be_bytes());
        vbytes.extend(&self.signature);
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&(self.transaction_type as u8).to_be_bytes());
        for input in &self.inputs {
            vbytes.extend(&input.serialize_for_net());
        }
        for output in &self.outputs {
            vbytes.extend(&output.serialize_for_net());
        }
        vbytes.extend(&self.message);
        vbytes
    }

    /// Runs when the chain is re-organized
    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        longest_chain: bool,
        block_id: u64,
    ) {
        let mut input_slip_value = 1;
        let mut output_slip_value = 0;

        if longest_chain {
            input_slip_value = block_id;
            output_slip_value = 1;
        }

        self.inputs.iter().for_each(|input| {
            input.on_chain_reorganization(utxoset, longest_chain, input_slip_value)
        });
        self.outputs.iter().for_each(|output| {
            output.on_chain_reorganization(utxoset, longest_chain, output_slip_value)
        });
    }

    //
    // we have to calculate cumulative fees and work sequentially.
    //
    pub fn pre_validation_calculations_cumulative_fees(&mut self, cumulative_fees: u64) -> u64 {
        self.cumulative_fees = cumulative_fees + self.total_fees;
        self.cumulative_fees
    }

    pub fn pre_validation_calculations_cumulative_work(&mut self, cumulative_work: u64) -> u64 {
        return cumulative_work + self.routing_work_for_creator;
    }

    pub fn calculate_total_fees(&mut self, creator_publickey: SaitoPublicKey) -> bool {
        //
        // and save the hash_for_signature so we can use it later...
        //
        // note that we must generate the HASH before we change the UUID in the
        // output slips created in this blog. Otherwise, our hash for the transaction
        // will change since the slips will be generated with a different UUID.
        //
        let hash_for_signature: SaitoHash = hash(&self.serialize_for_signature());
        self.set_hash_for_signature(hash_for_signature);

        //
        // calculate nolan in / out, fees
        //
        let mut nolan_in: u64 = 0;
        let mut nolan_out: u64 = 0;

        for input in &mut self.inputs {
            nolan_in += input.get_amount();
            // generate utxoset key cache
            input.generate_utxoset_key();
        }
        for output in &mut self.outputs {
            nolan_out += output.get_amount();

            // generate utxoset key cache
            // and set the UUID needed for insertion to shashmap
            output.set_uuid(hash_for_signature);
            output.generate_utxoset_key();
        }

        self.total_in = nolan_in;
        self.total_out = nolan_out;
        self.total_fees = 0;

        //
        // note that this is not validation code, permitting this. we may have
        // some transactions that do insert NOLAN, such as during testing of
        // monetary policy. All sanity checks need to be in the validate()
        // function.
        //
        if nolan_in > nolan_out {
            self.total_fees = nolan_in - nolan_out;
        }

        //
        // we also need to know how much routing work exists and is available
        // for the block producer, to ensure that they have met the conditions
        // required by the burn fee for block production.
        //
        self.routing_work_for_creator = self.get_routing_work_for_publickey(creator_publickey);

        true
    }

    pub fn validate(&self, utxoset: &UtxoSet) -> bool {
        if let Some(hash_for_signature) = self.get_hash_for_signature() {
            let sig: SaitoSignature = self.get_signature();

            if self.get_inputs().is_empty() {
                println!("Transaction must have at least 1 input");
                return false;
            }
            let publickey: SaitoPublicKey = self.get_inputs()[0].get_publickey();

            if !verify(&hash_for_signature, sig, publickey) {
                println!("message verifies not");
                return false;
            }
        }

        //
        // VALIDATE path sigs valid
        //
        if !self.validate_routing_path() {
            println!("routing path does not validate, transaction invalid");
            return false;
        }

        //
        // VALIDATE min one sender and receiver
        //
        if self.get_inputs().is_empty() {
            println!("ERROR 582039: less than 1 input in transaction");
            return false;
        }
        if self.get_outputs().is_empty() {
            println!("ERROR 582039: less than 1 output in transaction");
            return false;
        }

        // treasury in some amount.
        if self.total_out > self.total_in
            && self.get_transaction_type() != TransactionType::Fee
            && self.get_transaction_type() != TransactionType::Vip
        {
            println!("{} in and {} out", self.total_in, self.total_out);
            for z in self.get_outputs() {
                println!("{:?} --- ", z.get_amount());
            }
            println!("ERROR 672941: transaction spends more than it has available");
            return false;
        }

        self.inputs.par_iter().all(|input| input.validate(utxoset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{slip::Slip, time::create_timestamp, wallet::Wallet};

    #[test]
    fn transaction_new_test() {
        let tx = Transaction::new();
        assert_eq!(tx.timestamp, 0);
        assert_eq!(tx.inputs, vec![]);
        assert_eq!(tx.outputs, vec![]);
        assert_eq!(tx.message, Vec::<u8>::new());
        assert_eq!(tx.transaction_type, TransactionType::Normal);
        assert_eq!(tx.signature, [0; 64]);
        assert_eq!(tx.hash_for_signature, None);
        assert_eq!(tx.total_in, 0);
        assert_eq!(tx.total_out, 0);
        assert_eq!(tx.total_fees, 0);
        assert_eq!(tx.cumulative_fees, 0);
        assert_eq!(tx.routing_work_for_me, 0);
        assert_eq!(tx.routing_work_for_creator, 0);
    }

    #[test]
    fn transaction_sign_test() {
        let mut tx = Transaction::new();
        let wallet = Wallet::new();

        tx.set_outputs(vec![Slip::new()]);
        tx.sign(wallet.get_privatekey());

        assert_eq!(tx.get_outputs()[0].get_slip_ordinal(), 0);
        assert_ne!(tx.get_signature(), [0; 64]);
        assert_ne!(tx.get_hash_for_signature(), Some([0; 32]));
    }

    #[test]
    fn test_serialize_for_signature() {
        let tx = Transaction::new();
        assert_eq!(tx.serialize_for_signature(), vec![0; 12]);
    }

    #[test]
    fn transaction_pre_validation_cumulative_fees_test() {
        let mut tx = Transaction::new();
        tx.pre_validation_calculations_cumulative_fees(1_0000);
        assert_eq!(tx.cumulative_fees, 1_0000);
    }

    #[test]
    fn serialize_for_net_test() {
        let mock_input = Slip::new();
        let mock_output = Slip::new();
        let mut mock_tx = Transaction::new();
        mock_tx.set_timestamp(create_timestamp());
        mock_tx.add_input(mock_input);
        mock_tx.add_output(mock_output);
        mock_tx.set_message(vec![104, 101, 108, 108, 111]);
        mock_tx.set_transaction_type(TransactionType::Normal);
        mock_tx.set_signature([1; 64]);
        let serialized_tx = mock_tx.serialize_for_net();
        let deserialized_tx = Transaction::deserialize_from_net(serialized_tx);
        assert_eq!(mock_tx, deserialized_tx);
    }
}
