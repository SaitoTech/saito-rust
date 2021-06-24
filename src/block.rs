use crate::{time::create_timestamp, transaction::Transaction};
use serde::{Deserialize, Serialize};

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
    previous_block_hash: [u8; 32], // sha256hash
    #[serde_as(as = "[_; 33]")]
    creator: [u8; 33], // secp256k1 publickey
    merkle_root: [u8; 32],         // sha256hash
    #[serde_as(as = "[_; 64]")]
    signature: [u8; 64], // signature of block creator
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
    hash: [u8; 32],
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

    pub fn get_hash(&self) -> [u8; 32] {
        self.hash
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockCore::default())
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
