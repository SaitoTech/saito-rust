use std::convert::TryInto;

use crate::{
    blockchain::UtxoSet,
    crypto::{
        generate_random_bytes, hash, sign, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey,
        SaitoSignature, SaitoUTXOSetKey,
    },
    hop::{Hop, HOP_SIZE},
    slip::{Slip, SlipType, SLIP_SIZE},
    staking::Staking,
    wallet::Wallet,
};
use ahash::AHashMap;
use bigint::uint::U256;
use macros::TryFromByte;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use tracing::{event, Level};

pub const TRANSACTION_SIZE: usize = 89;

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
    StakerDeposit,
    StakerWithdrawal,
    Other,
    Issuance,
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
    #[allow(clippy::new_without_default)]
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

        true
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
        // println!("in generate transaction ab: {} and pr: {} and fr: {}", available_balance, with_payment, with_fee);

        if available_balance >= total_requested {
            let mut transaction = Transaction::new();
            let (mut input_slips, mut output_slips) = wallet.generate_slips(total_requested);
            let input_len = input_slips.len();
            let output_len = output_slips.len();

            for _i in 0..input_len {
                transaction.add_input(input_slips[0].clone());
                input_slips.remove(0);
            }
            for _i in 0..output_len {
                transaction.add_output(output_slips[0].clone());
                output_slips.remove(0);
            }

            // add the payment
            let mut output = Slip::new();
            output.set_publickey(to_publickey);
            output.set_amount(with_payment);
            transaction.add_output(output);

            transaction
        } else {
            if available_balance > with_payment {
                let mut transaction = Transaction::new();
                let (mut input_slips, mut output_slips) = wallet.generate_slips(total_requested);
                let input_len = input_slips.len();
                let output_len = output_slips.len();

                for _i in 0..input_len {
                    transaction.add_input(input_slips[0].clone());
                    input_slips.remove(0);
                }
                for _i in 0..output_len {
                    transaction.add_output(output_slips[0].clone());
                    output_slips.remove(0);
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
                    transaction.add_input(input_slips[0].clone());
                    input_slips.remove(0);
                }
                for _i in 0..output_len {
                    transaction.add_output(output_slips[0].clone());
                    output_slips.remove(0);
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

            transaction
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
        to_publickey: SaitoPublicKey,
        with_amount: u64,
        number_of_vip_slips: u64,
    ) -> Transaction {
        let mut transaction = Transaction::new();
        transaction.set_transaction_type(TransactionType::Vip);

        for _i in 0..number_of_vip_slips {
            let mut output = Slip::new();
            output.set_publickey(to_publickey);
            output.set_amount(with_amount);
            output.set_slip_type(SlipType::VipOutput);
            transaction.add_output(output);
        }

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
        let mut output_payment = 0;
        if output_slip_to_rebroadcast.get_amount() > with_fee {
            output_payment = output_slip_to_rebroadcast.get_amount() - with_fee;
        }

        transaction.set_transaction_type(TransactionType::ATR);

        let mut output = Slip::new();
        output.set_publickey(output_slip_to_rebroadcast.get_publickey());
        output.set_amount(output_payment);
        output.set_slip_type(SlipType::ATR);
        output.set_uuid(output_slip_to_rebroadcast.get_uuid());

        //
        // if this is the FIRST time we are rebroadcasting, we copy the
        // original transaction into the message field in serialized
        // form. this preserves the original message and its signature
        // in perpetuity.
        //
        // if this is the SECOND or subsequent rebroadcast, we do not
        // copy the ATR tx (no need for a meta-tx) and rather just update
        // the message field with the original transaction (which is
        // by definition already in the previous TX message space.
        //
        if output_slip_to_rebroadcast.get_slip_type() == SlipType::ATR {
            transaction.set_message(transaction_to_rebroadcast.get_message().to_vec());
        } else {
            transaction.set_message(transaction_to_rebroadcast.serialize_for_net().to_vec());
        }

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
        if self.path.is_empty() {
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

        routing_work_available_to_publickey
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

    pub fn is_atr_transaction(&self) -> bool {
        self.transaction_type == TransactionType::ATR
    }

    pub fn is_golden_ticket(&self) -> bool {
        self.transaction_type == TransactionType::GoldenTicket
    }

    pub fn is_issuance_transaction(&self) -> bool {
        self.transaction_type == TransactionType::Issuance
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
        if self.path.is_empty() {
            if !self.inputs.is_empty() {
                return self.inputs[0].get_publickey();
            } else {
                return [0; 33];
            }
        }

        //
        // no winning transaction should have no fees unless the
        // entire block has no fees, in which case we have a block
        // without any fee-paying transactions.
        //
        // burn these fees for the sake of safety.
        //
        if self.get_total_fees() == 0 {
            return [0; 33];
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

        //
        // find winning routing node
        //
        let x = U256::from_big_endian(&random_hash);
        let z = U256::from_big_endian(&aggregate_routing_work.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);
        let winning_routing_work_in_nolan = zy.low_u64();

        for i in 0..work_by_hop.len() {
            if winning_routing_work_in_nolan <= work_by_hop[i] {
                return self.path[i].get_to();
            }
        }

        //
        // we should never reach this
        //
        [0; 33]
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

    pub fn set_path(&mut self, path: Vec<Hop>) {
        self.path = path;
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
    /// [len of path - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    /// [hop][hop][hop]...
    pub fn deserialize_from_net(bytes: Vec<u8>) -> Transaction {
        let inputs_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let outputs_len: u32 = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let message_len: usize = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let path_len: usize = u32::from_be_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let signature: SaitoSignature = bytes[16..80].try_into().unwrap();
        let timestamp: u64 = u64::from_be_bytes(bytes[80..88].try_into().unwrap());
        let transaction_type: TransactionType = TransactionType::try_from(bytes[88]).unwrap();
        let start_of_inputs = TRANSACTION_SIZE;
        let start_of_outputs = start_of_inputs + inputs_len as usize * SLIP_SIZE;
        let start_of_message = start_of_outputs + outputs_len as usize * SLIP_SIZE;
        let start_of_path = start_of_message + message_len;
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
        let mut path: Vec<Hop> = vec![];
        for n in 0..path_len {
            let start_of_data: usize = start_of_path as usize + n as usize * HOP_SIZE;
            let end_of_data: usize = start_of_data + HOP_SIZE;
            let hop = Hop::deserialize_from_net(bytes[start_of_data..end_of_data].to_vec());
            path.push(hop);
        }

        let mut transaction = Transaction::new();
        transaction.set_timestamp(timestamp);
        transaction.set_inputs(inputs);
        transaction.set_outputs(outputs);
        transaction.set_message(message);
        transaction.set_transaction_type(transaction_type);
        transaction.set_signature(signature);
        transaction.set_path(path);
        transaction
    }

    /// Serialize a Transaction for transport or disk.
    /// [len of inputs - 4 bytes - u32]
    /// [len of outputs - 4 bytes - u32]
    /// [len of message - 4 bytes - u32]
    /// [len of path - 4 bytes - u32]
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [timestamp - 8 bytes - u64]
    /// [transaction type - 1 byte]
    /// [input][input][input]...
    /// [output][output][output]...
    /// [message]
    /// [hop][hop][hop]...
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.inputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.outputs.len() as u32).to_be_bytes());
        vbytes.extend(&(self.message.len() as u32).to_be_bytes());
        vbytes.extend(&(self.path.len() as u32).to_be_bytes());
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
        for hop in &self.path {
            vbytes.extend(&hop.serialize_for_net());
        }
        vbytes
    }

    // runs when block is deleted for good
    pub async fn delete(&self, utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>) -> bool {
        self.inputs.iter().for_each(|input| {
            input.delete(utxoset);
        });
        self.outputs.iter().for_each(|output| {
            output.delete(utxoset);
        });

        true
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
    // calculate cumulative fee share in block
    //
    pub fn generate_metadata_cumulative_fees(&mut self, cumulative_fees: u64) -> u64 {
        self.cumulative_fees = cumulative_fees + self.total_fees;
        self.cumulative_fees
    }
    //
    // calculate cumulative routing work in block
    //
    pub fn generate_metadata_cumulative_work(&mut self, cumulative_work: u64) -> u64 {
        cumulative_work + self.routing_work_for_creator
    }
    //
    // calculate hashes used in signatures
    //
    pub fn generate_metadata_hashes(&mut self) -> bool {
        //
        // and save the hash_for_signature so we can use it later...
        //
        // note that we must generate the HASH before we change the UUID in the
        // output slips created in this blog. Otherwise, our hash for the transaction
        // will change since the slips will be generated with a different UUID.
        //
        let hash_for_signature: SaitoHash = hash(&self.serialize_for_signature());
        self.set_hash_for_signature(hash_for_signature);

        true
    }
    //
    // calculate abstract metadata for fees
    //
    // note that this expects the hash_for_signature to have already
    // been calculated.
    //
    pub fn generate_metadata_fees_and_slips(&mut self, publickey: SaitoPublicKey) -> bool {
        //
        // calculate nolan in / out, fees
        //
        let mut nolan_in: u64 = 0;
        let mut nolan_out: u64 = 0;
        let hash_for_signature = self.get_hash_for_signature();

        for input in &mut self.inputs {
            nolan_in += input.get_amount();

            // generate utxoset key cache

            //
            // ATR txs have uuid already set
            // Fee txs have uuid already set
            //
            if input.get_slip_type() != SlipType::ATR {}

            input.generate_utxoset_key();
        }
        for output in &mut self.outputs {
            nolan_out += output.get_amount();

            //
            // generate utxoset key cache
            // and set the UUID needed for insertion to shashmap
            // skip for ATR slips
            //
            if let Some(hash_for_signature) = hash_for_signature {
                if output.get_slip_type() != SlipType::ATR {
                    output.set_uuid(hash_for_signature);
                }
            }
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
        self.routing_work_for_creator = self.get_routing_work_for_publickey(publickey);

        true
    }
    pub fn generate_metadata(&mut self, publickey: SaitoPublicKey) -> bool {
        self.generate_metadata_hashes();
        self.generate_metadata_fees_and_slips(publickey);
        true
    }

    pub fn validate(&self, utxoset: &UtxoSet, staking: &Staking) -> bool {
        //
        // Fee Transactions are validated in the block class. There can only
        // be one per block, and they are checked by ensuring the transaction hash
        // matches our self-generated safety check. We do not need to validate
        // their input slips as their input slips are records of what to do
        // when reversing/unwinding the chain and have been spent previously.
        //
        if self.get_transaction_type() == TransactionType::Fee {
            return true;
        }

        //
        // User-Sent Transactions
        //
        // most transactions are identifiable by the publickey that
        // has signed their input transaction, but some transactions
        // do not have senders as they are auto-generated as part of
        // the block itself.
        //
        // ATR transactions
        // VIP transactions
        // FEE transactions
        //
        // the first set of validation criteria is applied only to
        // user-sent transactions. validation criteria for the above
        // classes of transactions are further down in this function.
        // at the bottom is the validation criteria applied to ALL
        // transaction types.
        //
        let transaction_type = self.get_transaction_type();

        if transaction_type != TransactionType::Fee
            && transaction_type != TransactionType::ATR
            && transaction_type != TransactionType::Vip
            && transaction_type != TransactionType::Issuance
        {
            //
            // validate signature
            //
            if let Some(hash_for_signature) = self.get_hash_for_signature() {
                let sig: SaitoSignature = self.get_signature();
                let publickey: SaitoPublicKey = self.get_inputs()[0].get_publickey();
                if !verify(&hash_for_signature, sig, publickey) {
                    event!(Level::ERROR, "message verifies not");
                    return false;
                }
            } else {
                //
                // we reach here if we have not already calculated the hash
                // that is checked by the signature. while we could auto-gen
                // it here, we choose to throw an error to raise visibility of
                // unexpected behavior.
                //
                event!(
                    Level::ERROR,
                    "ERROR 757293: there is no hash for signature in a transaction"
                );
                return false;
            }
            println!("ERR6");

            //
            // validate sender exists
            //
            if self.get_inputs().is_empty() {
                event!(
                    Level::ERROR,
                    "ERROR 582039: less than 1 input in transaction"
                );
                return false;
            }

            println!("ERR7");
            //
            // validate routing path sigs
            //
            // a transaction without routing paths is valid, and pays off the
            // sender in the payment lottery. but a transaction with an invalid
            // routing path is fraudulent.
            //
            if !self.validate_routing_path() {
                event!(
                    Level::ERROR,
                    "ERROR 482033: routing paths do not validate, transaction invalid"
                );
                return false;
            }

            //
            // validate we're not creating tokens out of nothing
            //
            if self.total_out > self.total_in
                && self.get_transaction_type() != TransactionType::Fee
                && self.get_transaction_type() != TransactionType::Vip
            {
                event!(
                    Level::TRACE,
                    "{} in and {} out",
                    self.total_in,
                    self.total_out
                );
                for z in self.get_outputs() {
                    event!(Level::TRACE, "{:?} --- ", z.get_amount());
                }
                event!(
                    Level::TRACE,
                    "ERROR 672941: transaction spends more than it has available"
                );
                return false;
            }
            println!("ERR8");
        }

        //
        // fee transactions
        //
        if transaction_type == TransactionType::Fee {}

        //
        // atr transactions
        //
        if transaction_type == TransactionType::ATR {}

        //
        // normal transactions
        //
        if transaction_type == TransactionType::Normal {}

        //
        // golden ticket transactions
        //
        if transaction_type == TransactionType::GoldenTicket {}
        println!("ERR9");

        //
        // Staking Withdrawal Transactions
        //
        if transaction_type == TransactionType::StakerWithdrawal {
            for i in 0..self.inputs.len() {
                if self.inputs[i].get_slip_type() == SlipType::StakerWithdrawalPending {
                    if !staking.validate_slip_in_pending(self.inputs[i].clone()) {
                        println!("Staking Withdrawal Pending input slip is not in Pending thus transaction invalid!");
                        return false;
                    }
                }
                if self.inputs[i].get_slip_type() == SlipType::StakerWithdrawalStaking {
                    if !staking.validate_slip_in_stakers(self.inputs[i].clone()) {
                        println!("Staking Withdrawal Staker input slip is not in Staker thus transaction invalid!");
                        println!("STAKING SLIP WE HAVE: {:?}", self.inputs[i]);
                        println!("STAKING TABLE: {:?}", staking.stakers);
                        return false;
                    }
                }
            }
        }

        //
        // vip transactions
        //
        // a special class of transactions that do not pay rebroadcasting
        // fees. these are issued to the early supporters of the Saito
        // project. they carried us and we're going to carry them. thanks
        // for the faith and support.
        //
        if transaction_type == TransactionType::Vip {
            // we should validate that VIP transactions are signed by the
            // publickey associated with the Saito project.
        }

        println!("ERR1");

        //
        // all Transactions
        //
        // The following validation criteria apply to all transactions, including
        // those auto-generated and included in blocks.
        //
        //
        // all transactions must have outputs
        //
        if self.get_outputs().is_empty() {
            event!(
                Level::ERROR,
                "ERROR 582039: less than 1 output in transaction"
            );
            println!("ERR2");
            return false;
        }

        //
        // if inputs exist, they must validate against the UTXOSET
        // if they claim to spend tokens. if the slip has no spendable
        // tokens it will pass this check, which is conducted inside
        // the slip-level validation logic.
        //
        for i in 0..self.inputs.len() {
            let is_valid = self.inputs[i].validate(utxoset);
            if is_valid != true {
                println!("tx: {:?}", self);
                println!(
                    "this input is invalid: {:?}",
                    self.inputs[i].get_slip_type()
                );
                println!("ERR3");
                return false;
            }
        }

        let inputs_validate = self.inputs.par_iter().all(|input| input.validate(utxoset));
        println!("ERR4");
        inputs_validate
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
        let mut wallet = Wallet::new();
        wallet.load_keys("test/testwallet", Some("asdf"));

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
    fn transaction_generate_metadata_cumulative_fees_test() {
        let mut tx = Transaction::new();
        tx.generate_metadata_cumulative_fees(1_0000);
        assert_eq!(tx.cumulative_fees, 1_0000);
    }

    #[test]
    fn serialize_for_net_test() {
        let mock_input = Slip::new();
        let mock_output = Slip::new();
        let mut mock_hop = Hop::new();
        mock_hop.set_from([0; 33]);
        mock_hop.set_to([0; 33]);
        mock_hop.set_sig([0; 64]);
        let mut mock_tx = Transaction::new();
        let mut mock_path: Vec<Hop> = vec![];
        mock_path.push(mock_hop);
        let ctimestamp = create_timestamp();

        mock_tx.set_timestamp(ctimestamp);
        mock_tx.add_input(mock_input);
        mock_tx.add_output(mock_output);
        mock_tx.set_message(vec![104, 101, 108, 108, 111]);
        mock_tx.set_transaction_type(TransactionType::Normal);
        mock_tx.set_signature([1; 64]);
        mock_tx.set_path(mock_path);

        let serialized_tx = mock_tx.serialize_for_net();

        let deserialized_tx = Transaction::deserialize_from_net(serialized_tx);
        assert_eq!(mock_tx, deserialized_tx);
    }
}
