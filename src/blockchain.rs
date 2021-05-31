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
    AcceptedAsLongestChain,
    Accepted,
    AlreadyKnown,
    ParentNotFound,
    InvalidTransaction,
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
    fn is_block_hash_known(&mut self, _block_hash: &[u8; 32]) -> bool {
        // TODO check if the block is in the fork tree
        true
    }
    // If is the new longest chain
    fn is_new_longest_chain_tip(block: Block) -> bool {
        true
    }
    // Get the block from fork_tree and check if the hash matches the one in longest_chain_queue
    fn is_in_longest_chain(block_hash: [u8; 32]) -> bool {
        true
    }

    // Start at the block and go back, checking with longest_chain_queue by block_id until
    // the hash matches
    fn find_common_ancestor_in_longest_chain(new_tip_block: Block) -> [u8; 32] {
        [0; 32]
    }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockReason`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub fn add_block(&mut self, block: Block) -> AddBlockReason {
        // if(!all tx are valid)
        //   return some AddBlockReason
        // else
        //   add the block to the fork_tree
        //   if(!if_is_new_longest_chain())
        //
        //     return some AddBlockReason
        //   else
        //     find_common_ancestor_in_longest_chain()
        //     rollback from latest_block to common_ancestor
        //     rollforward to the new tip(i.e. block)
        if !block.are_sigs_valid() | block.are_slips_spendable(){
            return AddBlockReason::InvalidTransaction;
        } else if self.is_block_hash_known(block.hash()) {
            return AddBlockReason::AlreadyKnown;
        } else if !self.is_block_hash_known(block.previous_block_hash()) {
            return AddBlockReason::ParentNotFound;
        } else {
            self.fork_tree.insert(block.hash(), block);
            if (!self.is_new_longest_chain_tip(block)){
                self.longest_chain_queue.rollforward(block.hash())
                self.utxoset.rollforward(block.transactions());
                return AddBlockReason::Accepted;
            } else {
                // block is the tip of a new longest chain
                let common_ancestor = self.find_common_ancestor_in_longest_chain();
                let next_block = block;
                
                while next_block.previous_block_hash() != common_ancestor.hash() {
                    self.longest_chain_queue.rollback(block.hash())
                    self.utxoset.rollback( block.transactions());    
                }
                
                
                next_block = block;
                // 1) build a vec of blocks going back up the chain by walking it backwards once and
                // saving references to a Vec
                // 2) loop through the blocks in forward-order and do:
                //   self.longest_chain_queue.rollforward(block.hash())
                //   self.utxoset.rollforward( block.transactions());
                return AddBlockReason::AcceptedAsLongestChain
            }    
        }
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
