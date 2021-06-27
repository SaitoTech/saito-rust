use crate::{
    big_array::BigArray,
    crypto::{hash, SaitoHash, SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey},
    time::create_timestamp,
    transaction::Transaction,
};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

extern crate rayon;
use rayon::prelude::*;

/// BlockCore is a self-contained object containing only the minimum
/// information needed about a block. It exists to simplify block
/// serialization and deserialization until we have custom functions
/// and to.
///
/// This is a private variable. Access to variables within the BlockCore
/// should be handled through getters and setters in the block which
/// surrounds it.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockCore {
    id: u64,
    timestamp: u64,
    previous_block_hash: SaitoHash,
    #[serde(with = "BigArray")]
    creator: SaitoPublicKey, // public key of block creator
    merkle_root: SaitoHash, // merkle root of txs
    #[serde(with = "BigArray")]
    signature: SaitoSignature, // signature of block creator
    treasury: u64,
    burnfee: u64,
    difficulty: u64,
}

impl BlockCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        timestamp: u64,
        previous_block_hash: [u8; 32],
        creator: [u8; 33],
        merkle_root: [u8; 32],
        signature: [u8; 64],
        treasury: u64,
        burnfee: u64,
        difficulty: u64,
    ) -> Self {
        Self {
            id,
            timestamp,
            previous_block_hash,
            creator,
            merkle_root,
            signature,
            treasury,
            burnfee,
            difficulty,
        }
    }
}

impl Default for BlockCore {
    fn default() -> Self {
        Self::new(
            0,
            create_timestamp(),
            [0; 32],
            [0; 33],
            [0; 32],
            [0; 64],
            0,
            0,
            0,
        )
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// Consensus Level Variables
    core: BlockCore,
    /// Transactions
    transactions: Vec<Transaction>,
    /// Self-Calculated / Validated
    hash: SaitoHash,
    /// Is Block on longest chain
    lc: bool,
}

impl Block {
    pub fn new(core: BlockCore) -> Block {
        Block {
            core,
            transactions: vec![],
            hash: [0; 32],
            lc: false,
        }
    }

    pub fn get_hash(&self) -> SaitoHash {
        self.hash
    }

    pub fn get_lc(&self) -> bool {
        self.lc
    }

    pub fn get_id(&self) -> u64 {
        self.core.id
    }

    pub fn get_timestamp(&self) -> u64 {
        self.core.timestamp
    }

    pub fn get_previous_block_hash(&self) -> SaitoHash {
        self.core.previous_block_hash
    }

    pub fn get_creator(&self) -> SaitoPublicKey {
        self.core.creator
    }

    pub fn get_merkle_root(&self) -> SaitoHash {
        self.core.merkle_root
    }

    pub fn get_signature(&self) -> SaitoSignature {
        self.core.signature
    }

    pub fn get_treasury(&self) -> u64 {
        self.core.treasury
    }

    pub fn get_burnfee(&self) -> u64 {
        self.core.burnfee
    }

    pub fn get_difficulty(&self) -> u64 {
        self.core.difficulty
    }

    pub fn set_id(&mut self, id: u64) {
        self.core.id = id;
    }

    pub fn set_lc(&mut self, lc: bool) {
        self.lc = lc;
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.core.timestamp = timestamp;
    }

    pub fn set_previous_block_hash(&mut self, previous_block_hash: SaitoHash) {
        self.core.previous_block_hash = previous_block_hash;
    }

    pub fn set_creator(&mut self, creator: SaitoPublicKey) {
        self.core.creator = creator;
    }

    pub fn set_merkle_root(&mut self, merkle_root: SaitoHash) {
        self.core.merkle_root = merkle_root;
    }

    // TODO - signature needs to be generated from consensus vars
    pub fn set_signature(&mut self) {}

    pub fn set_treasury(&mut self, treasury: u64) {
        self.core.treasury = treasury;
    }

    pub fn set_burnfee(&mut self, burnfee: u64) {
        self.core.burnfee = burnfee;
    }

    pub fn set_difficulty(&mut self, difficulty: u64) {
        self.core.difficulty = difficulty;
    }

    pub fn set_hash(&mut self, hash: SaitoHash) {
        self.hash = hash;
    }

    // TODO
    //
    // hash is nor being serialized from the right data - requires
    // merkle_root as an input into the hash, and that is not yet
    // supported. this is a stub that uses the timestamp and the
    // id -- it exists so each block will still have a unique hash
    // for blockchain functions.
    //
    pub fn generate_hash(&mut self) -> SaitoHash {
        //
        // fastest known way that isn't bincode ??
        //
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.id.to_be_bytes());
        vbytes.extend(&self.core.timestamp.to_be_bytes());
        vbytes.extend(&self.core.previous_block_hash);
        vbytes.extend(&self.core.creator);
        vbytes.extend(&self.core.merkle_root);
        vbytes.extend(&self.core.signature);
        vbytes.extend(&self.core.treasury.to_be_bytes());
        vbytes.extend(&self.core.burnfee.to_be_bytes());
        vbytes.extend(&self.core.difficulty.to_be_bytes());

        hash(&vbytes)
    }

    pub fn generate_merkle_root(&mut self) -> SaitoHash {
        [0; 32]
    }

    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        longest_chain: bool,
    ) -> bool {
        for tx in &self.transactions {
            tx.on_chain_reorganization(utxoset, longest_chain, self.get_id());
        }
        return true;
    }

    pub fn validate(&self) -> bool {
        let _transactions_valid = &self.transactions.par_iter().all(|tx| tx.validate());

        return true;
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockCore::default())
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
    use crate::time::create_timestamp;

    #[test]
    fn block_core_new_test() {
        let timestamp = create_timestamp();
        let block_core = BlockCore::new(0, timestamp, [0; 32], [0; 33], [0; 32], [0; 64], 0, 0, 0);

        assert_eq!(block_core.id, 0);
        assert_eq!(block_core.timestamp, timestamp);
        assert_eq!(block_core.previous_block_hash, [0; 32]);
        assert_eq!(block_core.creator, [0; 33]);
        assert_eq!(block_core.merkle_root, [0; 32]);
        assert_eq!(block_core.signature, [0; 64]);
        assert_eq!(block_core.treasury, 0);
        assert_eq!(block_core.burnfee, 0);
        assert_eq!(block_core.difficulty, 0);
    }

    #[test]
    fn block_core_default_test() {
        let timestamp = create_timestamp();
        let block_core = BlockCore::default();

        assert_eq!(block_core.id, 0);
        assert_eq!(block_core.timestamp, timestamp);
        assert_eq!(block_core.previous_block_hash, [0; 32]);
        assert_eq!(block_core.creator, [0; 33]);
        assert_eq!(block_core.merkle_root, [0; 32]);
        assert_eq!(block_core.signature, [0; 64]);
        assert_eq!(block_core.treasury, 0);
        assert_eq!(block_core.burnfee, 0);
        assert_eq!(block_core.difficulty, 0);
    }

    #[test]
    fn block_new_test() {
        let timestamp = create_timestamp();
        let core = BlockCore::default();
        let block = Block::new(core);

        assert_eq!(block.core.id, 0);
        assert_eq!(block.core.timestamp, timestamp);
        assert_eq!(block.core.previous_block_hash, [0; 32]);
        assert_eq!(block.core.creator, [0; 33]);
        assert_eq!(block.core.merkle_root, [0; 32]);
        assert_eq!(block.core.signature, [0; 64]);
        assert_eq!(block.core.treasury, 0);
        assert_eq!(block.core.burnfee, 0);
        assert_eq!(block.core.difficulty, 0);
    }

    #[test]
    fn block_default_test() {
        let timestamp = create_timestamp();
        let block = Block::default();

        assert_eq!(block.core.id, 0);
        assert_eq!(block.core.timestamp, timestamp);
        assert_eq!(block.core.previous_block_hash, [0; 32]);
        assert_eq!(block.core.creator, [0; 33]);
        assert_eq!(block.core.merkle_root, [0; 32]);
        assert_eq!(block.core.signature, [0; 64]);
        assert_eq!(block.core.treasury, 0);
        assert_eq!(block.core.burnfee, 0);
        assert_eq!(block.core.difficulty, 0);
    }
}
