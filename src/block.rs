use crate::crypto::{hash, PublicKey};
use crate::time::create_timestamp;
use crate::transaction::Transaction;

/// The `Block` holds all data inside the block body,
/// and additional metadata not to be serialized
#[derive(PartialEq, Debug, Clone)]
pub struct Block {
    /// The header of the block object
    header: BlockHeader,
    /// The body and content of the block object
    body: BlockBody,
    /// Memoized hash of the block
    hash: Option<[u8; 32]>,
}

/// This `Header` holds `Block`'s metadata
#[derive(PartialEq, Debug, Clone)]
pub struct BlockHeader {
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: [u8; 32],
    /// `Publickey` of the block creator
    creator: PublicKey,
    /// `BurnFee` containing the fees paid to produce the block
    burnfee: u64,
    /// Block difficulty required to win the `LotteryGame` in golden ticket generation
    difficulty: f32,
    /// Treasury of existing Saito in the network
    treasury: u64,
    /// Total block reward being released in the block
    coinbase: u64,
}

/// This `BlockBody` holds data to be serialized along with
/// `Transaction`s
#[derive(PartialEq, Debug, Clone)]
pub struct BlockBody {
    /// List of transactions in the block
    transactions: Vec<Transaction>,
}

impl BlockHeader {
    /// Creates a new `BlockHeader`
    ///
    /// * `creator` - `secp256k1::PublicKey` of the block creator
    /// * `previous_block_hash` - Previous block hash in bytes
    pub fn new(creator: PublicKey, previous_block_hash: [u8; 32]) -> Self {
        BlockHeader {
            id: 0,
            timestamp: create_timestamp(),
            previous_block_hash,
            creator,
            burnfee: 0,
            difficulty: 0.0,
            treasury: 286_810_000_000_000_000,
            coinbase: 0,
        }
    }

    /// Returns the `Block` id
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the `Block` timestamp
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.difficulty
    }

    /// Returns the `Block` difficulty
    pub fn previous_block_hash(&self) -> [u8; 32] {
        self.previous_block_hash
    }

    /// Returns the `Block` treasury
    pub fn treasury(&self) -> u64 {
        self.treasury
    }

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.coinbase
    }
}

impl BlockBody {
    /// Creates a new `BlockBody`
    pub fn new() -> Self {
        BlockBody {
            transactions: vec![],
        }
    }
}

impl Block {
    /// Receives the a publickey and the previous block hash
    ///
    /// * `creator` - `secp256k1::PublicKey` of the block creator
    /// * `previous_block_hash` - Previous block hash in bytes
    pub fn new(creator: PublicKey, previous_block_hash: [u8; 32]) -> Block {
        Block {
            header: BlockHeader::new(creator, previous_block_hash),
            body: BlockBody::new(),
            hash: None,
        }
    }

    // Returns the `BlockHeader` of `Block`
    pub fn header(&self) -> &BlockHeader {
        &self.header
    }

    /// Returns the `Block` id
    pub fn id(&self) -> u64 {
        self.header.id
    }

    /// Returns the `Block` timestamp
    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    /// Returns the previous `Block` hash
    pub fn previous_block_hash(&self) -> &[u8; 32] {
        &self.header.previous_block_hash
    }

    /// Returns the `Block` creator's `secp256k1::PublicKey`
    pub fn creator(&self) -> &PublicKey {
        &self.header.creator
    }

    /// Returns the `Block`'s `Transaction`s
    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.body.transactions
    }

    /// Returns the `Block` burnfee
    pub fn burnfee(&self) -> u64 {
        self.header.burnfee
    }

    /// Returns the `Block` difficulty
    pub fn difficulty(&self) -> f32 {
        self.header.difficulty
    }

    /// Returns the `Block` treasury
    pub fn treasury(&self) -> u64 {
        self.header.treasury
    }

    /// Returns the `Block` coinbase
    pub fn coinbase(&self) -> u64 {
        self.header.coinbase
    }

    /// Compute and memoize the block hash
    pub fn compute_hash(&mut self) -> [u8; 32] {
        let hash = self.hash();
        self.hash = Some(hash);
        hash
    }

    /// Generate the block hash
    ///
    /// TODO -- extend list of information we use to calculate the block hash
    pub fn hash(&self) -> [u8; 32] {
        if self.hash.is_none() {
            let mut data: Vec<u8> = vec![];

            let id_bytes: [u8; 8] = self.header.id.to_be_bytes();
            let ts_bytes: [u8; 8] = self.header.timestamp.to_be_bytes();
            let cr_bytes: Vec<u8> = self.header.creator.serialize().iter().cloned().collect();

            data.extend(&id_bytes);
            data.extend(&ts_bytes);
            data.extend(&cr_bytes);
            hash(&data)
        } else {
            self.hash.unwrap()
        }
    }

    /// Converts our blockhash from a byte array into a hex string
    pub fn hash_as_hex(&self) -> String {
        let hash = self.hash.unwrap_or_else(|| self.hash());
        hex::encode(hash)
    }

    /// Sets the `Block`s list of `Transaction`s
    pub fn set_transactions(&mut self, transactions: &mut Vec<Transaction>) {
        let bid = self.header.id;

        for (i, tx) in transactions.iter_mut().enumerate() {
            // The slips are assigned the ids based on their slip index
            // in reference to all slips in the block, the transaction index,
            // and the block id
            let mut current_sid = 0;

            for slip in tx.outputs_mut().iter_mut() {
                slip.set_block_id(0);
                slip.set_tx_id(0);
                slip.set_slip_id(current_sid);
                current_sid += 1;
            }

            for slip in tx.inputs_mut().iter_mut() {
                slip.set_block_id(bid);
                slip.set_tx_id(i as u64);
                slip.set_slip_id(current_sid);
                current_sid += 1;
            }
        }
        self.body.transactions = transactions.to_vec();
    }

    /// Appends a transaction to the block
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.body.transactions.push(tx);
    }

    /// Sets the id of the block
    pub fn set_id(&mut self, id: u64) {
        update_field(&mut self.hash, &mut self.header.id, id)
    }

    /// Sets the `Block` burnfee
    pub fn set_burnfee(&mut self, bf: u64) {
        update_field(&mut self.hash, &mut self.header.burnfee, bf)
    }

    /// Sets the `Block` previous hash
    pub fn set_previous_block_hash(&mut self, previous_block_hash: [u8; 32]) {
        update_field(
            &mut self.hash,
            &mut self.header.previous_block_hash,
            previous_block_hash,
        )
    }

    /// Sets the `Block` difficulty
    pub fn set_difficulty(&mut self, difficulty: f32) {
        update_field(&mut self.hash, &mut self.header.difficulty, difficulty)
    }

    /// Sets the `Block` treasury
    pub fn set_treasury(&mut self, treasury: u64) {
        update_field(&mut self.hash, &mut self.header.treasury, treasury)
    }

    /// Sets the `Block` coinbase
    pub fn set_coinbase(&mut self, coinbase: u64) {
        update_field(&mut self.hash, &mut self.header.coinbase, coinbase)
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
        let keypair = Keypair::new();
        let mut block = Block::new(*keypair.public_key(), [0; 32]);

        assert_eq!(block.id(), 0);
        assert_eq!(block.previous_block_hash(), &[0; 32]);
        assert_eq!(block.creator(), keypair.public_key());
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

    #[test]
    fn block_set_transactions_test() {
        let keypair = Keypair::new();
        let mut block = Block::new(*keypair.public_key(), [0; 32]);

        let mut tx = Transaction::new(TransactionBroadcastType::Normal);
        let from_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        let to_slip = Slip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
        tx.add_input(from_slip);
        tx.add_output(to_slip);
        block.set_transactions(&mut vec![tx.clone()]);

        assert_eq!(block.transactions().len(), 1);

        assert_eq!(block.transactions()[0].outputs()[0].slip_id(), 0);
        assert_eq!(block.transactions()[0].outputs()[0].tx_id(), 0);
        assert_eq!(block.transactions()[0].outputs()[0].block_id(), 0);

        assert_eq!(block.transactions()[0].inputs()[0].slip_id(), 1);
        assert_eq!(block.transactions()[0].inputs()[0].tx_id(), 0);
        assert_eq!(block.transactions()[0].inputs()[0].block_id(), 0);
    }

    #[test]
    fn block_add_transaction_test() {
        let keypair = Keypair::new();
        let mut block = Block::new(*keypair.public_key(), [0; 32]);
        let tx = Transaction::new(TransactionBroadcastType::Normal);
        assert_eq!(*block.transactions(), vec![]);
        block.add_transaction(tx.clone());
        assert_eq!(*block.transactions(), vec![tx.clone()]);
    }
}
