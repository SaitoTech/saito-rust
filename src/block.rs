//use std::stream::Stream;
//use futures::io::AsyncRead;

use serde::{Serialize, Deserialize};

use crate::burnfee::BurnFee;

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
    // pub creator:     PublicKey,
    // pub txs:         Vec<Transaction>,
    pub bf:	         BurnFee,
    merkle:          [u8; 32],
    difficulty:      f32,
    paysplit:        f32,
    vote:            i8,
    treasury:        u64,
    coinbase:        u64,
    reclaimed:       u64
}

impl Block {
    // fn new() -> Self {
    //     Block {}
    // }
    // fn deserialize(stream: AsyncRead) -> Block {
    //     Block {}
    // }
    // fn serialize() -> AsyncRead {
    //     let retArr: [u8] = [];
    //     retArr
    // }
    // fn validate()
    // fn addTransaction(Transaction)
    // fn txCount() -> int
    // fn getTransactionByHash(hash) -> Transaction
    // fn getTransactionById(int id) -> Transaction
    // fn getTransactions() -> [Transaction]
    // fn getHash() -> String
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

