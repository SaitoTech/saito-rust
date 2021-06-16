use crate::crypto::{hash_bytes, PublicKey, Sha256Hash};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionSignatureCore};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const TREASURY: u64 = 286_810_000_000_000_000;

/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// Memoized hash of the block
    hash: Sha256Hash,
    /// BlockCore contains consensus data like id, creator,  etc.
    /// when we receive blocks over the network, we are receiving
    /// this data. The remaining data associated with this Block
    /// is created locally.
    core: BlockCore,
}

impl Block {
    pub fn default() -> Self {
        Block::new(BlockCore::default())
    }
    /// Creates a new `BlockCore`
    pub fn new_mock(
        previous_block_hash: Sha256Hash,
        transactions: &mut Vec<Transaction>,
        block_id: u64,
    ) -> Self {
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        let timestamp = create_timestamp();
        let block_core = BlockCore::new(
            block_id,
            timestamp,
            previous_block_hash,
            public_key,
            0,
            TREASURY,
            0.0,
            0.0,
            transactions,
        );
        Block::new(block_core)
    }

    pub fn new(core: BlockCore) -> Block {
        let core_bytes: Vec<u8> = core.hash_core().into();
        Block {
            hash: hash_bytes(&core_bytes),
            core,
        }
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.core.difficulty
    }

    /// Returns the `BlockCore` of `Block`
    pub fn core(&self) -> &BlockCore {
        &self.core
    }

    /// Returns the `Block` hash
    pub fn clone_hash(&self) -> Sha256Hash {
        self.hash.clone()
    }

    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn timestamp(&self) -> u64 {
        self.core.timestamp
    }

    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn creator(&self) -> PublicKey {
        self.core.creator
    }

    /// Returns the `Block` treasury
    pub fn treasury(&self) -> u64 {
        self.core.treasury
    }

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.core.coinbase
    }

    /// Returns the `Block` coinbase
    pub fn start_burnfee(&self) -> f64 {
        self.core.start_burnfee
    }

    /// Returns the `Block`'s `Transaction`s
    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.core.transactions
    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> &Sha256Hash {
        &self.core.previous_block_hash
    }

    /// Returns the `Block` id
    pub fn id(&self) -> u64 {
        self.core.id
    }

    /// Returns the `hash`
    pub fn hash(&self) -> Sha256Hash {
        self.hash
    }

    /// Converts our blockhash from a byte array into a hex string
    pub fn hash_as_hex(&self) -> String {
        hex::encode(self.hash)
    }
}

/// The `BlockCore` holds the most important metadata associated with the `Block`
/// it is essentially the critical block data needed for distribution from which
/// nodes can derive the block and transaction and slip data.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockCore {
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: Sha256Hash,
    /// `Publickey` of the block creator
    creator: PublicKey,
    /// Total block reward being released in the block
    coinbase: u64,
    /// Amount of SAITO left in reserve on the network
    treasury: u64,
    /// Start value in the `Burnfee` algorithm
    start_burnfee: f64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// simplified transaction cores
    transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlockHashCore {
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: Sha256Hash,
    /// `Publickey` of the block creator
    creator: PublicKey,
    /// Total block reward being released in the block
    coinbase: u64,
    /// Amount of SAITO left in reserve on the network
    treasury: u64,
    /// Start value in the `Burnfee` algorithm
    start_burnfee: f64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// simplified transaction cores
    transaction_sig_cores: Vec<TransactionSignatureCore>,
}

impl BlockCore {
    /// Creates a new mock `BlockCore` for use as we develop code. Please replace the fields with actual fields as we get them
    /// until we arrive at something the actual constructor:
    /// new(id: , timestamp: u64, previous_block_hash: Sha256Hash, creator: PublicKey, coinbase: u64, transactions: Vec<Transaction>)
    /// For example, blockchain.add_block shoudl at least know the id, so default() can become default(id: u64) when we write that.
    pub fn default() -> Self {
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        BlockCore::new(
            0,
            create_timestamp(),
            [0; 32],
            public_key,
            0,
            TREASURY,
            0.0,
            0.0,
            &mut vec![],
        )
    }

    /// Creates a new `BlockCore`
    pub fn new(
        id: u64,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        creator: PublicKey,
        coinbase: u64,
        treasury: u64,
        start_burnfee: f64,
        difficulty: f32,
        transactions: &mut Vec<Transaction>,
    ) -> Self {
        BlockCore {
            id,
            timestamp,
            previous_block_hash,
            creator,
            coinbase,
            treasury,
            start_burnfee,
            difficulty,
            transactions: transactions.to_vec(),
        }
    }

    pub fn hash_core(&self) -> BlockHashCore {
        BlockHashCore {
            id: self.id,
            timestamp: self.timestamp,
            previous_block_hash: self.previous_block_hash,
            creator: self.creator,
            coinbase: self.coinbase,
            treasury: self.treasury,
            start_burnfee: self.start_burnfee,
            difficulty: self.difficulty,
            transaction_sig_cores: self.transactions.iter().map(|tx| tx.core().sig_core()).collect()
        }
    }
}

impl Into<Vec<u8>> for BlockHashCore {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

impl From<Vec<u8>> for BlockCore {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data[..]).unwrap()
    }
}

impl Into<Vec<u8>> for BlockCore {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

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

impl Into<Vec<u8>> for &Block {
    fn into(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;
    use crate::slip::{SlipID, SlipType};
    use crate::test_utilities;

    #[test]
    fn block_test() {
        let block = Block::default();
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(block.creator(), public_key);
        assert_eq!(*block.transactions(), vec![]);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.coinbase(), 0);
        assert_eq!(block.start_burnfee(), 0.0);
    }

    #[test]
    fn block_set_transactions_test() {
        let keypair = Keypair::new();
        let public_key = keypair.public_key();

        let block = test_utilities::make_mock_block(&keypair, [0; 32], 0, SlipID::new([0; 32], 0));
        assert_eq!(block.transactions().len(), 1);
        assert_eq!(
            block.transactions()[0].core.outputs()[0].address(),
            public_key
        );

        assert_eq!(block.transactions()[0].core.outputs()[0].amount(), 10);
        assert_eq!(
            block.transactions()[0].core.outputs()[0].broadcast_type(),
            SlipType::Normal
        );

        assert_eq!(block.transactions()[0].core.inputs()[0].tx_id(), [0; 32]);

        assert_eq!(block.transactions()[0].core.inputs()[0].slip_ordinal(), 0);
    }
}
