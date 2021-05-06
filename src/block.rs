//use std::stream::Stream;
//use futures::io::AsyncRead;

use std::mem::transmute;
use serde::{Serialize, Deserialize};

use data_encoding::HEXLOWER;

use crate::burnfee::BurnFee;
use crate::crypto::{PublicKey, hash};
use crate::transaction::Transaction;

use crate::helper::create_timestamp;
/// The block holds all data inside the block body,
/// providing high level checks that the blockchain class
/// be concerned about
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// The body and content of the block object
    pub body:     BlockBody,
    /// Cached value to save the validity of the block
    pub is_valid: u8,
    /// The minimum transaction id allowed to be added to `Blockchain`
    mintid:   u32,
    /// The maximum transaction id allowed to be added to `Blockchain`
    maxtid:   u32,
    /// Byte array hash of the block
    bsh:      [u8; 32],
}
/// This BlockBody holds all of the necssary consensus data
/// and transactions
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockBody {
    /// Block id
    pub id:          u32,
    /// Block timestamp
    pub ts:          u64,
    /// Byte array hash of the previous block in the chain
    pub prevbsh:     [u8; 32],
    /// `Publickey` of the block creator
    pub creator:     PublicKey,
    /// List of transactions in the block
    pub txs:         Vec<Transaction>,
    /// `BurnFee` containing the fees paid to produce the block
    pub bf:	         BurnFee,
    /// Byte array of merkle tree hash
    merkle:          [u8; 32],
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty:      f32,
    /// Ratio determing the block reward split between nodes and miners
    paysplit:        f32,
    /// Vote determining change of difficulty and paysplit
    vote:            i8,
    /// Treasury of existing Saito in the network
    treasury:        u64,
    /// Total block reward being released in the block
    coinbase:        u64,
    /// Fees reclaimed from block
    reclaimed:       u64
}

impl BlockBody {
    /// Receives the a publickey and the previous block hash
    pub fn new(block_creator: PublicKey, prevbsh: [u8;32]) -> BlockBody {
        return BlockBody {
    	    id:          0,
    	    ts:          create_timestamp(),
    	    prevbsh:     prevbsh,
    	    merkle:      [0; 32],
    	    creator:     block_creator,
    	    txs:         vec![],
	        bf:          BurnFee::new(0.0, 0),
    	    difficulty:  2.1,
    	    paysplit:    0.5,
    	    vote:        0,
    	    treasury:    286_810_000_000_000_000,
    	    coinbase:    0,
    	    reclaimed:   0
        };
    }
}

impl Block {
    /// Receives the a publickey and the previous block hash
    pub fn new(creator: PublicKey, prevbsh: [u8;32]) -> Block {
        return Block {
            body:      BlockBody::new(creator, prevbsh),
            is_valid:  1,
            mintid:    0,
            maxtid:    0,
            bsh:       [0; 32],
        };
    }

    /// Creates a block solely from the block body. Used when
    /// deserializing a block either from disk or from the network
    pub fn create_from_block_body(body: BlockBody) -> Block {
        return Block {
            body:      body,
            is_valid:  1,
            mintid:    0,
            maxtid:    0,
            bsh:       [0; 32],
        };
    }

    /// Creates the high level block infromation without passing all associated
    /// data in block contained in the transactions
    pub fn header(&self) -> BlockHeader {
        return BlockHeader::new(
            self.get_bsh(),
            self.body.prevbsh,
            self.body.id,
            self.body.ts,
            self.body.bf.clone(),
            self.mintid,
            self.maxtid,
            self.body.difficulty,
            self.body.paysplit,
            self.body.vote,
            self.body.treasury,
            self.body.coinbase,
            self.body.reclaimed,
        );
    }

    // pub fn get_transactions(&mut self, tx: Transaction) -> Vec<Transaction> {
    //     return self.body.txs;
    // }

    /// Returns the amount of transactions found inside the block
    pub fn get_tx_count(&self) -> usize {
        return self.body.txs.len();
    }

    /// Appends a transaction to the block
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.body.txs.push(tx);
    }

    /// Swaps a list of transactions already in memory into blocks transactions
    ///
    /// The transactions are given the proper ids in reference to other transactions
    /// already on chain, as well as setting the proper metadata for each slip in
    /// each transaction
    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {

        // Memory swap of transactions so we don't have to copy large amounts of data twice
        std::mem::swap(&mut self.body.txs, transactions);

        let tx_length = self.body.txs.len();
        let maxtid = self.maxtid;
        let bid = self.body.id;
        let bsh = self.get_bsh();

        // used for calculating cumulative fees
        let mut cumulative_fees = 0;

        for (i, tx) in self.body.txs.iter_mut().enumerate() {
            let current_tid = maxtid + i as u32 + 1;
            let mut current_sid = 0;

            // set tx id
            tx.set_id(current_tid);

            let to_slips = tx.get_to_slips()
                .iter_mut()
                .map(move |slip| {
                    slip.set_ids(0, 0, current_sid);
                    current_sid += 1;
                    return slip.clone();
                })
                .collect();

            tx.set_to_slips(to_slips);

            let from_slips = tx.get_from_slips()
                .iter_mut()
                .map(move |slip| {
                    slip.set_ids(bid, current_tid, current_sid);
                    slip.set_bsh(bsh);
                    current_sid += 1;
                    return slip.clone();
                })
                .collect();

            tx.set_from_slips(from_slips);

            // calculate cumulative fees
            cumulative_fees = tx.calculate_cumulative_fees(cumulative_fees);
        }

        self.mintid = self.maxtid + 1;
        self.maxtid = self.maxtid + tx_length  as u32;
    }

    /// Create our block hash
    ///
    /// TODO -- extend list of information we use to calculate the block hash
    pub fn get_bsh(&self) -> [u8; 32] {
        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 4] = unsafe { transmute(self.body.id.to_be()) };
        let ts_bytes: [u8; 8] = unsafe { transmute(self.body.ts.to_be()) };
        let cr_bytes: Vec<u8> = self.body.creator.serialize().iter().cloned().collect();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);
        data.extend(&cr_bytes);

        let mut output: [u8; 32] = [0; 32];

        hash(data, &mut output);

        return output;
    }

    /// Converts our blockhash from a byte array into a hex string
    pub fn get_bsh_as_hex(&self) -> String {
        return HEXLOWER.encode(&self.get_bsh());
    }

    /// Returns the body of our block
    pub fn get_body(&self) -> &BlockBody {
        return &self.body;
    }

    /// Returns the publickey of the creator of the block
    pub fn get_creator(&self) -> PublicKey {
        return self.body.creator;
    }

    /// Returns the minimum transaction id
    pub fn get_mintid(&self) -> u32 {
        return self.mintid;
    }

    /// Returns the maximum transaction id
    pub fn get_maxtid(&self) -> u32 {
        return self.maxtid;
    }

    /// Returns the burnfee that was paid to create the block
    pub fn get_paid_burnfee(&self) -> u64 {
        return self.body.bf.current;
    }

    /// Returns the difficulty of the block
    pub fn get_difficulty(&self) -> f32 {
        return self.body.difficulty;
    }

    /// Returns the paysplit of the block
    pub fn get_paysplit(&self) -> f32 {
        return self.body.paysplit;
    }

    /// Returns the coinbase of the block
    pub fn get_coinbase(&self) -> u64 {
        return self.body.coinbase;
    }

    /// Calculates the available fees in the blocks
    pub fn get_available_fees(&self, publickey: &PublicKey) -> u64 {
        return self.body.txs
            .iter()
            .map(|tx| tx.get_fees_usable(publickey))
            .sum();
    }

    /// Sets the id of the block
    pub fn set_id(&mut self, id: u32) {
        self.body.id = id;
    }

    /// Sets the `BurnFee` of the block
    pub fn set_burnfee(&mut self, bf: BurnFee) {
        self.body.bf = bf;
    }

    /// Sets the minimum transaction id of the block
    pub fn set_mintid(&mut self, mintid: u32) {
        self.mintid = mintid;
    }

    /// Sets the maxiumum transaction id of the block
    pub fn set_maxtid(&mut self, maxtid: u32) {
        self.maxtid = maxtid;
    }

    /// Sets the treasury transaction id of the block
    pub fn set_treasury(&mut self, treasury: u64) {
        self.body.treasury = treasury;
    }

    /// Sets the previous hash of the block
    pub fn set_prevhash(&mut self, prevbsh: [u8; 32]) {
        self.body.prevbsh = prevbsh;
    }

    /// Sets the difficulty of the block
    pub fn set_difficulty(&mut self, difficulty: f32) {
        self.body.difficulty = difficulty;
    }

    /// Sets the paysplit of the block
    pub fn set_paysplit(&mut self, paysplit: f32) {
        self.body.paysplit = paysplit;
    }

    /// Sets the coinbase of the block
    pub fn set_coinbase(&mut self, coinbase: u64) {
        self.body.coinbase = coinbase;
    }

    /// Sets the reclaimed amount of the block
    pub fn set_reclaimed(&mut self, reclaimed: u64) {
        self.body.reclaimed = reclaimed;
    }
    // fn getTransactionByHash(hash) -> Transaction
    // fn getTransactionById(int id) -> Transaction
    // fn hasTransaction(String hash) -> bool
    // fn attemptToFindDifficultHash() -> Option<String>
}

/// Block Header (for index)
///
/// The contents of this data object represent the information
/// about the block itself that is stored in the blockchain
/// index. it is used primarily when rolling / unrolling the
/// blockchain.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockHeader {
    /// Byte array hash of the block
    pub bsh: [u8;32],
    /// Byte array hash of the previous block in the chain
    pub prevbsh: [u8;32],
    /// Block id
    pub bid: u32,
    /// Block timestamp
    pub ts:  u64,
    /// `BurnFee` containing the fees paid to produce the block
    pub bf: BurnFee,
    /// The minimum transaction id allowed to be added to `Blockchain`
    pub mintid: u32,
    /// The maximum transaction id allowed to be added to `Blockchain`
    pub maxtid: u32,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    pub difficulty: f32,
    /// Ratio determing the block reward split between nodes and miners
    pub paysplit: f32,
    /// Vote determining change of difficulty and paysplit
    pub vote: i8,
    /// Treasury of existing Saito in the network
    pub treasury: u64,
    /// Total block reward being released in the block
    pub coinbase: u64,
    /// Fees reclaimed from block
    pub reclaimed: u64,
}


impl BlockHeader {
    /// Create a summary of the block data without large quantities of data
    pub fn new (
        bsh: [u8;32],
        prevbsh: [u8;32],
        bid: u32,
        ts: u64,
        bf: BurnFee,
        mintid: u32,
        maxtid: u32,
        difficulty: f32,
        paysplit: f32,
        vote: i8,
        treasury: u64,
        coinbase: u64,
        reclaimed: u64
    ) -> BlockHeader {
        return BlockHeader {
            bsh,
            prevbsh,
            bid,
            ts,
            bf,
            mintid,
            maxtid,
            difficulty,
            paysplit,
            vote,
            treasury,
            coinbase,
            reclaimed
        };
    }
}


// Module std::stream

#[cfg(test)]
mod test {
    #[test]
    fn test_new() {
        assert!(false);
    }
}

