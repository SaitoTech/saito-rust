use crate::block::Block;
use crate::forktree::ForkTree;
use crate::longestchainqueue::LongestChainQueue;
use crate::utxoset::UtxoSet;

lazy_static! {
    // This allows us to get a reference to blockchain, utxoset, longest_chain_queue, or fork_tree
    // from anywhere in the codebase. In the future we can also use message passing or put
    // other objects into global statics when needed. This is just a temporary solution to get
    // things working before we optimize.
    // To use: add crate::blockchain::BLOCKCHAIN and add getters to Blockchain if needed
    pub static ref BLOCKCHAIN: Blockchain = Blockchain::new();
}

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum AddBlockReason {
    OK,
    ParentNotFound,
}

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// Hashmap of slips used by the network
    utxoset: UtxoSet,
    // A queue-like structure that holds the longest chain
    longest_chain_queue: LongestChainQueue,
    // hashmap back tree to track blocks and potential forks
    fork_tree: ForkTree,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            utxoset: UtxoSet::new(),
            longest_chain_queue: LongestChainQueue::new(),
            fork_tree: ForkTree::new(),
        }
    }

    /// If the block is in the fork
    pub fn is_block_hash_known(&mut self, _block_hash: [u8; 32]) -> bool {
        // TODO check if the block is in the fork tree
        true
    }
    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) -> AddBlockReason {
        if !self.is_block_hash_known(block.hash()) {
            return AddBlockReason::ParentNotFound;
        }
        // if(!all tx in block have valid sigs)
        // if(!all tx)
        self.longest_chain_queue.roll_forward(block.hash());
        for tx in block.transactions().iter() {
            self.utxoset.spend_transaction(tx);
        }
        self.fork_tree.insert(block.hash(), block);
        AddBlockReason::OK
    }

    // /// Return latest `Block` in blockchain
    // pub fn get_latest_block(&mut self) -> &Block {
    //
    // }
}

#[cfg(test)]
mod tests {

    // use super::*;
    // use crate::block::Block;
    // use crate::keypair::Keypair;
    // use crate::slip::{OutputSlip, SlipID};
    // use crate::transaction::{Transaction, TransactionType};
    // use secp256k1::Signature;

    // #[test]
    // fn blockchain_test() {
    //     let blockchain = Blockchain::new();
    //     assert_eq!(blockchain.index.blocks, vec![]);
    // }
    // #[test]
    // fn blockchain_get_latest_block_index_none_test() {
    //     let blockchain = Blockchain::new();
    //     match blockchain.get_latest_block_index() {
    //         None => assert!(true),
    //         _ => assert!(false),
    //     }
    // }
    // #[test]
    // fn blockchain_get_latest_block_index_some_test() {
    //     let mut blockchain = Blockchain::new();
    //     let block = Block::new(Keypair::new().public_key().clone(), [0; 32]);
    //
    //     blockchain.add_block(block.clone());
    //
    //     match blockchain.get_latest_block_index() {
    //         Some((prev_block_header, _)) => {
    //             assert_eq!(&prev_block_header.clone(), block.header());
    //             assert!(true);
    //         }
    //         None => assert!(false),
    //     }
    // }
    // #[test]
    // fn blockchain_add_block_test() {
    //     let keypair = Keypair::new();
    //     let mut blockchain = Blockchain::new();
    //     let mut block = Block::new(keypair.public_key().clone(), [0; 32]);
    //     let mut transaction = Transaction::new(TransactionType::Normal);
    //     let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 0);
    //     transaction.add_output(to_slip);
    //
    //     let signed_transaction =
    //         Transaction::add_signature(transaction, Signature::from_compact(&[0; 64]).unwrap());
    //     block.add_transaction(signed_transaction);
    //
    //     blockchain.add_block(block.clone());
    //     let (block_header, _) = blockchain.index.blocks[0].clone();
    //
    //     assert_eq!(block_header, *block.clone().header());
    //     //assert_eq!(blockchain.utxoset.slip_block_id(&slip), Some(&-1));
    // }
}
