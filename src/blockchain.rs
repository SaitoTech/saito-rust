use crate::crypto::Sha256Hash;
use crate::block::Block;
use crate::forktree::ForkTree;
use crate::longest_chain_queue::LongestChainQueue;
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
pub enum AddBlockEvent {
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

    fn latest_block(&self) -> Option<&Block> {
        self.fork_tree.block_by_hash(&self.longest_chain_queue.latest_block_hash())
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    // If is the new longest chain
    fn is_new_longest_chain_tip(&self, block: &Block) -> bool {
        let mut old_chain = vec![];
        let mut new_chain = vec![];

        // new_chain.push(block);

        let mut target_block = block;
        let mut found_ancestor = false;
        let mut search_completed = false;

        while !search_completed {
            new_chain.push(target_block);
            if self.longest_chain_queue.contains_hash(&(target_block.hash())) {
                search_completed = true;
                found_ancestor = true;
            } else {
                match self.fork_tree.block_by_hash(target_block.previous_block_hash()) {
                    Some(previous_block) => target_block = previous_block,
                    None => search_completed = true
                }
            }
        }

        if !found_ancestor {
            false
        } else {
            let ancestor_block = target_block;
            let mut i = self.longest_chain_queue.latest_block_id();

            while i > ancestor_block.id() {
                let hash = self.longest_chain_queue.block_hash_by_id(i as u64);
                old_chain.push(self.fork_tree.block_by_hash(&hash).unwrap());
                i = i - 1;
            }

            new_chain.len() >= old_chain.len()
        }
    }

    // Get the block from fork_tree and check if the hash matches the one in longest_chain_queue
    fn is_in_longest_chain(&self, block_hash: &Sha256Hash) -> bool {
        // self.longest_chain_queue.epoch_ring_array.iter().any(|&hash| &hash == block_hash)
        self.longest_chain_queue.contains_hash(block_hash)
    }

    // Start at the block and go back, checking with longest_chain_queue by block_id until
    // the hash matches
    fn find_common_ancestor_in_longest_chain(&self, block: &Block) -> Sha256Hash {
        let hash = block.hash();
        if self.longest_chain_queue.block_hash_by_id(block.id()) == hash {
           hash
        } else {
            match self.fork_tree.block_by_hash(block.previous_block_hash()) {
                Some(previous_block) => self.find_common_ancestor_in_longest_chain(previous_block),
                None => [0; 32]
            }
        }
    }

    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub fn add_block(&mut self, block: Block) -> AddBlockEvent {
        // if(!all tx are valid)
        //   return some AddBlockEvent
        // else
        //   add the block to the fork_tree
        //   if(!if_is_new_longest_chain())
        //
        //     return some AddBlockEvent
        //   else
        //     find_common_ancestor_in_longest_chain()
        //     rollback from latest_block to common_ancestor
        //     rollforward to the new tip(i.e. block)
        if !self.validate_block(&block) {
            AddBlockEvent::InvalidTransaction
        } else if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            self.fork_tree.insert(block.hash(), block.clone());
            let latest_block_hash = &self.longest_chain_queue.latest_block_hash();
            if latest_block_hash == block.previous_block_hash() {
                self.utxoset.roll_forward(&block);
                self.longest_chain_queue.roll_forward(block.hash());
                AddBlockEvent::AcceptedAsLongestChain
            } else if !self.is_new_longest_chain_tip(&block) {
                self.utxoset.roll_forward_on_potential_fork(&block);
                AddBlockEvent::Accepted
            } else {
                // block is the tip of a new longest chain
                let common_ancestor_hash = self.find_common_ancestor_in_longest_chain(&block);
                let mut prev_block_hash = block.hash();
                while prev_block_hash != common_ancestor_hash {
                    prev_block_hash = self.longest_chain_queue.roll_back();
                    let block = &self.fork_tree.block_by_hash(&prev_block_hash).unwrap();
                    self.utxoset.roll_back(block);
                }

                // let next_block = &block;
                // 1) build a vec of blocks going back up the chain by walking it backwards once and
                // saving references to a Vec
                // 2) loop through the blocks in forward-order and do:
                //   self.longest_chain_queue.rollforward(block.hash())
                //   self.utxoset.rollforward( block.transactions());
                AddBlockEvent::AcceptedAsLongestChain
            }
        }
    }

    fn validate_block(&self, block: &Block) -> bool {
        !block.are_sigs_valid() | block.are_slips_spendable()
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
