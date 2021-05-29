use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionCore};


/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(PartialEq, Debug, Clone)]
pub struct Block {

    /// BlockCore contains consensus data like id, creator,  etc.
    /// when we receive blocks over the network, we are receiving
    /// this data. The remaining data associated with this Block
    /// is created locally.
    core: BlockCore,

    /// full transactions in block
    transactions: Vec<Transaction>,

    /// Memoized hash of the block
    hash: Option<[u8; 32]>,

}

impl Block {

    pub fn new() -> Block {
        Block {
            core: BlockCore::new(),
            transactions: vec![],
            hash: None,
        }
    }

    /// Returns the `BlockCore` of `Block`
    pub fn core(&self) -> &BlockCore {
        &self.core
    }

    /// Returns the `Block` transactions
    pub fn transactions(&self) -> Vec<Transactions> {

	/// TODO - if transactions do not exist, create from TransactionCore

	/// TODO - if TransactionCores do not exist, load from disk

        self.transactions
    }

    /// Returns the `Block` hash
    pub fn hash(&self) -> Option<[u8; 32]> {

      if self.hash.is_none() {

        let mut data: Vec<u8> = vec![];

        let id_bytes: [u8; 8] = self.core.id.to_be_bytes();
        let ts_bytes: [u8; 8] = self.core.timestamp.to_be_bytes();
        let cr_bytes: Vec<u8> = self.core.creator.serialize().iter().cloned().collect();

        data.extend(&id_bytes);
        data.extend(&ts_bytes);
        data.extend(&cr_bytes);

        self.hash = hash(&data);

      }

      self.hash

    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> &[u8; 32] {
        &self.core.previous_block_hash
    }

    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn creator(&self) -> &PublicKey {
        &self.core.creator
    }

    /// Returns the `Block` burnfee
    pub fn burnfee(&self) -> u64 {
        self.core.burnfee
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.core.difficulty
    }

    /// Returns the `Block` treasury
    pub fn treasury(&self) -> u64 {
        self.core.treasury
    }

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.core.coinbase
    }


    /// Converts our blockhash from a byte array into a hex string
    /// -- is there any reason to keep the hash as binary? why not
    /// -- just. in that case we do not really need this additional
    /// -- function and can keep things a bit cleaner -- convert to
    /// -- hex automatically in hash() function if we need to run it
    ///pub fn hash_as_hex(&self) -> String {
    ///    let hash = self.hash.unwrap_or_else(|| self.hash());
    ///    hex::encode(hash)
    ///}


    /// Sets the id of the block
    pub fn set_id(&mut self, id: u64) {
        update_field(&mut self.hash, &mut self.core.id, id)
    }

    /// Sets the `Block` burnfee
    pub fn set_burnfee(&mut self, bf: u64) {
        update_field(&mut self.hash, &mut self.core.burnfee, bf)
    }

    /// Sets the `Block` creator
    pub fn set_creator(&mut self, creator: &Publickey) {
        update_field(&mut self.hash, &mut self.core.creator, creator)
    }

    /// Sets the `Block` previous hash
    pub fn set_previous_block_hash(&mut self, previous_block_hash: [u8; 32]) {
        update_field(&mut self.hash, &mut self.core.previous_block_hash, previous_block_hash)
    }

    /// Sets the `Block` difficulty
    pub fn set_difficulty(&mut self, difficulty: f32) {
        update_field(&mut self.hash, &mut self.core.difficulty, difficulty)
    }

    /// Sets the `Block` treasury
    pub fn set_treasury(&mut self, treasury: u64) {
        update_field(&mut self.hash, &mut self.core.treasury, treasury)
    }

    /// Sets the `Block` coinbase
    pub fn set_coinbase(&mut self, coinbase: u64) {
        update_field(&mut self.hash, &mut self.core.coinbase, coinbase)
    }



    /// adds transaction to `Block`
    pub fn add_transaction(&mut self, tx: Transaction) {
      self.transactions.push(tx);
    }

    /// check if `Block` is valid, returns true if valid, false if invalid
    pub fn validate(&mut self, tx: Transaction) {
      true
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
    creator: Option<PublicKey>,
    /// `BurnFee` containing the fees paid to produce the block
    burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// Treasury of existing Saito in the network
    treasury: u64,
    /// Total block reward being released in the block
    coinbase: u64,

    /// simplified transaction cores
    transaction_cores: Vec<TransactionCore>,

}


impl BlockCore {

    /// Creates a new `BlockCore`
    pub fn new() -> Self {
        BlockCore {
            id: 0,
            previous_block_hash: [0; 32],
            creator: None,
            burnfee: 0,
            difficulty: 0.0,
            treasury: 286_810_000_000_000_000,
            coinbase: 0,
            transaction_cores: vec![],
        }
    }

    /// Returns the `BlockCore` id
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the `BlockCore` id
    pub fn previous_block_hash(&self) -> [u8; 32] {
        self.previous_block_hash
    }

    /// Returns the `BlockCore` timestamp
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the `BlockCore` difficulty
    pub fn difficulty(&self) -> f32 {
        self.difficulty
    }

    /// Returns the `BlockCore` treasury
    pub fn treasury(&self) -> u64 {
        self.treasury
    }

    /// Returns the `BlockCore` coinbase
    pub fn coinbase(&self) -> u64 {
        self.coinbase
    }


    /// Sets the `BlockCore` id
    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    /// Sets the `BlockCore` burnfee
    pub fn set_burnfee(&mut self, bf: u64) {
        self.burnfee = bf;
    }

    /// Sets the `BlockCore` previous hash
    pub fn set_previous_block_hash(&mut self, previous_block_hash: [u8; 32]) {
        self.previous_block_hash = previous_block_hash;
    }

    /// Sets the `BlockCore` difficulty
    pub fn set_difficulty(&mut self, difficulty: f32) {
        self.difficulty = difficulty;
    }

    /// Sets the `BlockCore` treasury
    pub fn set_treasury(&mut self, treasury: u64) {
        self.treasury = treasury;
    }

    /// Sets the `BlockCore` coinbase
    pub fn set_coinbase(&mut self, coinbase: u64) {
        self.coinbase = coinbase;
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
        slip::{Slip, SlipBroadcastType},
        transaction::{Transaction, TransactionBroadcastType},
    };

    #[test]
    fn block_test() {
        let mut block = Block::new();

        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(block.creator(), &[0; 32]);
        assert_eq!(*block.transactions(), vec![]);
        assert_eq!(block.burnfee(), 0);
        assert_eq!(block.difficulty(), 0.0);
        assert_eq!(block.treasury(), 286_810_000_000_000_000);
        assert_eq!(block.coinbase(), 0);

        block.set_id(1);
        assert_eq!(block.id(), 1);

        block.set_burnfee(10_000);
        assert_eq!(block.burnfee(), 10_000);

        block.set_difficulty(5.0);
        assert_eq!(block.difficulty(), 5.0);

        block.set_treasury(1_000_000_000);
        assert_eq!(block.treasury(), 1_000_000_000);

        block.set_coinbase(100_000);
        assert_eq!(block.coinbase(), 100_000);
    }

}
