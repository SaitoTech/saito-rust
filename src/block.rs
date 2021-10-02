use crate::{
    blockchain::{Blockchain, GENESIS_PERIOD, MAX_STAKER_RECURSION},
    burnfee::BurnFee,
    crypto::{
        hash, sign, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature,
        SaitoUTXOSetKey,
    },
    golden_ticket::GoldenTicket,
    hop::HOP_SIZE,
    merkle::MerkleTreeLayer,
    slip::{Slip, SlipType, SLIP_SIZE},
    staking::Staking,
    storage::Storage,
    time::create_timestamp,
    transaction::{Transaction, TransactionType, TRANSACTION_SIZE},
    wallet::Wallet,
};
use ahash::AHashMap;
use bigint::uint::U256;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::{mem, sync::Arc};
use tokio::sync::RwLock;
use tracing::{event, span, Level};

pub const BLOCK_HEADER_SIZE: usize = 213;

//
// object used when generating and validation transactions, containing the
// information that is created selectively according to the transaction fees
// and the optional outbound payments.
//
#[derive(PartialEq, Debug, Clone)]
pub struct ConsensusValues {
    // expected transaction containing outbound payments
    pub fee_transaction: Option<Transaction>,
    // number of issuance transactions if exists
    pub it_num: u8,
    // index of issuance transactions if exists
    pub it_idx: Option<usize>,
    // number of FEE in transactions if exists
    pub ft_num: u8,
    // index of FEE in transactions if exists
    pub ft_idx: Option<usize>,
    // number of GT in transactions if exists
    pub gt_num: u8,
    // index of GT in transactions if exists
    pub gt_idx: Option<usize>,
    // total fees in block
    pub total_fees: u64,
    // expected difficulty
    pub expected_difficulty: u64,
    // rebroadcast txs
    pub rebroadcasts: Vec<Transaction>,
    // number of rebroadcast slips
    pub total_rebroadcast_slips: u64,
    // number of rebroadcast txs
    pub total_rebroadcast_nolan: u64,
    // number of rebroadcast fees in block
    pub total_rebroadcast_fees_nolan: u64,
    // all ATR txs hashed together
    pub rebroadcast_hash: [u8; 32],
    // dust falling off chain, needs adding to treasury
    pub nolan_falling_off_chain: u64,
    // staker treasury -> amount to add
    pub staking_treasury: i64,
    // block payout
    pub block_payout: Vec<BlockPayout>,
}
impl ConsensusValues {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> ConsensusValues {
        ConsensusValues {
            fee_transaction: None,
            it_num: 0,
            it_idx: None,
            ft_num: 0,
            ft_idx: None,
            gt_num: 0,
            gt_idx: None,
            total_fees: 0,
            expected_difficulty: 0,
            rebroadcasts: vec![],
            total_rebroadcast_slips: 0,
            total_rebroadcast_nolan: 0,
            total_rebroadcast_fees_nolan: 0,
            // must be initialized zeroed-out for proper hashing
            rebroadcast_hash: [0; 32],
            nolan_falling_off_chain: 0,
            staking_treasury: 0,
            block_payout: vec![],
        }
    }
}

//
// The BlockPayout object is returned by each block to report who
// receives the payment from the block. It is included in the
// consensus_values so that the fee transaction can be generated
// and validated.
//
#[derive(PartialEq, Debug, Clone)]
pub struct BlockPayout {
    pub miner: SaitoPublicKey,
    pub router: SaitoPublicKey,
    pub staker: SaitoPublicKey,
    pub miner_payout: u64,
    pub router_payout: u64,
    pub staker_payout: u64,
    pub staking_treasury: i64,
    pub staker_slip: Slip,
    pub random_number: SaitoHash,
}
impl BlockPayout {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> BlockPayout {
        BlockPayout {
            miner: [0; 33],
            router: [0; 33],
            staker: [0; 33],
            miner_payout: 0,
            router_payout: 0,
            staker_payout: 0,
            staking_treasury: 0,
            staker_slip: Slip::new(),
            random_number: [0; 32],
        }
    }
}

//
// The RouterPayout object is returned by the function that calculates
// who deserves the payment for each block. This is somewhat obsolete but
// is used to generate the BlockPayout object above.
//
// TODO eliminate reliance on this struct when creating the block payout
// object to permit its removal.
//
#[derive(PartialEq, Debug, Clone)]
pub struct RouterPayout {
    // expected transaction containing outbound payments
    pub publickey: SaitoPublicKey,
    pub random_number: SaitoHash,
}
impl RouterPayout {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> RouterPayout {
        RouterPayout {
            publickey: [0; 33],
            random_number: [0; 32],
        }
    }
}

///
/// BlockType is a human-readable indicator of the state of the block
/// with particular attention to its state of pruning and the amount of
/// data that is available. It is used by some functions to fetch blocks
/// that require certain types of data, such as the full set of transactions
/// or the UTXOSet
///
/// Hash - a ghost block sent to lite-clients primarily for SPV mode
/// Header - the header of the block without transaction data
/// Full - the full block including transactions and signatures
///
#[derive(Serialize, Deserialize, Debug, Copy, PartialEq, Clone)]
pub enum BlockType {
    Ghost,
    Header,
    Pruned,
    Full,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// Consensus Level Variables
    id: u64,
    timestamp: u64,
    previous_block_hash: [u8; 32],
    #[serde_as(as = "[_; 33]")]
    creator: [u8; 33],
    merkle_root: [u8; 32],
    #[serde_as(as = "[_; 64]")]
    signature: [u8; 64],
    treasury: u64,
    burnfee: u64,
    difficulty: u64,
    staking_treasury: u64,
    /// Transactions
    transactions: Vec<Transaction>,
    /// Self-Calculated / Validated
    pre_hash: SaitoHash,
    /// Self-Calculated / Validated
    hash: SaitoHash,
    /// total fees paid into block
    total_fees: u64,
    /// total fees paid into block
    routing_work_for_creator: u64,
    /// Is Block on longest chain
    lc: bool,
    // has golden ticket
    pub has_golden_ticket: bool,
    // has issuance transaction
    pub has_issuance_transaction: bool,
    // issuance transaction index
    pub issuance_transaction_idx: u64,
    // has fee transaction
    has_fee_transaction: bool,
    // golden ticket index
    golden_ticket_idx: u64,
    // fee transaction index
    fee_transaction_idx: u64,
    // number of rebroadcast slips
    total_rebroadcast_slips: u64,
    // number of rebroadcast txs
    total_rebroadcast_nolan: u64,
    // all ATR txs hashed together
    rebroadcast_hash: [u8; 32],
    // the state of the block w/ pruning etc
    block_type: BlockType,
    // vector of staker slips spent this block - used to prevent withdrawals and payouts same block
    #[serde(skip)]
    pub slips_spent_this_block: AHashMap<SaitoUTXOSetKey, u64>,
    #[serde(skip)]
    created_hashmap_of_slips_spent_this_block: bool,
}

impl Block {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Block {
        Block {
            id: 0,
            timestamp: 0,
            previous_block_hash: [0; 32],
            creator: [0; 33],
            merkle_root: [0; 32],
            signature: [0; 64],
            treasury: 0,
            burnfee: 0,
            difficulty: 0,
            staking_treasury: 0,
            transactions: vec![],
            pre_hash: [0; 32],
            hash: [0; 32],
            total_fees: 0,
            routing_work_for_creator: 0,
            lc: false,
            has_golden_ticket: false,
            has_fee_transaction: false,
            has_issuance_transaction: false,
            issuance_transaction_idx: 0,
            golden_ticket_idx: 0,
            fee_transaction_idx: 0,
            total_rebroadcast_slips: 0,
            total_rebroadcast_nolan: 0,
            // must be initialized zeroed-out for proper hashing
            rebroadcast_hash: [0; 32],
            //filename: String::new(),
            block_type: BlockType::Full,
            // hashmap of all SaitoUTXOSetKeys of the slips in the block
            slips_spent_this_block: AHashMap::new(),
            created_hashmap_of_slips_spent_this_block: false,
        }
    }

    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn get_hash(&self) -> SaitoHash {
        self.hash
    }

    pub fn get_lc(&self) -> bool {
        self.lc
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_previous_block_hash(&self) -> SaitoHash {
        self.previous_block_hash
    }

    pub fn get_creator(&self) -> SaitoPublicKey {
        self.creator
    }

    pub fn get_merkle_root(&self) -> SaitoHash {
        self.merkle_root
    }

    pub fn get_signature(&self) -> SaitoSignature {
        self.signature
    }

    pub fn get_treasury(&self) -> u64 {
        self.treasury
    }

    pub fn get_staking_treasury(&self) -> u64 {
        self.staking_treasury
    }

    pub fn get_burnfee(&self) -> u64 {
        self.burnfee
    }

    pub fn get_block_type(&self) -> BlockType {
        self.block_type
    }

    pub fn get_difficulty(&self) -> u64 {
        self.difficulty
    }

    pub fn get_has_golden_ticket(&self) -> bool {
        self.has_golden_ticket
    }

    pub fn get_has_issuance_transaction(&self) -> bool {
        self.has_issuance_transaction
    }

    pub fn get_has_fee_transaction(&self) -> bool {
        self.has_fee_transaction
    }

    pub fn get_golden_ticket_idx(&self) -> u64 {
        self.golden_ticket_idx
    }

    pub fn get_issuance_transaction_idx(&self) -> u64 {
        self.issuance_transaction_idx
    }

    pub fn get_fee_transaction_idx(&self) -> u64 {
        self.fee_transaction_idx
    }

    pub fn get_pre_hash(&self) -> SaitoHash {
        self.pre_hash
    }

    pub fn get_total_fees(&self) -> u64 {
        self.total_fees
    }

    pub fn get_routing_work_for_creator(&self) -> u64 {
        self.routing_work_for_creator
    }

    pub fn set_routing_work_for_creator(&mut self, routing_work_for_creator: u64) {
        self.routing_work_for_creator = routing_work_for_creator;
    }

    pub fn set_has_issuance_transaction(&mut self, hit: bool) {
        self.has_issuance_transaction = hit;
    }

    pub fn set_has_golden_ticket(&mut self, hgt: bool) {
        self.has_golden_ticket = hgt;
    }

    pub fn set_has_fee_transaction(&mut self, hft: bool) {
        self.has_fee_transaction = hft;
    }

    pub fn set_issuance_transaction_idx(&mut self, idx: u64) {
        self.issuance_transaction_idx = idx;
    }

    pub fn set_golden_ticket_idx(&mut self, idx: u64) {
        self.golden_ticket_idx = idx;
    }

    pub fn set_fee_transaction_idx(&mut self, idx: u64) {
        self.fee_transaction_idx = idx;
    }

    pub fn set_total_fees(&mut self, total_fees: u64) {
        self.total_fees = total_fees;
    }

    // TODO refactor: All of these setters which are setting something which is included
    // in the pre_hash or hash are dangerous. The purpose of the set/get paradigm is for
    // a class/unit to be able to enforce an API which guarantees it's consistency to
    // those using it. To be correct, each of these setters should call generate_hashes()
    // after it sets the state. However, this would be ridiculous because everytime we
    // construct a new Block, we call all these one after the other. A comprimise might
    // be to remove them and at least just set the private fields directly, or make the
    // setters private, but this misses the point. We want to encapsulate any state
    // changes into a single black-box so that state is easier to reason about.
    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
        self.transactions = transactions.to_vec();
    }

    pub fn set_block_type(&mut self, block_type: BlockType) {
        self.block_type = block_type;
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn set_lc(&mut self, lc: bool) {
        self.lc = lc;
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn set_previous_block_hash(&mut self, previous_block_hash: SaitoHash) {
        self.previous_block_hash = previous_block_hash;
    }

    pub fn set_creator(&mut self, creator: SaitoPublicKey) {
        self.creator = creator;
    }

    pub fn set_merkle_root(&mut self, merkle_root: SaitoHash) {
        self.merkle_root = merkle_root;
    }

    pub fn set_signature(&mut self, signature: SaitoSignature) {
        self.signature = signature;
    }

    pub fn set_staking_treasury(&mut self, staking_treasury: u64) {
        self.staking_treasury = staking_treasury;
    }

    pub fn set_treasury(&mut self, treasury: u64) {
        self.treasury = treasury;
    }

    pub fn set_burnfee(&mut self, burnfee: u64) {
        self.burnfee = burnfee;
    }

    pub fn set_difficulty(&mut self, difficulty: u64) {
        self.difficulty = difficulty;
    }

    pub fn set_pre_hash(&mut self, pre_hash: SaitoHash) {
        self.pre_hash = pre_hash;
    }

    pub fn set_hash(&mut self, hash: SaitoHash) {
        self.hash = hash;
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    //
    // if the block is not at the proper type, try to upgrade it to have the
    // data that is necessary for blocks of that type if possible. if this is
    // not possible, return false. if it is possible, return true once upgraded.
    //
    pub async fn upgrade_block_to_block_type(&mut self, block_type: BlockType) -> bool {
        let _span = span!(Level::TRACE, "UPGRADE BLOCK");
        event!(
            Level::TRACE,
            "UPGRADE_BLOCK_TO_BLOCK_TYPE {:?}",
            self.block_type
        );
        if self.block_type == block_type {
            return true;
        }

        //
        // TODO - if the block does not exist on disk, we have to
        // attempt a remote fetch.
        //

        //
        // if the block type needed is full and we are not,
        // load the block if it exists on disk.
        //
        if block_type == BlockType::Full {
            let mut new_block =
                Storage::load_block_from_disk(Storage::generate_block_filename(&self)).await;
            let hash_for_signature = hash(&new_block.serialize_for_signature());
            new_block.set_pre_hash(hash_for_signature);
            let hash_for_hash = hash(&new_block.serialize_for_hash());
            new_block.set_hash(hash_for_hash);

            //
            // in-memory swap copying txs in block from mempool
            //
            mem::swap(&mut new_block.transactions, &mut self.transactions);
            //
            // transactions need hashes
            //
            self.generate_metadata();
            self.set_block_type(BlockType::Full);

            return true;
        }

        false
    }

    //
    // if the block is not at the proper type, try to downgrade it by removing elements
    // that take up significant amounts of data / memory. if this is possible return
    // true, otherwise return false.
    //
    pub async fn downgrade_block_to_block_type(&mut self, block_type: BlockType) -> bool {
        let _span = span!(Level::TRACE, "DOWNGRADE BLOCK");
        event!(Level::TRACE, "BLOCK_ID {:?}", self.get_id());

        if self.block_type == block_type {
            return true;
        }

        //
        // if the block type needed is full and we are not,
        // load the block if it exists on disk.
        //
        if block_type == BlockType::Pruned {
            self.transactions = vec![];
            self.set_block_type(BlockType::Pruned);
            return true;
        }

        false
    }

    pub fn sign(&mut self, publickey: SaitoPublicKey, privatekey: SaitoPrivateKey) {
        //
        // we set final data
        //
        self.set_creator(publickey);
        self.generate_hashes();
        self.set_signature(sign(&self.get_pre_hash(), privatekey));
    }

    pub fn generate_hashes(&mut self) -> SaitoHash {
        //
        // fastest known way that isn't bincode ??
        //
        let hash_for_signature = hash(&self.serialize_for_signature());
        self.set_pre_hash(hash_for_signature);
        let hash_for_hash = hash(&self.serialize_for_hash());
        self.set_hash(hash_for_hash);

        hash_for_hash
    }

    // serialize the pre_hash and the signature_for_source into a
    // bytes array that can be hashed and then have the hash set.
    pub fn serialize_for_hash(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.get_pre_hash());
        vbytes.extend(&self.get_signature());
        vbytes.extend(&self.get_previous_block_hash());
        vbytes
    }

    // serialize major block components for block signature
    // this will manually calculate the merkle_root if necessary
    // but it is advised that the merkle_root be already calculated
    // to avoid speed issues.
    pub fn serialize_for_signature(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.id.to_be_bytes());
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&self.previous_block_hash);
        vbytes.extend(&self.creator);
        vbytes.extend(&self.merkle_root);
        vbytes.extend(&self.treasury.to_be_bytes());
        vbytes.extend(&self.staking_treasury.to_be_bytes());
        vbytes.extend(&self.burnfee.to_be_bytes());
        vbytes.extend(&self.difficulty.to_be_bytes());
        vbytes
    }

    /// Serialize a Block for transport or disk.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [staking_treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn serialize_for_net(&self, block_type: BlockType) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];

        // block headers do not get tx data
        if block_type == BlockType::Header {
            vbytes.extend(&(0 as u32).to_be_bytes());
        } else {
            vbytes.extend(&(self.transactions.iter().len() as u32).to_be_bytes());
        }

        vbytes.extend(&self.id.to_be_bytes());
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&self.previous_block_hash);
        vbytes.extend(&self.creator);
        vbytes.extend(&self.merkle_root);
        vbytes.extend(&self.signature);
        vbytes.extend(&self.treasury.to_be_bytes());
        vbytes.extend(&self.staking_treasury.to_be_bytes());
        vbytes.extend(&self.burnfee.to_be_bytes());
        vbytes.extend(&self.difficulty.to_be_bytes());

        println!("SERIALIZED PRE TXS: {:?}", vbytes);
        let mut serialized_txs = vec![];

        // block headers do not get tx data
        if block_type != BlockType::Header {
            self.transactions.iter().for_each(|transaction| {
                serialized_txs.extend(transaction.serialize_for_net());
            });
            vbytes.extend(serialized_txs);
        }

        vbytes
    }
    /// Deserialize from bytes to a Block.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [staking_treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn deserialize_for_net(bytes: &Vec<u8>) -> Block {
        let transactions_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let id: u64 = u64::from_be_bytes(bytes[4..12].try_into().unwrap());
        let timestamp: u64 = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
        let previous_block_hash: SaitoHash = bytes[20..52].try_into().unwrap();
        let creator: SaitoPublicKey = bytes[52..85].try_into().unwrap();
        let merkle_root: SaitoHash = bytes[85..117].try_into().unwrap();
        let signature: SaitoSignature = bytes[117..181].try_into().unwrap();

        let treasury: u64 = u64::from_be_bytes(bytes[181..189].try_into().unwrap());
        let staking_treasury: u64 = u64::from_be_bytes(bytes[189..197].try_into().unwrap());

        let burnfee: u64 = u64::from_be_bytes(bytes[197..205].try_into().unwrap());
        let difficulty: u64 = u64::from_be_bytes(bytes[205..213].try_into().unwrap());
        let mut transactions = vec![];
        let mut start_of_transaction_data = BLOCK_HEADER_SIZE;
        for _n in 0..transactions_len {
            let inputs_len: u32 = u32::from_be_bytes(
                bytes[start_of_transaction_data..start_of_transaction_data + 4]
                    .try_into()
                    .unwrap(),
            );
            let outputs_len: u32 = u32::from_be_bytes(
                bytes[start_of_transaction_data + 4..start_of_transaction_data + 8]
                    .try_into()
                    .unwrap(),
            );
            let message_len: usize = u32::from_be_bytes(
                bytes[start_of_transaction_data + 8..start_of_transaction_data + 12]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let path_len: usize = u32::from_be_bytes(
                bytes[start_of_transaction_data + 12..start_of_transaction_data + 16]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let end_of_transaction_data = start_of_transaction_data
                + TRANSACTION_SIZE
                + ((inputs_len + outputs_len) as usize * SLIP_SIZE)
                + message_len
                + path_len as usize * HOP_SIZE;
            let transaction = Transaction::deserialize_from_net(
                bytes[start_of_transaction_data..end_of_transaction_data].to_vec(),
            );
            transactions.push(transaction);
            start_of_transaction_data = end_of_transaction_data;
        }

        let mut block = Block::new();
        block.set_id(id);
        block.set_timestamp(timestamp);
        block.set_previous_block_hash(previous_block_hash);
        block.set_creator(creator);
        block.set_merkle_root(merkle_root);
        block.set_signature(signature);
        block.set_treasury(treasury);
        block.set_burnfee(burnfee);
        block.set_difficulty(difficulty);
        block.set_staking_treasury(staking_treasury);
        block.set_transactions(&mut transactions);
        if transactions_len == 0 {
            block.set_block_type(BlockType::Header);
        }
        block.generate_hashes();
        block
    }

    //
    // TODO - this logic should probably be in the merkle-root class
    //
    pub fn generate_merkle_root(&self) -> SaitoHash {
        if self.transactions.is_empty() {
            return [0; 32];
        }

        let tx_sig_hashes: Vec<SaitoHash> = self
            .transactions
            .iter()
            .map(|tx| tx.get_hash_for_signature().unwrap())
            .collect();

        let mut mrv: Vec<MerkleTreeLayer> = vec![];

        //
        // or let's try another approach
        //
        let tsh_len = tx_sig_hashes.len();
        let mut leaf_depth = 0;

        for i in 0..tsh_len {
            if (i + 1) < tsh_len {
                mrv.push(MerkleTreeLayer::new(
                    tx_sig_hashes[i],
                    tx_sig_hashes[i + 1],
                    leaf_depth,
                ));
            } else {
                mrv.push(MerkleTreeLayer::new(tx_sig_hashes[i], [0; 32], leaf_depth));
            }
        }

        let mut start_point = 0;
        let mut stop_point = mrv.len();
        let mut keep_looping = true;

        while keep_looping {
            // processing new layer
            leaf_depth += 1;

            // hash the parent in parallel
            mrv[start_point..stop_point]
                .par_iter_mut()
                .all(|leaf| leaf.hash());

            let start_point_old = start_point;
            start_point = mrv.len();

            for i in (start_point_old..stop_point).step_by(2) {
                if (i + 1) < stop_point {
                    mrv.push(MerkleTreeLayer::new(
                        mrv[i].get_hash(),
                        mrv[i + 1].get_hash(),
                        leaf_depth,
                    ));
                } else {
                    mrv.push(MerkleTreeLayer::new(mrv[i].get_hash(), [0; 32], leaf_depth));
                }
            }

            stop_point = mrv.len();
            if stop_point > 0 {
                keep_looping = start_point < stop_point - 1;
            } else {
                keep_looping = false;
            }
        }

        //
        // hash the final leaf
        //
        mrv[start_point].hash();
        mrv[start_point].get_hash()
    }

    //
    // generate hashes and payouts and fee calculations
    //
    pub async fn generate_consensus_values(&self, blockchain: &Blockchain) -> ConsensusValues {
        let mut cv = ConsensusValues::new();

        //
        // calculate total fees
        //
        // calculate fee and golden ticket and issuance (block 1) indices
        //
        let mut idx: usize = 0;
        for transaction in &self.transactions {
            if !transaction.is_fee_transaction() {
                cv.total_fees += transaction.get_total_fees();
            } else {
                cv.ft_num += 1;
                cv.ft_idx = Some(idx);
            }
            if transaction.is_golden_ticket() {
                cv.gt_num += 1;
                cv.gt_idx = Some(idx);
            }
            if transaction.is_issuance_transaction() {
                cv.it_num += 1;
                cv.it_idx = Some(idx);
            }
            idx += 1;
        }

        //
        // calculate expected burn-fee
        //
        if let Some(previous_block) = blockchain.blocks.get(&self.get_previous_block_hash()) {
            let difficulty = previous_block.get_difficulty();
            if !previous_block.get_has_golden_ticket() && cv.gt_num == 0 {
                if difficulty > 0 {
                    cv.expected_difficulty = previous_block.get_difficulty() - 1;
                }
            } else if previous_block.get_has_golden_ticket() && cv.gt_num > 0 {
                cv.expected_difficulty = difficulty + 1;
            } else {
                cv.expected_difficulty = difficulty;
            }
        } else {
            // TODO Is this actually an error? What should we do here?
            event!(Level::ERROR, "CAN'T FIND PREVIOUS BLOCK");
        }

        //
        // calculate automatic transaction rebroadcasts / ATR / atr
        //
        if self.get_id() > GENESIS_PERIOD {
            let pruned_block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(self.get_id() - 2);

            //
            // generate metadata should have prepared us with a pre-prune block
            // that contains all of the transactions and is ready to have its
            // ATR rebroadcasts calculated.
            //
            if let Some(pruned_block) = blockchain.blocks.get(&pruned_block_hash) {
                //
                // identify all unspent transactions
                //
                for transaction in &pruned_block.transactions {
                    for output in transaction.get_outputs() {
                        //
                        // valid means spendable and non-zero
                        //
                        if output.validate(&blockchain.utxoset) {
                            if output.get_amount() > 200_000_000 {
                                cv.total_rebroadcast_nolan += output.get_amount();
                                cv.total_rebroadcast_fees_nolan += 200_000_000;
                                cv.total_rebroadcast_slips += 1;

                                //
                                // create rebroadcast transaction
                                //
                                // TODO - floating fee based on previous block average
                                //
                                let rebroadcast_transaction =
                                    Transaction::generate_rebroadcast_transaction(
                                        &transaction,
                                        output,
                                        200_000_000,
                                    );

                                //
                                // update cryptographic hash of all ATRs
                                //
                                let mut vbytes: Vec<u8> = vec![];
                                vbytes.extend(&cv.rebroadcast_hash);
                                vbytes.extend(&rebroadcast_transaction.serialize_for_signature());
                                cv.rebroadcast_hash = hash(&vbytes);

                                cv.rebroadcasts.push(rebroadcast_transaction);
                            } else {
                                //
                                // rebroadcast dust is either collected into the treasury or
                                // distributed as a fee for the next block producer. for now
                                // we will simply distribute it as a fee. we may need to
                                // change this if the DUST becomes a significant enough amount
                                // each block to reduce consensus security.
                                //
                                cv.total_rebroadcast_fees_nolan += output.get_amount();
                            }
                        }
                    }
                }
            }
        }

        //
        // calculate payments to miners / routers / stakers
        //
        if let Some(gt_idx) = cv.gt_idx {
            let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                self.transactions[gt_idx].get_message().to_vec(),
            );
            // generate input hash for router
            let mut next_random_number = hash(&golden_ticket.get_random().to_vec());
            let _miner_publickey = golden_ticket.get_publickey();

            //
            // miner payout is fees from previous block, no staking treasury
            //
            if let Some(previous_block) = blockchain.blocks.get(&self.get_previous_block_hash()) {
                let miner_payment = previous_block.get_total_fees() / 2;
                let router_payment = previous_block.get_total_fees() - miner_payment;

                //
                // calculate miner and router payments
                //
                let block_payouts: RouterPayout =
                    previous_block.find_winning_router(next_random_number);
                let router_publickey = block_payouts.publickey;

                // these two from find_winning_router - 3, 4
                next_random_number = hash(&next_random_number.to_vec());
                next_random_number = hash(&next_random_number.to_vec());

                let mut payout = BlockPayout::new();
                payout.miner = golden_ticket.get_publickey();
                payout.router = router_publickey;
                payout.miner_payout = miner_payment;
                payout.router_payout = router_payment;

                cv.block_payout.push(payout);

                //
                // loop backwards until MAX recursion OR golden ticket
                //
                let mut cont = 1;
                let mut loop_idx = 0;
                let mut did_the_block_before_our_staking_block_have_a_golden_ticket =
                    previous_block.get_has_golden_ticket();
                //
                // staking block hash is 3 back, pre
                //
                let mut staking_block_hash = previous_block.get_previous_block_hash();

                while cont == 1 {
                    loop_idx += 1;

                    //
                    // we start with the second block, so once loop_IDX hits the same
                    // number as MAX_STAKER_RECURSION we have processed N blocks where
                    // N is MAX_STAKER_RECURSION.
                    //
                    if loop_idx >= MAX_STAKER_RECURSION {
                        cont = 0;
                    } else {
                        if let Some(staking_block) = blockchain.blocks.get(&staking_block_hash) {
                            staking_block_hash = staking_block.get_previous_block_hash();
                            if !did_the_block_before_our_staking_block_have_a_golden_ticket {
                                //
                                // update with this block info in case of next loop
                                //
                                did_the_block_before_our_staking_block_have_a_golden_ticket =
                                    staking_block.get_has_golden_ticket();

                                //
                                // calculate staker and router payments
                                //
                                // the staker payout is contained in the slip of the winner. this is
                                // because we calculate it afresh every time we reset the staking table
                                // the payment for the router requires calculating the amount that will
                                // be withheld for the staker treasury, which is what previous_staker_
                                // payment is measuring.
                                //
                                let sp = staking_block.get_total_fees() / 2;
                                let rp = staking_block.get_total_fees() - sp;

                                let mut payout = BlockPayout::new();
                                payout.router = staking_block
                                    .find_winning_router(next_random_number)
                                    .publickey;
                                payout.router_payout = rp;
                                payout.staking_treasury = sp as i64;

                                // router consumes 2 hashes
                                next_random_number = hash(&next_random_number.to_vec());
                                next_random_number = hash(&next_random_number.to_vec());

                                let staker_slip_option =
                                    blockchain.staking.find_winning_staker(next_random_number);
                                if let Some(staker_slip) = staker_slip_option {
                                    let mut slip_was_spent = 0;

                                    //
                                    // check to see if the block already pays out to this slip
                                    //
                                    for i in 0..cv.block_payout.len() {
                                        if cv.block_payout[i].staker_slip.get_utxoset_key()
                                            == staker_slip.get_utxoset_key()
                                        {
                                            slip_was_spent = 1;
                                            break;
                                        }
                                    }

                                    //
                                    // check to see if staker slip already spent/withdrawn
                                    //
                                    if self
                                        .slips_spent_this_block
                                        .contains_key(&staker_slip.get_utxoset_key())
                                    {
                                        slip_was_spent = 1;
                                    }

                                    //
                                    // add payout to staker if staker is new
                                    //
                                    // the payout is the return on staking, stored separately so that the
                                    // UTXO for the slip will still validate.
                                    //
                                    if slip_was_spent == 0 {
                                        payout.staker = staker_slip.get_publickey();
                                        payout.staker_payout =
                                            staker_slip.get_amount() + staker_slip.get_payout();
                                        payout.staker_slip = staker_slip.clone();
                                    }

                                    next_random_number = hash(&next_random_number.to_vec());

                                    cv.block_payout.push(payout);
                                }
                            }
                        }
                    }
                }
            }

            //
            // now create fee transaction using the block payout data
            //
            let mut slip_ordinal = 0;
            let mut transaction = Transaction::new();
            transaction.set_transaction_type(TransactionType::Fee);

            for i in 0..cv.block_payout.len() {
                if cv.block_payout[i].miner != [0; 33] {
                    let mut output = Slip::new();
                    output.set_publickey(cv.block_payout[i].miner);
                    output.set_amount(cv.block_payout[i].miner_payout);
                    output.set_slip_type(SlipType::MinerOutput);
                    output.set_slip_ordinal(slip_ordinal);
                    transaction.add_output(output.clone());
                    slip_ordinal += 1;
                }
                if cv.block_payout[i].router != [0; 33] {
                    let mut output = Slip::new();
                    output.set_publickey(cv.block_payout[i].router);
                    output.set_amount(cv.block_payout[i].router_payout);
                    output.set_slip_type(SlipType::RouterOutput);
                    output.set_slip_ordinal(slip_ordinal);
                    transaction.add_output(output.clone());
                    slip_ordinal += 1;
                }
                if cv.block_payout[i].staker != [0; 33] {
                    transaction.add_input(cv.block_payout[i].staker_slip.clone());

                    let mut output = Slip::new();
                    output.set_publickey(cv.block_payout[i].staker);
                    output.set_amount(cv.block_payout[i].staker_payout);
                    output.set_slip_type(SlipType::StakerOutput);
                    output.set_slip_ordinal(slip_ordinal);
                    transaction.add_output(output);
                    slip_ordinal += 1;
                    cv.staking_treasury += cv.block_payout[i].staking_treasury;
                    cv.staking_treasury -= cv.block_payout[i].staker_payout as i64;
                }
            }

            cv.fee_transaction = Some(transaction);
        }

        //
        // if there is no golden ticket AND there is no golden ticket before the MAX
        // blocks we recurse to collect NOLAN we have to add the amount of the unpaid
        // block to the amount of NOLAN that is falling off our chain.
        //
        if cv.gt_num == 0 {
            for i in 1..=MAX_STAKER_RECURSION {
                if i >= self.get_id() {
                    break;
                }

                let bid = self.get_id() - i;
                let previous_block_hash = blockchain
                    .blockring
                    .get_longest_chain_block_hash_by_block_id(bid);

                // previous block hash can be [0; 32] if there is no longest-chain block

                if previous_block_hash != [0; 32] {
                    let previous_block = blockchain.get_block(&previous_block_hash).await.unwrap();

                    if previous_block.get_has_golden_ticket() {
                        break;
                    } else {
                        //
                        // this is the block BEFORE from which we need to collect the nolan due to
                        // our iterator starting at 0 for the current block. i.e. if MAX_STAKER_
                        // RECURSION is 3, at 3 we are the fourth block back.
                        //
                        if i == MAX_STAKER_RECURSION {
                            cv.nolan_falling_off_chain = previous_block.get_total_fees();
                        }
                    }
                }
            }
        }

        cv
    }

    // consumes two hashes every time
    pub fn find_winning_router(&self, random_number: SaitoHash) -> RouterPayout {
        let mut rp = RouterPayout::new();

        //
        // find winning nolan
        //
        let x = U256::from_big_endian(&random_number);
        //
        // fee calculation should be the same used in block when
        // generating the fee transaction.
        //
        let y = self.get_total_fees();

        //
        // if there are no fees, payout to
        //
        if y == 0 {
            rp.publickey = [0; 33];
            return rp;
        }

        let z = U256::from_big_endian(&y.to_be_bytes());
        let (zy, _bolres) = x.overflowing_rem(z);
        let winning_nolan = zy.low_u64();
        // we may need function-timelock object if we need to recreate
        // an ATR transaction to pick the winning routing node.
        let winning_tx_placeholder: Transaction;
        let mut winning_tx: &Transaction;

        //
        // winning TX contains the winning nolan
        //
        // either a fee-paying transaction or an ATR transaction
        //
        winning_tx = &self.transactions[0];
        for transaction in &self.transactions {
            if transaction.cumulative_fees > winning_nolan {
                break;
            }
            winning_tx = &transaction;
        }

        //
        // if winner is atr, we take inside TX
        //
        if winning_tx.get_transaction_type() == TransactionType::ATR {
            let tmptx = winning_tx.get_message().to_vec();
            winning_tx_placeholder = Transaction::deserialize_from_net(tmptx);
            winning_tx = &winning_tx_placeholder;
        }

        //
        // hash random number to pick routing node
        //
        rp.publickey = winning_tx.get_winning_routing_node(hash(&random_number.to_vec()));

        rp
    }

    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        longest_chain: bool,
    ) -> bool {
        for tx in &self.transactions {
            tx.on_chain_reorganization(utxoset, longest_chain, self.get_id());
        }
        true
    }

    //
    // before we validate the block we need to generate some information such
    // as the hash of the transaction message data that is used to generate
    // the signature. because this requires mutable access to the transactions
    // Rust forces us to do it in a separate function.
    //
    // we first calculate as much information as we can in parallel before
    // sweeping through the transactions to find out what percentage of the
    // cumulative block fees they contain.
    //
    pub fn generate_metadata(&mut self) -> bool {
        event!(
            Level::TRACE,
            " ... block.prevalid - pre hash:  {:?}",
            create_timestamp(),
        );

        //
        // if we are generating the metadata for a block, we use the
        // publickey of the block creator when we calculate the fees
        // and the routing work.
        //
        let creator_publickey = self.get_creator();

        let _transactions_pre_calculated = &self
            .transactions
            .par_iter_mut()
            .all(|tx| tx.generate_metadata(creator_publickey));

        let _span = span!(Level::TRACE, "PREVALIDATION");
        event!(
            Level::TRACE,
            " ... block.prevalid - pst hash:  {:?}",
            create_timestamp()
        );

        //
        // we need to calculate the cumulative figures AFTER the
        // original figures.
        //
        let mut cumulative_fees = 0;
        let mut cumulative_work = 0;

        let mut has_golden_ticket = false;
        let mut has_fee_transaction = false;
        let mut has_issuance_transaction = false;
        let mut issuance_transaction_idx = 0;
        let mut golden_ticket_idx = 0;
        let mut fee_transaction_idx = 0;

        //
        // we have to do a single sweep through all of the transactions in
        // non-parallel to do things like generate the cumulative order of the
        // transactions in the block for things like work and fee calculations
        // for the lottery.
        //
        // we take advantage of the sweep to perform other pre-validation work
        // like counting up our ATR transactions and generating the hash
        // commitment for all of our rebroadcasts.
        //
        for i in 0..self.transactions.len() {
            let transaction = &mut self.transactions[i];

            cumulative_fees = transaction.generate_metadata_cumulative_fees(cumulative_fees);
            cumulative_work = transaction.generate_metadata_cumulative_work(cumulative_work);

            //
            // update slips_spent_this_block so that we have a record of
            // how many times input slips are spent in this block. we will
            // use this later to ensure there are no duplicates. this include
            // during the fee transaction, so that we cannot pay a staker
            // that is also paid this block otherwise.
            //
            // we skip the fee transaction as otherwise we have trouble
            // validating the staker slips if we have received a block from
            // someone else -- i.e. we will think the slip is spent in the
            // block when generating the FEE TX to check against the in-block
            // fee tx.
            //
            if !self.created_hashmap_of_slips_spent_this_block {
                if transaction.get_transaction_type() != TransactionType::Fee {
                    for input in transaction.get_inputs() {
                        self.slips_spent_this_block
                            .entry(input.get_utxoset_key())
                            .and_modify(|e| *e += 1)
                            .or_insert(1);
                    }
                    self.created_hashmap_of_slips_spent_this_block = true;
                }
            }

            //
            // also check the transactions for golden ticket and fees
            //
            match transaction.get_transaction_type() {
                TransactionType::Issuance => {
                    has_issuance_transaction = true;
                    issuance_transaction_idx = i as u64;
                }
                TransactionType::Fee => {
                    has_fee_transaction = true;
                    fee_transaction_idx = i as u64;
                }
                TransactionType::GoldenTicket => {
                    has_golden_ticket = true;
                    golden_ticket_idx = i as u64;
                }
                TransactionType::ATR => {
                    let mut vbytes: Vec<u8> = vec![];
                    vbytes.extend(&self.rebroadcast_hash);
                    vbytes.extend(&transaction.serialize_for_signature());
                    self.rebroadcast_hash = hash(&vbytes);

                    for input in transaction.get_inputs() {
                        self.total_rebroadcast_slips += 1;
                        self.total_rebroadcast_nolan += input.get_amount();
                    }
                }
                _ => {}
            };
        }
        self.set_has_fee_transaction(has_fee_transaction);
        self.set_has_golden_ticket(has_golden_ticket);
        self.set_has_issuance_transaction(has_issuance_transaction);
        self.set_fee_transaction_idx(fee_transaction_idx);
        self.set_golden_ticket_idx(golden_ticket_idx);
        self.set_issuance_transaction_idx(issuance_transaction_idx);

        //
        // update block with total fees
        //
        self.set_total_fees(cumulative_fees);
        self.set_routing_work_for_creator(cumulative_work);

        event!(
            Level::TRACE,
            " ... block.pre_validation_done:  {:?}",
            create_timestamp(),
            // tracing_tracker.time_since_last();
        );

        true
    }

    pub async fn validate(
        &self,
        blockchain: &Blockchain,
        utxoset: &AHashMap<SaitoUTXOSetKey, u64>,
        staking: &Staking,
    ) -> bool {
        //
        // no transactions? no thank you
        //
        if self.transactions.is_empty() {
            event!(
                Level::ERROR,
                "ERROR 424342: block does not validate as it has no transactions",
            );
            return false;
        }

        event!(
            Level::TRACE,
            " ... block.validate: (burn fee)  {:?}",
            create_timestamp(),
            // tracing_tracker.time_since_last();
        );

        //
        // verify signed by creator
        //
        if !verify(
            &self.get_pre_hash(),
            self.get_signature(),
            self.get_creator(),
        ) {
            event!(
                Level::ERROR,
                "ERROR 582039: block is not signed by creator or signature does not validate",
            );
            return false;
        }

        //
        // Consensus Values
        //
        // consensus data refers to the info in the proposed block that depends
        // on its relationship to other blocks in the chain -- things like the burn
        // fee, the ATR transactions, the golden ticket solution and more.
        //
        // the first step in validating our block is asking our software to calculate
        // what it thinks this data should be. this same function should have been
        // used by the block creator to create this block, so consensus rules allow us
        // to validate it by checking the variables we can see in our block with what
        // they should be given this function.
        //
        let cv = self.generate_consensus_values(&blockchain).await;

        //
        // only block #1 can have an issuance transaction
        //
        if cv.it_num > 0 && self.get_id() > 1 {
            event!(
                Level::ERROR,
                "ERROR: blockchain contains issuance after block 1 in chain",
            );
            return false;
        }

        //
        // Previous Block
        //
        // many kinds of validation like the burn fee and the golden ticket solution
        // require the existence of the previous block in order to validate. we put all
        // of these validation steps below so they will have access to the previous block
        //
        // if no previous block exists, we are valid only in a limited number of
        // circumstances, such as this being the first block we are adding to our chain.
        //
        if let Some(previous_block) = blockchain.blocks.get(&self.get_previous_block_hash()) {
            //
            // validate treasury
            //
            if self.get_treasury() != previous_block.get_treasury() + cv.nolan_falling_off_chain {
                event!(
                    Level::ERROR,
                    "ERROR: treasury does not validate: {} expected versus {} found",
                    (previous_block.get_treasury() + cv.nolan_falling_off_chain),
                    self.get_treasury(),
                    // tracing_tracker.time_since_last();
                );
                return false;
            }

            //
            // validate staking treasury
            //
            let mut adjusted_staking_treasury = previous_block.get_staking_treasury();
            if cv.staking_treasury < 0 {
                let x = cv.staking_treasury * -1;
                if adjusted_staking_treasury > x as u64 {
                    adjusted_staking_treasury -= x as u64;
                } else {
                    adjusted_staking_treasury = 0;
                }
            } else {
                let x: u64 = cv.staking_treasury as u64;
                adjusted_staking_treasury += x;
            }

            if self.get_staking_treasury() != adjusted_staking_treasury {
                event!(
                    Level::ERROR,
                    "ERROR: staking treasury does not validate: {} expected versus {} found",
                    adjusted_staking_treasury,
                    self.get_staking_treasury(),
                );
                //     "ERROR: staking treasury does not validate: {} expected versus {} found",
                //     adjusted_staking_treasury,
                //     self.get_staking_treasury(),
                return false;
            }

            //
            // validate burn fee
            //
            let new_burnfee: u64 =
                BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
                    previous_block.get_burnfee(),
                    self.get_timestamp(),
                    previous_block.get_timestamp(),
                );
            if new_burnfee != self.get_burnfee() {
                event!(
                    Level::ERROR,
                    "ERROR: burn fee does not validate, expected: {}",
                    new_burnfee
                );
                return false;
            }

            event!(
                Level::TRACE,
                " ... burn fee in blk validated:  {:?}",
                create_timestamp()
            );

            //
            // validate routing work required
            //
            // this checks the total amount of fees that need to be burned in this
            // block to be considered valid according to consensus criteria.
            //
            let amount_of_routing_work_needed: u64 =
                BurnFee::return_routing_work_needed_to_produce_block_in_nolan(
                    previous_block.get_burnfee(),
                    self.get_timestamp(),
                    previous_block.get_timestamp(),
                );
            if self.routing_work_for_creator < amount_of_routing_work_needed {
                event!(
                    Level::ERROR,
                    "Error 510293: block lacking adequate routing work from creator"
                );
                return false;
            }

            event!(
                Level::TRACE,
                " ... done routing work required: {:?}",
                create_timestamp()
            );

            //
            // validate golden ticket
            //
            // the golden ticket is a special kind of transaction that stores the
            // solution to the network-payment lottery in the transaction message
            // field. it targets the hash of the previous block, which is why we
            // tackle it's validation logic here.
            //
            // first we reconstruct the ticket, then calculate that the solution
            // meets our consensus difficulty criteria. note that by this point in
            // the validation process we have already examined the fee transaction
            // which was generated using this solution. If the solution is invalid
            // we find that out now, and it invalidates the block.
            //
            if let Some(gt_idx) = cv.gt_idx {
                let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                    self.get_transactions()[gt_idx].get_message().to_vec(),
                );
                let solution = GoldenTicket::generate_solution(
                    golden_ticket.get_random(),
                    golden_ticket.get_publickey(),
                );
                if !GoldenTicket::is_valid_solution(
                    previous_block.get_hash(),
                    solution,
                    previous_block.get_difficulty(),
                ) {
                    event!(
                        Level::ERROR,
                        "ERROR: Golden Ticket solution does not validate against previous block hash and difficulty"
                    );
                    return false;
                }
            }
            event!(
                Level::TRACE,
                " ... golden ticket: (validated)  {:?}",
                create_timestamp()
            );
        }

        event!(
            Level::TRACE,
            " ... block.validate: (merkle rt) {:?}",
            create_timestamp()
        );

        //
        // validate atr
        //
        // Automatic Transaction Rebroadcasts are removed programmatically from
        // an earlier block in the blockchain and rebroadcast into the latest
        // block, with a fee being deducted to keep the data on-chain. In order
        // to validate ATR we need to make sure we have the correct number of
        // transactions (and ONLY those transactions!) included in our block.
        //
        // we do this by comparing the total number of ATR slips and nolan
        // which we counted in the generate_metadata() function, with the
        // expected number given the consensus values we calculated earlier.
        //
        if cv.total_rebroadcast_slips != self.total_rebroadcast_slips {
            event!(
                Level::ERROR,
                "ERROR 624442: rebroadcast slips total incorrect"
            );
            return false;
        }
        if cv.total_rebroadcast_nolan != self.total_rebroadcast_nolan {
            event!(
                Level::ERROR,
                "ERROR 294018: rebroadcast nolan amount incorrect"
            );
            return false;
        }
        if cv.rebroadcast_hash != self.rebroadcast_hash {
            event!(
                Level::ERROR,
                "ERROR 123422: hash of rebroadcast transactions incorrect"
            );
            return false;
        }

        //
        // validate merkle root
        //
        if self.get_merkle_root() == [0; 32]
            && self.get_merkle_root() != self.generate_merkle_root()
        {
            event!(Level::ERROR, "merkle root is unset or is invalid false 1");
            return false;
        }

        event!(
            Level::TRACE,
            " ... block.validate: (cv-data)   {:?}",
            create_timestamp()
        );

        //
        // validate fee transactions
        //
        // if this block contains a golden ticket, we have to use the random
        // number associated with the golden ticket to create a fee-transaction
        // that stretches back into previous blocks and finds the winning nodes
        // that should collect payment.
        //
        if let (Some(ft_idx), Some(mut fee_transaction)) = (cv.ft_idx, cv.fee_transaction) {
            //
            // no golden ticket? invalid
            //
            if cv.gt_idx.is_none() {
                event!(
                    Level::ERROR,
                    "ERROR 48203: block appears to have fee transaction without golden ticket"
                );
                return false;
            }

            //
            // the fee transaction we receive from the CV needs to be updated with
            // block-specific data in the same way that all of the transactions in
            // the block have been. we must do this prior to comparing them.
            //
            fee_transaction.generate_metadata(self.get_creator());

            let hash1 = hash(&fee_transaction.serialize_for_signature());
            let hash2 = hash(&self.transactions[ft_idx].serialize_for_signature());
            if hash1 != hash2 {
                event!(
                    Level::ERROR,
                    "ERROR 627428: block fee transaction doesn't match cv fee transaction"
                );
                return false;
            }
        }

        //
        // validate difficulty
        //
        // difficulty here refers the difficulty of generating a golden ticket
        // for any particular block. this is the difficulty of the mining
        // puzzle that is used for releasing payments.
        //
        // those more familiar with POW and POS should note that "difficulty" of
        // finding a block is represented in the burn fee variable which we have
        // already examined and validated above. producing a block requires a
        // certain amount of golden ticket solutions over-time, so the
        // distinction is in practice less clean.
        //
        if cv.expected_difficulty != self.get_difficulty() {
            event!(
                Level::ERROR,
                "difficulty is false {} vs {}",
                cv.expected_difficulty,
                self.get_difficulty()
            );
            return false;
        }

        event!(
            Level::TRACE,
            " ... block.validate: (txs valid) {:?}",
            create_timestamp()
        );

        //
        // validate transactions
        //
        // validating transactions requires checking that the signatures are valid,
        // the routing paths are valid, and all of the input slips are pointing
        // to spendable tokens that exist in our UTXOSET. this logic is separate
        // from the validation of block-level variables, so is handled in the
        // transaction objects.
        //
        // this is one of the most computationally intensive parts of processing a
        // block which is why we handle it in parallel. the exact logic needed to
        // examine a transaction may depend on the transaction itself, as we have
        // some specific types (Fee / ATR / etc.) that are generated automatically
        // and may have different requirements.
        //
        // the validation logic for transactions is contained in the transaction
        // class, and the validation logic for slips is contained in the slips
        // class. Note that we are passing in a read-only copy of our UTXOSet so
        // as to determine spendability.
        //
        // TODO - remove when convenient. when transactions fail to validate using
        // parallel processing can make it difficult to find out exactly what the
        // problem is. ergo this code that tries to do them on the main thread so
        // debugging output works.
        //
        for i in 0..self.transactions.len() {
            let transactions_valid2 = self.transactions[i].validate(utxoset, staking);
            if !transactions_valid2 {
                println!("TType: {:?}", self.transactions[i].get_transaction_type());
                println!("Data {:?}", self.transactions[i]);
            }
        }
        //true

        let transactions_valid = self
            .transactions
            .par_iter()
            .all(|tx| tx.validate(utxoset, staking));

        // println!(" ... block.validate: (done all)  {:?}", create_timestamp());

        //
        // and if our transactions are valid, so is the block...
        //
        // println!(" ... are txs valid: {}", transactions_valid);
        transactions_valid
    }

    pub async fn generate(
        transactions: &mut Vec<Transaction>,
        previous_block_hash: SaitoHash,
        wallet_lock: Arc<RwLock<Wallet>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        current_timestamp: u64,
    ) -> Block {
        let blockchain = blockchain_lock.read().await;
        let wallet = wallet_lock.read().await;
        let publickey = wallet.get_publickey();

        let mut previous_block_id = 0;
        let mut previous_block_burnfee = 0;
        let mut previous_block_timestamp = 0;
        let mut previous_block_difficulty = 0;
        let mut previous_block_treasury = 0;
        let mut previous_block_staking_treasury = 0;

        if let Some(previous_block) = blockchain.blocks.get(&previous_block_hash) {
            previous_block_id = previous_block.get_id();
            previous_block_burnfee = previous_block.get_burnfee();
            previous_block_timestamp = previous_block.get_timestamp();
            previous_block_difficulty = previous_block.get_difficulty();
            previous_block_treasury = previous_block.get_treasury();
            previous_block_staking_treasury = previous_block.get_staking_treasury();
        }

        let mut block = Block::new();

        let current_burnfee: u64 =
            BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
                previous_block_burnfee,
                current_timestamp,
                previous_block_timestamp,
            );

        block.set_id(previous_block_id + 1);
        block.set_previous_block_hash(previous_block_hash);
        block.set_burnfee(current_burnfee);
        block.set_timestamp(current_timestamp);
        block.set_difficulty(previous_block_difficulty);

        //
        // in-memory swap copying txs in block from mempool
        //
        mem::swap(&mut block.transactions, transactions);

        //
        // update slips_spent_this_block so that we have a record of
        // how many times input slips are spent in this block. we will
        // use this later to ensure there are no duplicates. this include
        // during the fee transaction, so that we cannot pay a staker
        // that is also paid this block otherwise.
        //
        // this will not include the fee transaction or the ATR txs
        // because they have not been added to teh block yet, but they
        // permit us to avoid paying out StakerWithdrawal slips when we
        // generate the fee payment.
        //
        // note -- no need to have an exception for the FEE TX here as
        // we have not added it yet.
        //
        if !block.created_hashmap_of_slips_spent_this_block {
            for transaction in &block.transactions {
                for input in transaction.get_inputs() {
                    block
                        .slips_spent_this_block
                        .entry(input.get_utxoset_key())
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
                block.created_hashmap_of_slips_spent_this_block = true;
            }
        }

        //
        // contextual values
        //
        let mut cv: ConsensusValues = block.generate_consensus_values(&blockchain).await;

        //
        // ATR transactions
        //
        let rlen = cv.rebroadcasts.len();
        // TODO -- figure out if there is a more efficient solution
        // than iterating through the entire transaction set here.
        let _tx_hashes_generated = cv.rebroadcasts[0..rlen]
            .par_iter_mut()
            .all(|tx| tx.generate_metadata(publickey));
        if rlen > 0 {
            block.transactions.append(&mut cv.rebroadcasts);
        }

        //
        // fee transactions
        //
        // if a golden ticket is included in THIS block Saito uses the randomness
        // associated with that golden ticket to create a fair output for the
        // previous block.
        //
        if cv.fee_transaction.is_some() {
            //
            // creator signs fee transaction
            //
            let mut fee_tx = cv.fee_transaction.unwrap();
            let hash_for_signature: SaitoHash = hash(&fee_tx.serialize_for_signature());
            fee_tx.set_hash_for_signature(hash_for_signature);
            fee_tx.sign(wallet.get_privatekey());

            //
            // and we add it to the block
            //
            block.add_transaction(fee_tx);
        }

        //
        // update slips_spent_this_block so that we have a record of
        // how many times input slips are spent in this block. we will
        // use this later to ensure there are no duplicates. this include
        // during the fee transaction, so that we cannot pay a staker
        // that is also paid this block otherwise.
        //
        for transaction in &block.transactions {
            if transaction.get_transaction_type() != TransactionType::Fee {
                for input in transaction.get_inputs() {
                    block
                        .slips_spent_this_block
                        .entry(input.get_utxoset_key())
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }
        }
        block.created_hashmap_of_slips_spent_this_block = true;

        //
        // set difficulty
        //
        if cv.expected_difficulty != 0 {
            block.set_difficulty(cv.expected_difficulty);
        }

        //
        // set treasury
        //
        if cv.nolan_falling_off_chain != 0 {
            block.set_treasury(previous_block_treasury + cv.nolan_falling_off_chain);
        }

        //
        // set staking treasury
        //
        if cv.staking_treasury != 0 {
            let mut adjusted_staking_treasury = previous_block_staking_treasury;
            if cv.staking_treasury < 0 {
                let x = cv.staking_treasury * -1;
                if adjusted_staking_treasury > x as u64 {
                    adjusted_staking_treasury -= x as u64;
                } else {
                    adjusted_staking_treasury = 0;
                }
            } else {
                adjusted_staking_treasury += cv.staking_treasury as u64;
            }
            // println!(
            //     "adjusted staking treasury written into block {}",
            //     adjusted_staking_treasury
            // );
            block.set_staking_treasury(adjusted_staking_treasury);
        }

        //
        // generate merkle root
        //
        let block_merkle_root = block.generate_merkle_root();
        block.set_merkle_root(block_merkle_root);

        block.sign(wallet.get_publickey(), wallet.get_privatekey());

        block
    }

    pub async fn delete(&self, utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>) -> bool {
        for tx in &self.transactions {
            tx.delete(utxoset).await;
        }
        true
    }
}

#[cfg(test)]

mod tests {

    use super::*;
    use crate::{
        slip::Slip,
        test_utilities::test_manager::TestManager,
        time::create_timestamp,
        transaction::{Transaction, TransactionType},
        wallet::Wallet,
    };

    #[test]
    fn block_new_test() {
        let block = Block::new();
        assert_eq!(block.id, 0);
        assert_eq!(block.timestamp, 0);
        assert_eq!(block.previous_block_hash, [0; 32]);
        assert_eq!(block.creator, [0; 33]);
        assert_eq!(block.merkle_root, [0; 32]);
        assert_eq!(block.signature, [0; 64]);
        assert_eq!(block.treasury, 0);
        assert_eq!(block.burnfee, 0);
        assert_eq!(block.difficulty, 0);
        assert_eq!(block.transactions, vec![]);
        assert_eq!(block.hash, [0; 32]);
        assert_eq!(block.total_fees, 0);
        assert_eq!(block.lc, false);
        assert_eq!(block.has_golden_ticket, false);
        assert_eq!(block.has_fee_transaction, false);
        assert_eq!(block.has_issuance_transaction, false);
        assert_eq!(block.issuance_transaction_idx, 0);
        assert_eq!(block.fee_transaction_idx, 0);
        assert_eq!(block.golden_ticket_idx, 0);
        assert_eq!(block.routing_work_for_creator, 0);
        TestManager::check_block_consistency(&block);
    }

    #[test]
    // signs and verifies the signature of a block
    fn block_sign_test() {
        let wallet = Wallet::new();
        let mut block = Block::new();

        block.sign(wallet.get_publickey(), wallet.get_privatekey());

        assert_eq!(block.creator, wallet.get_publickey());
        assert_eq!(
            verify(
                &block.get_pre_hash(),
                block.get_signature(),
                block.get_creator()
            ),
            true
        );
        assert_ne!(block.get_hash(), [0; 32]);
        assert_ne!(block.get_signature(), [0; 64]);
        TestManager::check_block_consistency(&block);
    }

    #[test]
    // test that we are properly generating pre_hash and hash
    fn block_generate_hashes() {
        let mut block = Block::new();
        let hash = block.generate_hashes();
        assert_ne!(hash, [0; 32]);
        assert_ne!(block.get_pre_hash(), [0; 32]);
        assert_ne!(block.get_hash(), [0; 32]);
        TestManager::check_block_consistency(&block);
    }

    #[test]
    // confirm we have not modified the length of the serialized block
    fn block_serialize_for_signature_hash() {
        let block = Block::new();
        let serialized_body = block.serialize_for_signature();
        assert_eq!(serialized_body.len(), 145);
        TestManager::check_block_consistency(&block);
    }

    #[test]
    // confirm serialization / deserialization does not modify block content
    fn block_serialize_for_net_test() {
        let mock_input = Slip::new();
        let mock_output = Slip::new();
        let mut mock_tx = Transaction::new();
        mock_tx.set_timestamp(create_timestamp());
        mock_tx.add_input(mock_input.clone());
        mock_tx.add_output(mock_output.clone());
        mock_tx.set_message(vec![104, 101, 108, 111]);
        mock_tx.set_transaction_type(TransactionType::Normal);
        mock_tx.set_signature([1; 64]);

        let mut mock_tx2 = Transaction::new();
        mock_tx2.set_timestamp(create_timestamp());
        mock_tx2.add_input(mock_input);
        mock_tx2.add_output(mock_output);
        mock_tx2.set_message(vec![]);
        mock_tx2.set_transaction_type(TransactionType::Normal);
        mock_tx2.set_signature([2; 64]);

        let timestamp = create_timestamp();

        let mut block = Block::new();
        block.set_id(1);
        block.set_timestamp(timestamp);
        block.set_previous_block_hash([1; 32]);
        block.set_creator([2; 33]);
        block.set_merkle_root([3; 32]);
        block.set_signature([4; 64]);
        block.set_treasury(1);
        block.set_burnfee(2);
        block.set_difficulty(3);
        block.set_transactions(&mut vec![mock_tx, mock_tx2]);

        let serialized_block = block.serialize_for_net(BlockType::Full);
        let deserialized_block = Block::deserialize_for_net(&serialized_block);

        let serialized_block_header = block.serialize_for_net(BlockType::Header);
        let deserialized_block_header = Block::deserialize_for_net(&serialized_block_header);

        assert_eq!(
            block.serialize_for_net(BlockType::Full),
            deserialized_block.serialize_for_net(BlockType::Full)
        );
        assert_eq!(deserialized_block.get_id(), 1);
        assert_eq!(deserialized_block.get_timestamp(), timestamp);
        assert_eq!(deserialized_block.get_previous_block_hash(), [1; 32]);
        assert_eq!(deserialized_block.get_creator(), [2; 33]);
        assert_eq!(deserialized_block.get_merkle_root(), [3; 32]);
        assert_eq!(deserialized_block.get_signature(), [4; 64]);
        assert_eq!(deserialized_block.get_treasury(), 1);
        assert_eq!(deserialized_block.get_burnfee(), 2);
        assert_eq!(deserialized_block.get_difficulty(), 3);

        assert_eq!(
            deserialized_block_header.serialize_for_net(BlockType::Full),
            deserialized_block.serialize_for_net(BlockType::Header)
        );
        assert_eq!(deserialized_block_header.get_id(), 1);
        assert_eq!(deserialized_block_header.get_timestamp(), timestamp);
        assert_eq!(deserialized_block_header.get_previous_block_hash(), [1; 32]);
        assert_eq!(deserialized_block_header.get_creator(), [2; 33]);
        assert_eq!(deserialized_block_header.get_merkle_root(), [3; 32]);
        assert_eq!(deserialized_block_header.get_signature(), [4; 64]);
        assert_eq!(deserialized_block_header.get_treasury(), 1);
        assert_eq!(deserialized_block_header.get_burnfee(), 2);
        assert_eq!(deserialized_block_header.get_difficulty(), 3);

        TestManager::check_block_consistency(&block);
        TestManager::check_block_consistency(&deserialized_block);
        TestManager::check_block_consistency(&deserialized_block_header);
    }

    #[test]
    // confirm merkle root is being generated from transactions in block
    fn block_merkle_root_test() {
        let mut block = Block::new();
        let wallet = Wallet::new();

        let mut transactions = (0..5)
            .into_iter()
            .map(|_| {
                let mut transaction = Transaction::new();
                transaction.sign(wallet.get_privatekey());
                transaction
            })
            .collect();

        block.set_transactions(&mut transactions);
        block.set_merkle_root(block.generate_merkle_root());

        assert!(block.get_merkle_root().len() == 32);
        assert_ne!(block.get_merkle_root(), [0; 32]);

        TestManager::check_block_consistency(&block);
    }

    #[tokio::test]
    // downgrade and upgrade a block with transactions
    async fn block_downgrade_upgrade_test() {
        let mut block = Block::new();
        let wallet = Wallet::new();
        let mut transactions = (0..5)
            .into_iter()
            .map(|_| {
                let mut transaction = Transaction::new();
                transaction.sign(wallet.get_privatekey());
                transaction
            })
            .collect();
        block.set_transactions(&mut transactions);

        Storage::write_block_to_disk(&mut block);

        assert_eq!(block.transactions.len(), 5);
        assert_eq!(block.get_block_type(), BlockType::Full);

        let serialized_full_block = block.serialize_for_net(BlockType::Full);

        block.downgrade_block_to_block_type(BlockType::Pruned).await;

        assert_eq!(block.transactions.len(), 0);
        assert_eq!(block.get_block_type(), BlockType::Pruned);

        block.upgrade_block_to_block_type(BlockType::Full).await;

        assert_eq!(block.get_block_type(), BlockType::Full);
        assert_eq!(
            serialized_full_block,
            block.serialize_for_net(BlockType::Full)
        );

        TestManager::check_block_consistency(&block);
    }
}
