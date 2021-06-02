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
    blocks: Vec<Block>,
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
        match self.longest_chain_queue.latest_block_hash() {
            Some(latest_block_hash) => self.fork_tree.block_by_hash(&latest_block_hash),
            None => None,
        }
    }

    // Get the block from fork_tree and check if the hash matches the one in longest_chain_queue
    fn _contains_block_hash_in_longest_chain(&self, block_hash: &Sha256Hash) -> bool {
        self.longest_chain_queue.contains_hash(block_hash)
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    /// If the block is in the fork
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Block> {
        self.fork_tree.block_by_hash(block_hash)
    }

    fn fork_chains(&self, block: &Block) -> Option<(Block, ChainFork, ChainFork)> {
        let mut old_chain = ChainFork { blocks: vec![] };
        let mut new_chain = ChainFork { blocks: vec![] };

        let mut target_block = block;
        let mut search_completed = false;

        while !search_completed {
            if self
                .longest_chain_queue
                .contains_hash_by_block_id(target_block.hash(), target_block.id())
            {
                search_completed = true;
            } else {
                new_chain.blocks.push(target_block.clone());
                match self
                    .fork_tree
                    .block_by_hash(target_block.previous_block_hash())
                {
                    Some(previous_block) => target_block = previous_block,
                    None => panic!("Target block's ancestor is not known."),
                }
            }
        }

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

    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub fn add_block(&mut self, block: Block) -> AddBlockEvent {
        //println!("add_block");
        let is_first_block = block.previous_block_hash() == &[0u8; 32]
            && !self.contains_block_hash(block.previous_block_hash());
        if !self.validate_block(&block) {
            AddBlockEvent::InvalidTransaction
        } else if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            self.fork_tree.insert(block.hash(), block.clone());
            let latest_block_hash = &self.longest_chain_queue.latest_block_hash();

            let is_new_lc_tip = !latest_block_hash.is_none()
                && &latest_block_hash.unwrap() == block.previous_block_hash();
            //println!("is_new_lc_tip {}", is_new_lc_tip);
            //println!("previous_block_hash Some({:?})", block.previous_block_hash());
            //println!("latest_block_hash   {:?}", latest_block_hash);
            if is_first_block || is_new_lc_tip {
                // First Block or we're new tip of the longest chain
                self.utxoset.roll_forward(&block);
                self.longest_chain_queue.roll_forward(block.hash());
                AddBlockEvent::AcceptedAsLongestChain
            // We are not on the longest chain, need to find the commone ancestor
            } else {
                if let Some((_ancestor_block, old_chain, new_chain)) = self.fork_chains(&block) {
                    if self.is_longer_chain(&new_chain, &old_chain) {
                        //println!("LONGER CHAIN!!");
                        //println!("{}", old_chain.blocks.len());
                        // Unwind the old chain
                        old_chain.blocks.iter().for_each(|block| {
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                        });

                        //println!("{}", new_chain.blocks.len());
                        // Wind up the new chain
                        new_chain.blocks.iter().rev().for_each(|block| {
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
        new_chain.blocks.len() > old_chain.blocks.len()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    // use crate::block::Block;
    // use crate::keypair::Keypair;
    // use crate::slip::{OutputSlip, SlipID};
    // use crate::transaction::{Transaction, TransactionType};
    // use secp256k1::Signature;

    use crate::test_utilities;
    #[test]
    fn validation_test() {
        let blockchain = Blockchain::new();
        let block = test_utilities::make_mock_block([0; 32]);
        assert!(blockchain.validate_block(&block));
    }
    #[test]
    fn add_block_test() {
        let mut blockchain = Blockchain::new();
        let block = test_utilities::make_mock_block([0; 32]);
        let mut prev_block_hash = block.hash().clone();
        let first_block_hash = block.hash().clone();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        for _n in 0..5 {
            let block = test_utilities::make_mock_block(prev_block_hash);
            prev_block_hash = block.hash().clone();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        }
        // make a fork
        let block = test_utilities::make_mock_block(first_block_hash);
        let mut prev_block_hash = block.hash().clone();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _n in 0..4 {
            let block = test_utilities::make_mock_block(prev_block_hash);
            prev_block_hash = block.hash().clone();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let block = test_utilities::make_mock_block(prev_block_hash);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make another fork
        let block = test_utilities::make_mock_block(first_block_hash);
        let mut prev_block_hash = block.hash().clone();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _n in 0..5 {
            let block = test_utilities::make_mock_block(prev_block_hash);
            prev_block_hash = block.hash().clone();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let block = test_utilities::make_mock_block(prev_block_hash);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make a 3rd fork by rolling back by 3 blocks
        prev_block_hash = blockchain
            .latest_block()
            .unwrap()
            .previous_block_hash()
            .clone();
        let mut prev_block = blockchain.get_block_by_hash(&prev_block_hash).unwrap();
        prev_block_hash = prev_block.previous_block_hash().clone();
        prev_block = blockchain.get_block_by_hash(&prev_block_hash).unwrap();
        prev_block_hash = prev_block.previous_block_hash().clone();

        for _n in 0..3 {
            let block = test_utilities::make_mock_block(prev_block_hash);
            prev_block_hash = block.hash().clone();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }

        let block = test_utilities::make_mock_block(prev_block_hash);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);
    }
}
