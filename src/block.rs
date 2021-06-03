use crate::crypto::{make_message_from_bytes, PublicKey, Sha256Hash};
use crate::time::create_timestamp;
use crate::transaction::Transaction;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use std::{mem, slice};

/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block {
    /// Memoized hash of the block
    hash: Sha256Hash,
    /// `BurnFee` containing the fees paid to produce the block
    burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// BlockCore contains consensus data like id, creator,  etc.
    /// when we receive blocks over the network, we are receiving
    /// this data. The remaining data associated with this Block
    /// is created locally.
    core: BlockCore,
}

impl Block {
    pub fn default() -> Self {
        Block::new(0, 0.0, BlockCore::default())
    }
    /// Creates a new `BlockCore`
    pub fn new_mock(previous_block_hash: Sha256Hash, transactions: Vec<Transaction>) -> Self {
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        let timestamp = create_timestamp();
        let block_core = BlockCore::new(
            0,
            timestamp,
            previous_block_hash,
            public_key,
            0,
            transactions,
        );
        Block::new(0, 0.0, block_core)
    }
    pub fn new(burnfee: u64, difficulty: f32, core: BlockCore) -> Block {
        let hash = make_message_from_bytes(&core.serialize());
        Block {
            hash: hash,
            burnfee: burnfee,
            difficulty: difficulty,
            core: core,
        }
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.difficulty
    }

    /// Returns the `Block` burnfee
    pub fn burnfee(&self) -> u64 {
        self.burnfee
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

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.core.coinbase
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

    /// Loops through all tx and
    pub fn are_sigs_valid(&self) -> bool {
        // loops through all tx and do tx.sig_is_valid()
        true
    }

    /// Loops through all tx and
    pub fn are_slips_spendable(&self) -> bool {
        // loops through all tx and check with utxoset that all inputs are spendable
        // their receiver and amount are correct
        true
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
    /// simplified transaction cores
    transactions: Vec<Transaction>,
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
        BlockCore::new(0, create_timestamp(), [0; 32], public_key, 0, vec![])
    }
    /// Creates a new `BlockCore`
    pub fn new(
        id: u64,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        creator: PublicKey,
        coinbase: u64,
        transactions: Vec<Transaction>,
    ) -> Self {
        BlockCore {
            id: id,
            timestamp: timestamp,
            previous_block_hash: previous_block_hash,
            creator: creator,
            coinbase: coinbase,
            transactions: transactions,
        }
    }

    // pub fn deserialize(bytes: [u8; 42]) -> Slip {
    //     let public_key: PublicKey = PublicKey::from_slice(&bytes[..33]).unwrap();
    //     let broadcast_type: SlipBroadcastType = SlipBroadcastType::try_from(bytes[41]).unwrap();
    //     let amount = u64::from_be_bytes(bytes[33..41].try_into().unwrap());
    //     Slip::new(public_key, broadcast_type, amount)
    // }

    pub fn serialize(&self) -> [u8; 44] {
        let mut ret = [0; 44];
        ret[..32].clone_from_slice(&self.previous_block_hash);
        unsafe {
            ret[32..40].clone_from_slice(&slice::from_raw_parts(
                (&self.timestamp as *const u64) as *const u8,
                mem::size_of::<u64>(),
            ));
        }
        // TODO REMOVE THESE RANDOM BYTES ONCE WE ARE ACTUALLY DOING A FULL HASH, THIS IS JUST
        // FOR HASHING BECAUSE THE TIMESTAMP ISN'T PRECISE ENOUGH TO GUARANTEE UNIQUENESS
        let random_bytes = rand::thread_rng().gen::<[u8; 4]>();
        ret[40..44].clone_from_slice(&random_bytes);
        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::Block;
    use crate::slip::SlipType;
    use crate::test_utilities;
    use secp256k1::Signature;

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
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.coinbase(), 0);
    }

    #[test]
    fn block_set_transactions_test() {
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        let block = test_utilities::make_mock_block([0; 32]);
        println!("txs: {:?}", block.transactions());
        assert_eq!(block.transactions().len(), 2);
        assert_eq!(
            block.transactions()[0].core.outputs()[0].address(),
            &public_key
        );

        assert_eq!(block.transactions()[0].core.outputs()[0].amount(), 10);
        assert_eq!(
            block.transactions()[0].core.outputs()[0].broadcast_type(),
            SlipType::Normal
        );

        assert_eq!(
            block.transactions()[0].core.inputs()[0].tx_id(),
            Signature::from_compact(&[0; 64]).unwrap()
        );

        assert_eq!(block.transactions()[0].core.inputs()[0].slip_ordinal(), 0);
    }
}
