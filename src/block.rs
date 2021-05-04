//use std::stream::Stream;
//use futures::io::AsyncRead;

use std::mem::transmute;
use serde::{Serialize, Deserialize};

use data_encoding::HEXLOWER;

use crate::burnfee::BurnFee;
use crate::crypto::{PublicKey, hash};
use crate::transaction::Transaction;

use crate::helper::create_timestamp;
/// Our Block object
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    pub body:     BlockBody,
    pub is_valid: u8,
    mintid:   u32,
    maxtid:   u32,
    bsh:      [u8; 32],
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockBody {
    pub id:          u32,
    pub ts:          u64,
    pub prevbsh:     [u8; 32],
    pub creator:     PublicKey,
    pub txs:         Vec<Transaction>,
    pub bf:	         BurnFee,
    merkle:          [u8; 32],
    difficulty:      f32,
    paysplit:        f32,
    vote:            i8,
    treasury:        u64,
    coinbase:        u64,
    reclaimed:       u64
}

impl BlockBody {
    pub fn new(block_creator: PublicKey, prevbsh: [u8;32]) -> BlockBody {
        return BlockBody {
    	    id:          0,
    	    ts:          create_timestamp(),
    	    prevbsh:     prevbsh,
    	    merkle:      [0; 32],
    	    creator:     block_creator,
    	    txs:         vec![],
	        bf:          BurnFee::new(0.0, 0),
    	    difficulty:  1.0,
    	    paysplit:    0.5,
    	    vote:        0,
    	    treasury:    286_810_000_000_000_000,
    	    coinbase:    0,
    	    reclaimed:   0
        };
    }
}

impl Block {
    pub fn new(creator: PublicKey, prevbsh: [u8;32]) -> Block {
        return Block {
            body:      BlockBody::new(creator, prevbsh),
            is_valid:  1,
            mintid:    0,
            maxtid:    0,
            bsh:       [0; 32],
        };
    }

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

    pub fn get_tx_count(&self) -> usize {
        return self.body.txs.len();
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.body.txs.push(tx);
    }

    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
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

    pub fn get_bsh_as_hex(&self) -> String {
        return HEXLOWER.encode(&self.get_bsh());
    }

    pub fn get_creator(&self) -> PublicKey {
        return self.body.creator;
    }

    pub fn get_mintid(&self) -> u32 {
        return self.mintid;
    }

    pub fn get_maxtid(&self) -> u32 {
        return self.maxtid;
    }

    pub fn get_paid_burnfee(&self) -> u64 {
        return self.body.bf.current;
    }

    pub fn get_difficulty(&self) -> f32 {
        return self.body.difficulty;
    }

    pub fn get_paysplit(&self) -> f32 {
        return self.body.paysplit;
    }

    pub fn get_coinbase(&self) -> u64 {
        return self.body.coinbase;
    }

    pub fn get_available_fees(&self, publickey: &PublicKey) -> u64 {
        return self.body.txs
            .iter()
            .map(|tx| tx.get_fees_usable(publickey))
            .sum();
    }

    pub fn set_id(&mut self, id: u32) {
        self.body.id = id;
    }

    pub fn set_burnfee(&mut self, bf: BurnFee) {
        self.body.bf = bf;
    }

    pub fn set_mintid(&mut self, mintid: u32) {
        self.mintid = mintid;
    }

    pub fn set_maxtid(&mut self, maxtid: u32) {
        self.maxtid = maxtid;
    }

    pub fn set_treasury(&mut self, treasury: u64) {
        self.body.treasury = treasury;
    }

    pub fn set_prevhash(&mut self, prevbsh: [u8; 32]) {
        self.body.prevbsh = prevbsh;
    }

    pub fn set_difficulty(&mut self, difficulty: f32) {
        self.body.difficulty = difficulty;
    }

    pub fn set_paysplit(&mut self, paysplit: f32) {
        self.body.paysplit = paysplit;
    }

    pub fn set_coinbase(&mut self, coinbase: u64) {
        self.body.coinbase = coinbase;
    }

    pub fn set_reclaimed(&mut self, reclaimed: u64) {
        self.body.reclaimed = reclaimed;
    }
    // fn getTransactionByHash(hash) -> Transaction
    // fn getTransactionById(int id) -> Transaction
    // fn hasTransaction(String hash) -> bool
    // fn attemptToFindDifficultHash() -> Option<String>
}
//
// Block Header (for index)
//
// the contents of this data object represent the information
// about the block itself that is stored in the blockchain
// index. it is used primarily when rolling / unrolling the
// blockchain.
//
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockHeader {
    pub bsh: [u8;32],
    pub prevbsh: [u8;32],
    pub bid: u32,
    pub ts:  u64,
    pub bf: BurnFee,
    pub mintid: u32,
    pub maxtid: u32,
    pub difficulty: f32,
    pub paysplit: f32,
    pub vote: i8,
    pub treasury: u64,
    pub coinbase: u64,
    pub reclaimed: u64,
}


impl BlockHeader {
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

