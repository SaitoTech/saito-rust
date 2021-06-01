use crate::block::Block;
use crate::crypto::Sha256Hash;
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
    AcceptedAsNewLongestChain,
    AcceptedAsLongestChain,
    Accepted,
    AlreadyKnown,
    AncestorNotFound,
    ParentNotFound,
    InvalidTransaction,
}

pub struct ChainFork {
    // TODO -- add lifetime and reference to block
    blocks: Vec<Block>
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

    pub fn latest_block(&self) -> Option<&Block> {
        self.fork_tree
            .block_by_hash(&self.longest_chain_queue.latest_block_hash())
    }

    // Get the block from fork_tree and check if the hash matches the one in longest_chain_queue
    fn _contains_block_hash_in_longest_chain(&self, block_hash: &Sha256Hash) -> bool {
        self.longest_chain_queue.contains_hash(block_hash)
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    fn fork_chains(&self, block: &Block) -> Option<(Block, ChainFork, ChainFork)> {
        let mut old_chain = ChainFork { blocks: vec![] };
        let mut new_chain = ChainFork { blocks: vec![] };

        let mut target_block = block;
        let mut found_ancestor = false;
        let mut search_completed = false;

        while !search_completed {
            new_chain.blocks.push(target_block.clone());
            if self
                .longest_chain_queue
                .contains_hash_by_block_id(target_block.hash(), target_block.id())
            {
                search_completed = true;
                found_ancestor = true;
            } else {
                match self
                    .fork_tree
                    .block_by_hash(target_block.previous_block_hash())
                {
                    Some(previous_block) => target_block = previous_block,
                    None => search_completed = true,
                }
            }
        }

        if !found_ancestor {
            None
        } else {
            let ancestor_block = target_block.clone();
            let mut i = self.longest_chain_queue.latest_block_id();

            while i > ancestor_block.id() {
                let hash = self.longest_chain_queue.block_hash_by_id(i as u64);
                let block = self.fork_tree.block_by_hash(&hash).unwrap();
                old_chain.blocks.push(block.clone());
                i = i - 1;
            }

            Some((ancestor_block, old_chain, new_chain))
        }
    }

    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub fn add_block(&mut self, block: Block) -> AddBlockEvent {
        if !self.validate_block(&block) {
            AddBlockEvent::InvalidTransaction
        } else if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {

            self.fork_tree.insert(block.hash(), block.clone());
            let latest_block_hash = &self.longest_chain_queue.latest_block_hash();

            // We're on the longest chain so this is simple for us!
            if latest_block_hash == block.previous_block_hash() {
                self.utxoset.roll_forward(&block);
                self.longest_chain_queue.roll_forward(block.hash());
                AddBlockEvent::AcceptedAsLongestChain
            // We are not on the longest chain, need to find the commone ancestor
            } else {

                if let Some((_ancestor_block, old_chain, new_chain)) = self.fork_chains(&block) {
                    if self.is_longer_chain(&new_chain, &old_chain) {

                        // Unwind the old chain
                        old_chain.blocks.iter().rev().for_each(|block| {
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                        });

                        // Wind up the new chain
                        new_chain.blocks.iter().for_each(|block| {
                            self.longest_chain_queue.roll_forward(block.hash());
                            self.utxoset.roll_forward(block);
                        });

                        AddBlockEvent::AcceptedAsNewLongestChain

                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        self.utxoset.roll_forward_on_potential_fork(&block);
                        AddBlockEvent::Accepted
                    }
                } else {
                    // We're in deep trouble here, we can't find any ancestor!
                    // Let's delete this from the forktree and pretend it never happened
                    self.fork_tree.remove(&(block.hash()));
                    AddBlockEvent::AncestorNotFound
                }
            }
        }
    }

    fn validate_block(&self, block: &Block) -> bool {
        !block.are_sigs_valid() | block.are_slips_spendable()
    }

    fn is_longer_chain(&self, new_chain: &ChainFork, old_chain: &ChainFork) -> bool {
        new_chain.blocks.len() >= old_chain.blocks.len()
    }
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
