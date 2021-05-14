use std::mem::transmute;

use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::Transaction;

/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    /// The body and content of the block object
    pub body: BlockBody,
    /// Byte array hash of the block
    bsh: [u8; 32],
}
/// This `BlockBody` holds data to be serialized along with
/// `Transaction`s
#[derive(PartialEq, Debug, Clone)]
pub struct BlockBody {
    /// Block id
    pub id: u64,
    /// Block timestamp
    pub ts: u64,
    /// Byte array hash of the previous block in the chain
    pub previous_block_hash: [u8; 32],
    /// `Publickey` of the block creator
    pub creator: PublicKey,
    /// List of transactions in the block
    pub txs: Vec<Transaction>,
    /// `BurnFee` containing the fees paid to produce the block
    pub burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// Ratio determing the block reward split between nodes and miners
    paysplit: f32,
    /// Vote determining change of difficulty and paysplit
    vote: i8,
    /// Treasury of existing Saito in the network
    treasury: u64,
    /// Total block reward being released in the block
    coinbase: u64,
}

impl BlockBody {
    /// Creates a new `BlockBody`
    ///
    /// * `block_creator` - `secp256k1::PublicKey` of the block creator
    /// * `prevbsh` - Previous block hash in bytes
    pub fn new(block_creator: PublicKey, prevbsh: [u8; 32]) -> BlockBody {
        return BlockBody {
            id: 0,
            ts: create_timestamp(),
            previous_block_hash: prevbsh,
            creator: block_creator,
            txs: vec![],
            burnfee: 0,
            difficulty: 0.0,
            paysplit: 0.5,
            vote: 0,
            treasury: 286_810_000_000_000_000,
            coinbase: 0,
        };
    }
}

impl Block {
    /// Receives the a publickey and the previous block hash
    ///
    /// * `block_creator` - `secp256k1::PublicKey` of the block creator
    /// * `prevbsh` - Previous block hash in bytes
    pub fn new(creator: PublicKey, prevbsh: [u8; 32]) -> Block {
        return Block {
            body: BlockBody::new(creator, prevbsh),
            bsh: [0; 32],
        };
    }

    /// Creates a block solely from the block body. Used when
    /// deserializing a block either from disk or from the network
    ///
    /// * `body` - `BlockBody` of new `Block`
    pub fn from_block_body(body: BlockBody) -> Block {
        return Block {
            body: body,
            bsh: [0; 32],
        };
    }

    /// Returns the `Block` id
    pub fn id(&self) -> u64 {
        self.body.id
    }

    /// Returns the `Block` timestamp
    pub fn timestamp(&self) -> u64 {
        self.body.ts
    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> [u8; 32] {
        self.body.previous_block_hash
    }

    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn creator(&self) -> PublicKey {
        self.body.creator
    }

    /// Returns the `Block`'s `Transaction`s
    pub fn txs(&self) -> &Vec<Transaction> {
        &self.body.txs
    }

    /// Returns the `Block` burnfee
    pub fn burnfee(&self) -> u64 {
        self.body.burnfee
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.body.difficulty
    }

    /// Returns the `Block` vote
    pub fn vote(&self) -> i8 {
        self.body.vote
    }

    /// Returns the `Block` paysplit
    pub fn paysplit(&self) -> f32 {
        self.body.paysplit
    }

    /// Returns the `Block` treasury
    pub fn treasury(&self) -> u64 {
        self.body.treasury
    }

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.body.coinbase
    }

    /// Generate the block hash
    ///
    /// TODO -- extend list of information we use to calculate the block hash
    pub fn block_hash(&self) -> [u8; 32] {
        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = unsafe { transmute(self.body.id.to_be()) };
        let ts_bytes: [u8; 8] = unsafe { transmute(self.body.ts.to_be()) };
        let cr_bytes: Vec<u8> = self.body.creator.serialize().iter().cloned().collect();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);
        data.extend(&cr_bytes);

        return hash(data);
    }

    /// Converts our blockhash from a byte array into a hex string
    pub fn block_hash_hex(&self) -> String {
        hex::encode(&self.block_hash())
    }

    /// Swaps a list of transactions already in memory into blocks transactions
    ///
    /// The transactions are given the proper ids in reference to other transactions
    /// already on chain, as well as setting the proper metadata for each slip in
    /// each transaction
    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
        // Memory swap of transactions so we don't have to copy large amounts of data twice
        std::mem::swap(&mut self.body.txs, transactions);

        // let maxtid = self.maxtid;
        let bid = self.body.id;
        let bsh = self.block_hash();

        for (i, tx) in self.body.txs.iter_mut().enumerate() {
            let mut current_sid = 0;

            let to_slips = tx
                .to_slips()
                .iter_mut()
                .map(move |slip| {
                    slip.set_block_id(0);
                    slip.set_tx_id(0);
                    slip.set_slip_id(current_sid);
                    current_sid += 1;
                    return slip.clone();
                })
                .collect();
            tx.set_to_slips(to_slips);

            let from_slips = tx
                .from_slips()
                .iter_mut()
                .map(move |slip| {
                    slip.set_block_id(bid);
                    slip.set_tx_id(i as u64);
                    slip.set_slip_id(current_sid);
                    slip.set_block_hash(bsh);
                    current_sid += 1;
                    return slip.clone();
                })
                .collect();
            tx.set_from_slips(from_slips);
        }
    }

    /// Appends a transaction to the block
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.body.txs.push(tx);
    }

    /// Sets the id of the block
    pub fn set_id(&mut self, id: u64) {
        self.body.id = id;
    }

    /// Sets the `Block` burnfee
    pub fn set_burnfee(&mut self, bf: u64) {
        self.body.burnfee = bf;
    }

    /// Sets the `Block` previous hash
    pub fn set_previous_block_hash(&mut self, prevbsh: [u8; 32]) {
        self.body.previous_block_hash = prevbsh;
    }

    /// Sets the `Block` vote
    pub fn set_vote(&mut self, vote: i8) {
        self.body.vote = vote;
    }

    /// Sets the `Block` difficulty
    pub fn set_difficulty(&mut self, difficulty: f32) {
        self.body.difficulty = difficulty;
    }

    /// Sets the `Block` paysplit
    pub fn set_paysplit(&mut self, paysplit: f32) {
        self.body.paysplit = paysplit;
    }

    /// Sets the `Block` treasury
    pub fn set_treasury(&mut self, treasury: u64) {
        self.body.treasury = treasury;
    }

    /// Sets the `Block` coinbase
    pub fn set_coinbase(&mut self, coinbase: u64) {
        self.body.coinbase = coinbase;
    }
}

// Module std::stream
#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        keypair::Keypair,
        transaction::{Transaction, TransactionBroadcastType},
    };

    #[test]
    fn block_test() {
        let keypair = Keypair::new();
        let mut block = Block::new(*keypair.public_key(), [0; 32]);

        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), [0; 32]);
        assert_eq!(block.creator(), *keypair.public_key());
        assert_eq!(*block.txs(), vec![]);
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.paysplit(), 0.5);
        assert_eq!(block.vote(), 0);
        assert_eq!(block.treasury(), 286_810_000_000_000_000);
        assert_eq!(block.coinbase(), 0);

        block.set_id(1);
        assert_eq!(block.id(), 1);

        block.set_burnfee(10_000);
        assert_eq!(block.burnfee(), 10_000);

        block.set_difficulty(5.0);
        assert_eq!(block.difficulty(), 5.0);

        block.set_paysplit(0.75);
        assert_eq!(block.paysplit(), 0.75);

        block.set_vote(1);
        assert_eq!(block.vote(), 1);

        block.set_vote(-1);
        assert_eq!(block.vote(), -1);

        block.set_treasury(1_000_000_000);
        assert_eq!(block.treasury(), 1_000_000_000);

        block.set_coinbase(100_000);
        assert_eq!(block.coinbase(), 100_000);

        let tx = Transaction::new(TransactionBroadcastType::Normal);
        block.set_transactions(&mut vec![tx.clone()]);
        assert_eq!(*block.txs(), vec![tx.clone()]);

        let tx2 = Transaction::new(TransactionBroadcastType::Normal);
        block.add_transaction(tx2.clone());
        assert_eq!(*block.txs(), vec![tx.clone(), tx2.clone()])
    }
}
