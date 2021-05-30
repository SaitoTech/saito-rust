use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::Transaction;
use std::str::FromStr;

/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    /// BlockCore contains consensus data like id, creator,  etc.
    /// when we receive blocks over the network, we are receiving
    /// this data. The remaining data associated with this Block
    /// is created locally.
    core: BlockCore,
    /// Memoized hash of the block
    hash: [u8; 32],
    /// `BurnFee` containing the fees paid to produce the block
    burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
}

impl Block {
    pub fn new_mock() -> Self {
        Block::new(BlockCore::new_mock(), [0;32], 0, 0.0);
    }
    pub fn new(core: BlockCore, hash: [u8; 32], burnfee: u64, difficulty: f32) -> Block {
        Block {
            core: core,
            hash: hash,
            burnfee: burnfee,
            difficulty: difficulty,
        }
    }

    /// Returns the `BlockCore` of `Block`
    pub fn core(&self) -> &BlockCore {
        &self.core
    }

    /// Returns the `Block` hash
    pub fn clone_hash(&self) -> [u8; 32] {
      self.hash.clone()
    }
    
    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn creator(&self) -> PublicKey {
        self.core.creator
    }
    
    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.core.coinbase
    }
    
    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.header.difficulty
    }
    
    /// Returns the `Block` burnfee
    pub fn burnfee(&self) -> u64 {
        self.header.burnfee
    }
}


/// The `BlockCore` holds the most important metadata associated with the `Block`
/// it is essentially the critical block data needed for distribution from which
/// nodes can derive the block and transaction and slip data.
#[derive(PartialEq, Debug, Clone)]
pub struct BlockCore {
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: [u8; 32],
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
    /// new(id: , timestamp: u64, previous_block_hash: [u8; 32], creator: PublicKey, coinbase: u64, transactions: Vec<Transaction>)
    /// For example, blockchain.add_block shoudl at least know the id, so new_mock() can become new_mock(id: u64) when we write that.
    pub fn new_mock() -> Self {
        let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        BlockCore::new(0, create_timestamp(), [0;32], public_key, 0, vec![])
    }
    /// Creates a new `BlockCore`
    pub fn new(id: u64, timestamp: u64, previous_block_hash: [u8; 32], creator: PublicKey, coinbase: u64, transactions: Vec<Transaction>) -> Self {
        BlockCore {
            id: id,
            timestamp: timestamp,
            previous_block_hash: previous_block_hash,
            creator: creator,
            coinbase: coinbase,
            transactions: transactions,
        }
    }

}


/// Update value of given field, reset memoised hash if changed.
fn update_field<T>(hash: &mut Option<[u8; 32]>, field: &mut T, value: T)
where
    T: PartialEq<T>,
{
    if field != &value {
        *field = value;
        *hash = None;
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        keypair::Keypair,
        slip::{OutputSlip, SlipBroadcastType, SlipID},
        transaction::{Transaction, TransactionType},
    };
    use secp256k1::Signature;

    #[test]
    fn block_test() {
        let mut block = Block::new_mock();

        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(block.creator(), &[0; 32]);
        assert_eq!(*block.transactions(), vec![]);
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.coinbase(), 0);
    }

    // #[test]
    // fn block_set_transactions_test() {
    //     let keypair = Keypair::new();
    //     let mut block = Block::new(*keypair.public_key(), [0; 32]);
    // 
    //     let mut tx = Transaction::new(TransactionType::Normal);
    //     let from_slip = SlipID::new(10, 10, 10);
    //     let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
    //     tx.add_input(from_slip);
    //     tx.add_output(to_slip);
    // 
    //     let signed_transaction =
    //         Transaction::add_signature(tx, Signature::from_compact(&[0; 64]).unwrap());
    //     block.set_transactions(&mut vec![signed_transaction.clone()]);
    // 
    //     assert_eq!(block.transactions().len(), 1);
    // 
    //     assert_eq!(
    //         block.transactions()[0].body.outputs()[0].address(),
    //         keypair.public_key()
    //     );
    //     assert_eq!(block.transactions()[0].body.outputs()[0].amount(), 0);
    //     assert_eq!(
    //         block.transactions()[0].body.outputs()[0].broadcast_type(),
    //         SlipBroadcastType::Normal
    //     );
    // 
    //     assert_eq!(block.transactions()[0].body.inputs()[0].slip_id(), 10);
    //     assert_eq!(block.transactions()[0].body.inputs()[0].tx_id(), 10);
    //     assert_eq!(block.transactions()[0].body.inputs()[0].block_id(), 10);
    // }
    // 
    // #[test]
    // fn block_add_transaction_test() {
    //     let keypair = Keypair::new();
    //     let mut block = Block::new(*keypair.public_key(), [0; 32]);
    //     let tx = Transaction::new(TransactionType::Normal);
    //     assert_eq!(*block.transactions(), vec![]);
    //     let signed_transaction =
    //         Transaction::add_signature(tx, Signature::from_compact(&[0; 64]).unwrap());
    //     block.add_transaction(signed_transaction.clone());
    //     assert_eq!(*block.transactions(), vec![signed_transaction.clone()]);
    // }
}
