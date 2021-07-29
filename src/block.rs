use crate::{
    blockchain::Blockchain,
    burnfee::BurnFee,
    crypto::{
        hash, sign, SaitoHash, SaitoPrivateKey, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey,
    },
    golden_ticket::GoldenTicket,
    hop::HOP_SIZE,
    merkle::MerkleTreeLayer,
    slip::{Slip, SlipType, SLIP_SIZE},
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

//
// object used when generating and validation transactions, containing the
// information that is created selectively according to the transaction fees
// and the optional outbound payments.
//
#[derive(PartialEq, Debug, Clone)]
pub struct ConsensusValues {
    // expected transaction containing outbound payments
    pub fee_transaction: Option<Transaction>,
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
}
impl ConsensusValues {
    #[allow(clippy::too_many_arguments)]
    pub fn new() -> ConsensusValues {
        ConsensusValues {
            fee_transaction: None,
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
        }
    }
}

//
// The BlockPayout object is returned by the function that calculates
// who deserves the payment for each block.
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
    /// Transactions
    pub transactions: Vec<Transaction>,
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
    // has fee transaction
    pub has_fee_transaction: bool,
    // number of rebroadcast slips
    pub total_rebroadcast_slips: u64,
    // number of rebroadcast txs
    pub total_rebroadcast_nolan: u64,
    // all ATR txs hashed together
    pub rebroadcast_hash: [u8; 32],
}

impl Block {
    #[allow(clippy::clippy::new_without_default)]
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
            transactions: vec![],
            hash: [0; 32],
            total_fees: 0,
            routing_work_for_creator: 0,
            lc: false,
            has_golden_ticket: false,
            has_fee_transaction: false,
            total_rebroadcast_slips: 0,
            total_rebroadcast_nolan: 0,
            // must be initialized zeroed-out for proper hashing
            rebroadcast_hash: [0; 32],
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

    pub fn get_burnfee(&self) -> u64 {
        self.burnfee
    }

    pub fn get_difficulty(&self) -> u64 {
        self.difficulty
    }

    pub fn get_has_golden_ticket(&self) -> bool {
        self.has_golden_ticket
    }

    pub fn get_has_fee_transaction(&self) -> bool {
        self.has_fee_transaction
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

    pub fn set_has_golden_ticket(&mut self, hgt: bool) {
        self.has_golden_ticket = hgt;
    }

    pub fn set_has_fee_transaction(&mut self, hft: bool) {
        self.has_fee_transaction = hft;
    }

    pub fn set_total_fees(&mut self, total_fees: u64) {
        self.total_fees = total_fees;
    }

    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
        self.transactions = transactions.to_vec();
    }

    //pub fn set_transactions(&mut self, transactions: Vec<Transaction>) {
    //    self.transactions = transactions;
    //}

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

    pub fn set_treasury(&mut self, treasury: u64) {
        self.treasury = treasury;
    }

    pub fn set_burnfee(&mut self, burnfee: u64) {
        self.burnfee = burnfee;
    }

    pub fn set_difficulty(&mut self, difficulty: u64) {
        self.difficulty = difficulty;
    }

    pub fn set_hash(&mut self, hash: SaitoHash) {
        self.hash = hash;
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn sign(&mut self, publickey: SaitoPublicKey, privatekey: SaitoPrivateKey) {
        //
        // we set final data
        //
        self.set_creator(publickey);

        let hash_for_signature = hash(&self.serialize_for_signature());
        self.set_hash(hash_for_signature);

        self.set_signature(sign(&hash_for_signature, privatekey));
    }

    //
    // TODO
    //
    // hash is nor being serialized from the right data - requires
    // merkle_root as an input into the hash, and that is not yet
    // supported. this is a stub that uses the timestamp and the
    // id -- it exists so each block will still have a unique hash
    // for blockchain functions.
    //
    pub fn generate_hash(&self) -> SaitoHash {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.id.to_be_bytes());
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&self.previous_block_hash);
        vbytes.extend(&self.creator);
        vbytes.extend(&self.merkle_root);
        vbytes.extend(&self.signature);
        vbytes.extend(&self.treasury.to_be_bytes());
        vbytes.extend(&self.burnfee.to_be_bytes());
        vbytes.extend(&self.difficulty.to_be_bytes());

        hash(&vbytes)
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
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.transactions.iter().len() as u32).to_be_bytes());
        vbytes.extend(&self.id.to_be_bytes());
        vbytes.extend(&self.timestamp.to_be_bytes());
        vbytes.extend(&self.previous_block_hash);
        vbytes.extend(&self.creator);
        vbytes.extend(&self.merkle_root);
        vbytes.extend(&self.signature);
        vbytes.extend(&self.treasury.to_be_bytes());
        vbytes.extend(&self.burnfee.to_be_bytes());
        vbytes.extend(&self.difficulty.to_be_bytes());
        let mut serialized_txs = vec![];
        self.transactions.iter().for_each(|transaction| {
            serialized_txs.extend(transaction.serialize_for_net());
        });
        vbytes.extend(serialized_txs);
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
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn deserialize_for_net(bytes: Vec<u8>) -> Block {
        let transactions_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let id: u64 = u64::from_be_bytes(bytes[4..12].try_into().unwrap());
        let timestamp: u64 = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
        let previous_block_hash: SaitoHash = bytes[20..52].try_into().unwrap();
        let creator: SaitoPublicKey = bytes[52..85].try_into().unwrap();
        let merkle_root: SaitoHash = bytes[85..117].try_into().unwrap();
        let signature: SaitoSignature = bytes[117..181].try_into().unwrap();

        let treasury: u64 = u64::from_be_bytes(bytes[181..189].try_into().unwrap());
        let burnfee: u64 = u64::from_be_bytes(bytes[189..197].try_into().unwrap());
        let difficulty: u64 = u64::from_be_bytes(bytes[197..205].try_into().unwrap());
        let mut transactions = vec![];
        let mut start_of_transaction_data = 205;
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

        block.set_transactions(&mut transactions);
        block
    }

    //
    // TODO - this logic should probably be in the merkle-root class
    //
    pub fn generate_merkle_root(&self) -> SaitoHash {
        if self.transactions.len() == 0 {
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
    pub fn generate_consensus_values(&self, blockchain: &Blockchain) -> ConsensusValues {
        let mut cv = ConsensusValues::new();

        //
        // calculate total fees
        //
        // calculate fee and golden ticket indices
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
        }

        //
        // calculate automatic transaction rebroadcasts / ATR / atr
        //
        if self.get_id() > 2 {
            let pruned_block_hash = blockchain
                .blockring
                .get_longest_chain_block_hash_by_block_id(self.get_id() - 2);

            println!("pruned block hash: {:?}", pruned_block_hash);

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
println!("GENERATING REBROADCAST TX: {:?}", transaction.get_transaction_type());
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
                                // dust is collected as fee
                                //
                                cv.total_rebroadcast_fees_nolan += output.get_amount();
                            }
                        }
                    }
                }
            }
        }

        //
        // calculate fee transaction
        //
        if let Some(gt_idx) = cv.gt_idx {
            //
            // grab random input from golden ticket
            //
            let golden_ticket: GoldenTicket = GoldenTicket::deserialize_for_transaction(
                self.transactions[gt_idx].get_message().to_vec(),
            );
            let random_number = golden_ticket.get_random();
            let miner_publickey = golden_ticket.get_publickey();

            //
            // payout is from last block
            //
            if let Some(previous_block) = blockchain.blocks.get(&self.get_previous_block_hash()) {
                let mut transaction = Transaction::new();
                transaction.set_transaction_type(TransactionType::Fee);

                //
                // calculate miner and router payments
                //
                let miner_payment = previous_block.get_total_fees() / 2;
                let router_payment = previous_block.get_total_fees() - miner_payment;

                let block_payouts: RouterPayout = previous_block.find_winning_router(random_number);
                let router_publickey = block_payouts.publickey;
                let _next_random_number = block_payouts.random_number;

                let mut output1 = Slip::new();
                output1.set_publickey(miner_publickey);
                output1.set_amount(miner_payment);
                output1.set_slip_type(SlipType::MinerOutput);
                output1.set_slip_ordinal(0);

                let mut output2 = Slip::new();
                output2.set_publickey(router_publickey);
                output2.set_amount(router_payment);
                output2.set_slip_type(SlipType::RouterOutput);
                output2.set_slip_ordinal(1);

                transaction.add_output(output1);
                transaction.add_output(output2);

                //
                // fee transaction added to consensus values
                //
                cv.fee_transaction = Some(transaction);
            }
        }

        cv
    }

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
            rp.random_number = random_number;
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
        let random_number2 = hash(&random_number.to_vec());

        rp.publickey = winning_tx.get_winning_routing_node(random_number2);
        rp.random_number = hash(&random_number2.to_vec());

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
        println!(" ... block.prevalid - pre hash:  {:?}", create_timestamp());

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

        println!(" ... block.prevalid - pst hash:  {:?}", create_timestamp());

        //
        // CUMULATIVE FEES only AFTER parallel calculations
        //
        // we need to calculate the cumulative figures AFTER the
        // transactions have been fleshed out with all of the
        // original figures.
        //
        let mut cumulative_fees = 0;
        let mut cumulative_work = 0;

        let mut has_golden_ticket = false;
        let mut has_fee_transaction = false;

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
        for transaction in &mut self.transactions {
            cumulative_fees = transaction.generate_metadata_cumulative_fees(cumulative_fees);
            cumulative_work = transaction.generate_metadata_cumulative_work(cumulative_work);

            //
            // also check the transactions for golden ticket and fees
            //
            match transaction.get_transaction_type() {
                TransactionType::Fee => has_fee_transaction = true,
                TransactionType::GoldenTicket => has_golden_ticket = true,
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

        //
        // update block with total fees
        //
        self.set_total_fees(cumulative_fees);
        self.set_routing_work_for_creator(cumulative_work);

        println!(" ... block.pre_validation_done:  {:?}", create_timestamp());

        true
    }

    pub fn validate(
        &self,
        blockchain: &Blockchain,
        utxoset: &AHashMap<SaitoUTXOSetKey, u64>,
    ) -> bool {
        //
        // no transactions? no thank you
        //
        if self.transactions.len() == 0 {
            println!("ERROR 424342: block does not validate as it has no transactions");
            return false;
        }

        println!(" ... block.validate: (burn fee)  {:?}", create_timestamp());

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
        let cv = self.generate_consensus_values(&blockchain);

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
            // validate burn fee
            //
            let new_burnfee: u64 =
                BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
                    previous_block.get_burnfee(),
                    self.get_timestamp(),
                    previous_block.get_timestamp(),
                );
            if new_burnfee != self.get_burnfee() {
                println!(
                    "ERROR: burn fee does not validate, expected: {}",
                    new_burnfee
                );
                return false;
            }

            println!(" ... burn fee in blk validated:  {:?}", create_timestamp());

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
                println!("Error 510293: block lacking adequate routing work from creator");
                return false;
            }

            println!(" ... done routing work required: {:?}", create_timestamp());

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
                    println!("ERROR: Golden Ticket solution does not validate against previous block hash and difficulty");
                    return false;
                }
            }
            println!(" ... golden ticket: (validated)  {:?}", create_timestamp());
        }

        println!(" ... block.validate: (merkle rt) {:?}", create_timestamp());

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
            println!("ERROR 624442: rebroadcast slips total incorrect");
            return false;
        }
        if cv.total_rebroadcast_nolan != self.total_rebroadcast_nolan {
            println!("ERROR 294018: rebroadcast nolan amount incorrect");
            return false;
        }
        if cv.rebroadcast_hash != self.rebroadcast_hash {
            println!("ERROR 123422: hash of rebroadcast transactions incorrect");
            return false;
        }

        //
        // validate merkle root
        //
        if self.get_merkle_root() == [0; 32]
            && self.get_merkle_root() != self.generate_merkle_root()
        {
            println!("merkle root is unset or is invalid false 1");
            return false;
        }

        println!(" ... block.validate: (cv-data)   {:?}", create_timestamp());

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
                println!(
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

            //println!("CV: {:?}", fee_transaction);
            //println!("BLK: {:?}", self.transactions[ft_idx]);

            let hash1 = hash(&fee_transaction.serialize_for_signature());
            let hash2 = hash(&self.transactions[ft_idx].serialize_for_signature());
            if hash1 != hash2 {
                println!("ERROR 627428: block fee transaction doesn't match cv fee transaction");
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
            println!(
                "difficulty is false {} vs {}",
                cv.expected_difficulty,
                self.get_difficulty()
            );
            return false;
        }

        println!(" ... block.validate: (txs valid) {:?}", create_timestamp());

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
        let transactions_valid = self.transactions.par_iter().all(|tx| tx.validate(utxoset));

        println!(" ... block.validate: (done all)  {:?}", create_timestamp());

        //
        // and if our transactions are valid, so is the block...
        //
        println!(" ... are txs valid: {}", transactions_valid);
        transactions_valid
    }

    pub async fn generate(
        transactions: &mut Vec<Transaction>,
        previous_block_hash: SaitoHash,
        wallet_lock: Arc<RwLock<Wallet>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> Block {
        let current_timestamp = create_timestamp();
        return Block::generate_with_timestamp(
            transactions,
            previous_block_hash,
            wallet_lock,
            blockchain_lock,
            current_timestamp,
        )
        .await;
    }

    pub async fn generate_with_timestamp(
        transactions: &mut Vec<Transaction>,
        previous_block_hash: SaitoHash,
        wallet_lock: Arc<RwLock<Wallet>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        current_timestamp: u64,
    ) -> Block {
        let blockchain = blockchain_lock.read().await;

        let wallet = wallet_lock.read().await;
        let publickey = wallet.get_publickey();
        let privatekey = wallet.get_privatekey();

        let mut previous_block_id = 0;
        let mut previous_block_burnfee = 0;
        let mut previous_block_timestamp = 0;
        let mut previous_block_difficulty = 0;

        if let Some(previous_block) = blockchain.blocks.get(&previous_block_hash) {
            previous_block_id = previous_block.get_id();
            previous_block_burnfee = previous_block.get_burnfee();
            previous_block_timestamp = previous_block.get_timestamp();
            previous_block_difficulty = previous_block.get_difficulty();
        }

        let mut block = Block::new();
        block.set_timestamp(current_timestamp);

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
        // contextual values
        //
        let mut cv: ConsensusValues = block.generate_consensus_values(&blockchain);

        //
        // TODO - remove
        //
        // for testing create some VIP transactions
        //
        if previous_block_id == 0 {
            for i in 0..10 as i32 {
                println!("generating VIP transaction {}", i);
                let mut transaction =
                    Transaction::generate_vip_transaction(wallet_lock.clone(), publickey, 100000)
                        .await;
                transaction.sign(privatekey);
                block.add_transaction(transaction);
            }
        }

        //
        // ATR transactions
        //
        let rlen = cv.rebroadcasts.len();
        // TODO -- delete if pos
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
        if !cv.fee_transaction.is_none() {
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
        // validate difficulty
        //
        if cv.expected_difficulty != 0 {
            block.set_difficulty(cv.expected_difficulty);
        }

        //
        // generate merkle root
        //
        let block_merkle_root = block.generate_merkle_root();
        block.set_merkle_root(block_merkle_root);

        let block_hash = block.generate_hash();
        block.set_hash(block_hash);

        block.sign(wallet.get_publickey(), wallet.get_privatekey());

        block
    }
}

//
// TODO
//
// temporary data-serialization of blocks so that we can save
// to disk. These should only be called through the serialization
// functions within the block class, so that all access is
// compartmentalized and we can move to custom serialization
//
impl From<Vec<u8>> for Block {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for Block {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[cfg(test)]

mod tests {

    use super::*;
    use crate::{
        slip::Slip,
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
    }

    #[test]
    fn block_sign_test() {
        let wallet = Wallet::new();
        let mut block = Block::new();

        block.sign(wallet.get_publickey(), wallet.get_privatekey());

        assert_eq!(block.creator, wallet.get_publickey());
        assert_ne!(block.get_hash(), [0; 32]);
        assert_ne!(block.get_signature(), [0; 64]);
    }

    #[test]
    fn block_generate_hash() {
        let block = Block::new();
        let hash = block.generate_hash();
        assert_ne!(hash, [0; 32]);
    }

    #[test]
    fn block_serialize_for_signature_hash() {
        let block = Block::new();
        let serialized_body = block.serialize_for_signature();
        assert_eq!(serialized_body.len(), 137);
    }

    #[test]
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

        let serialized_block = block.serialize_for_net();
        let deserialized_block = Block::deserialize_for_net(serialized_block);
        assert_eq!(block, deserialized_block);
        assert_eq!(deserialized_block.get_id(), 1);
        assert_eq!(deserialized_block.get_timestamp(), timestamp);
        assert_eq!(deserialized_block.get_previous_block_hash(), [1; 32]);
        assert_eq!(deserialized_block.get_creator(), [2; 33]);
        assert_eq!(deserialized_block.get_merkle_root(), [3; 32]);
        assert_eq!(deserialized_block.get_signature(), [4; 64]);
        assert_eq!(deserialized_block.get_treasury(), 1);
        assert_eq!(deserialized_block.get_burnfee(), 2);
        assert_eq!(deserialized_block.get_difficulty(), 3);
    }

    #[test]
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

        assert!(block.generate_merkle_root().len() == 32);
    }

    #[test]
    fn block_pre_validateion_calculations() {}

    #[test]
    fn block_onchain_reorganization_test() {}

    #[test]
    fn block_validation() {}
}
