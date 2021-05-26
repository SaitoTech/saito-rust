use crate::block::{Block, BlockHeader};
use crate::utxoset::UTXOSet;
use crate::transaction::Transaction;
use crate::crypto::SECP256K1Hash;

/// A tuple of `Block` metadata
pub type BlockIndex = (BlockHeader, SECP256K1Hash);

/// Indexes of chain attribute
#[derive(Debug, Clone)]
pub struct BlockchainIndex {
    /// Vector of blocks
    blocks: Vec<BlockIndex>,
}

impl BlockchainIndex {
    /// Creates new `BlockchainIndex`
    pub fn new() -> Self {
        BlockchainIndex { blocks: vec![] }
    }
}

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but potentially still useful in case of a reorg.
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// Index of `Block`s
    index: BlockchainIndex,
    /// Hashmap of slips used by the network
    utxoset: UTXOSet,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            index: BlockchainIndex::new(),
            utxoset: UTXOSet::new(),
        }
    }

    /// Returns the latest `Block` as part of the longest chain
    pub fn get_latest_block_index(&self) -> Option<&BlockIndex> {
        self.index.blocks.last()
    }

    /// Append `Block` to the index of `Blockchain`
    ///
    /// * `block` - `Block` appended to index
    pub fn add_block(&mut self, block: Block) {
        for (index, signed_tx) in block.transactions().iter().enumerate() {
            let tx = Transaction::new(index as u64, signed_tx.clone());
            self.utxoset.insert_transaction(&tx, block.id());
        }
        let block_index: BlockIndex = (block.header().clone(), block.hash());
        println!("{:?}", block_index.clone());
        self.index.blocks.push(block_index);
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::block::Block;
    use crate::keypair::Keypair;
    use crate::slip::{OutputSlip, SlipBroadcastType};
    use crate::transaction::{SignedTransaction, TransactionBody, TransactionBroadcastType};
    use secp256k1::Signature;

    #[test]
    fn blockchain_test() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.index.blocks, vec![]);
    }
    #[test]
    fn blockchain_get_latest_block_index_none_test() {
        let blockchain = Blockchain::new();
        match blockchain.get_latest_block_index() {
            None => assert!(true),
            _ => assert!(false),
        }
    }
    #[test]
    fn blockchain_get_latest_block_index_some_test() {
        let mut blockchain = Blockchain::new();
        let block = Block::new(Keypair::new().public_key().clone(), [0; 32]);

        blockchain.add_block(block.clone());

        match blockchain.get_latest_block_index() {
            Some((prev_block_header, _)) => {
                assert_eq!(&prev_block_header.clone(), block.header());
                assert!(true);
            }
            None => assert!(false),
        }
    }
    #[test]
    fn blockchain_add_block_test() {
        let keypair = Keypair::new();
        let mut blockchain = Blockchain::new();
        let mut block = Block::new(keypair.public_key().clone(), [0; 32]);
        let mut transaction_body = TransactionBody::new(TransactionBroadcastType::Normal);
        let output_slip = OutputSlip::new(
            *keypair.public_key(),
            SlipBroadcastType::Normal,
            2_0000_0000,
        );
        transaction_body.add_output(output_slip);
        let transaction = SignedTransaction::new(Signature::from_compact(&[0; 64]).unwrap(), transaction_body);
        block.add_transaction(transaction);

        blockchain.add_block(block.clone());
        let (block_header, _) = blockchain.index.blocks[0].clone();

        assert_eq!(block_header, *block.clone().header());
        // TODO Fix this
        // assert_eq!(blockchain.utxoset.slip_block_id(&slip), Some(&block.id()));
    }
}
