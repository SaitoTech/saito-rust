use crate::block::Block;
use crate::burnfee::BurnFee;
use crate::crypto::{verify_bytes_message, Sha256Hash};
use crate::forktree::ForkTree;
use crate::golden_ticket::GoldenTicket;
use crate::longest_chain_queue::LongestChainQueue;
use crate::transaction::{Transaction, TransactionType};
use crate::utxoset::UtxoSet;
use std::sync::Arc;
use std::sync::Mutex;

// A lazy-loaded global static reference to Blockchain. For now, we will simply treat
// everything(utxoset, mempool, etc) as a single shared resource which is managed by blockchain.
// In the future we may want to create separate globals for some of the resources being held
// by blockchain by giving them a similar lazy_static Arc<Mutex> treatment, but we will wait
// to see the performance of this simple state management scheme before we try to optimize.
lazy_static! {
    // We use Arc for thread-safe reference counting and Mutex for thread-safe mutabilitity
    pub static ref BLOCKCHAIN_GLOBAL: Arc<Mutex<Blockchain>> = Arc::new(Mutex::new(Blockchain::new()));
}

// TODO It might be more performant to keep everything in a single thread and use a
// RefCell instead of an Arc<Mutex>. However, thread_local requiers a closure(or FnOnce)
// to get a reference to BLOCKCHAIN_THREAD_LOCAL. Rust support of async closures is unstable,
// so getting this to work has proved to be more tricky than simply using an Arc<Mutex>. In
// the interest of not doing premature optimization, we will use an Arc<Mutex> and let tokio
// do whatever it wants are far as using threads or not. This may turn out to be more performany anyway
// even though we are paying the cost of atomic reference counting. However, it may be worth the effort to
// try to configure tokio to run on a single thread and use a thread_local global instead sometime in
// the future.
// thread_local!(pub static BLOCKCHAIN_THREAD_LOCAL: RefCell<Blockchain> = RefCell::new(Blockchain::new()));

/// Enumerated types of `Transaction`s to be handlded by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum AddBlockEvent {
    AcceptedAsNewLongestChain,
    AcceptedAsLongestChain,
    Accepted,
    AlreadyKnown,
    AncestorNotFound,
    ParentNotFound,
    InvalidBlock,
}

pub struct ChainFork {
    // TODO -- add lifetime and reference to block
    blocks: Vec<Sha256Hash>,
}
pub type ForkTuple = (Block, ChainFork, ChainFork);

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// Hashmap of slips used by the network
    pub utxoset: UtxoSet,
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

    /// If the block is in the fork
    pub fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Block> {
        self.fork_tree.block_by_hash(block_hash)
    }

    // Get the block from fork_tree and check if the hash matches the one in longest_chain_queue
    fn _contains_block_hash_in_longest_chain(&self, block_hash: Sha256Hash, block_id: u64) -> bool {
        self.longest_chain_queue.contains_hash(&block_hash)
            && self
                .longest_chain_queue
                .contains_hash_by_block_id(block_hash, block_id)
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    fn fork_chains(&self, block: &Block) -> ForkTuple {
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
                new_chain.blocks.push(target_block.hash());
                match self
                    .fork_tree
                    .block_by_hash(target_block.previous_block_hash())
                {
                    Some(previous_block) => target_block = previous_block,
                    None => search_completed = true,
                }
            }
        }

        let ancestor_block = target_block.clone();
        let i: Option<u64> = self.longest_chain_queue.latest_block_id();
        // TODO do this in a more rusty way
        if !i.is_none() {
            let mut i = i.unwrap();
            while i > ancestor_block.id() {
                let hash = self.longest_chain_queue.block_hash_by_id(i as u64);
                let block = self.fork_tree.block_by_hash(&hash).unwrap();
                old_chain.blocks.push(block.hash());
                i = i - 1;
            }
        }

        (ancestor_block, old_chain, new_chain)
    }
    // TODO we'd like to use these functions to simply the code but there is some borrowing
    // problem with self
    // fn roll_forward(&mut self, block: &Block) {
    //     // self.utxoset.roll_forward(&block);
    //     // self.longest_chain_queue.roll_forward(block.hash().clone());
    //     self.longest_chain_queue.roll_forward(block.hash());
    //     self.utxoset.roll_forward(&block);
    // }
    // fn roll_back(&mut self, block: &Block) {
    //     self.longest_chain_queue.roll_back();
    //     self.utxoset.roll_back(block);
    // }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub fn add_block(&mut self, block: Block) -> AddBlockEvent {
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        let is_first_block = block.previous_block_hash() == &[0u8; 32]
            && !self.contains_block_hash(block.previous_block_hash());
        if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            let fork_tuple: ForkTuple = self.fork_chains(&block);
            if !self.validate_block(&block, &fork_tuple) {
                AddBlockEvent::InvalidBlock
            } else {
                let latest_block_hash = &self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = !latest_block_hash.is_none()
                    && &latest_block_hash.unwrap() == block.previous_block_hash();
                if is_first_block || is_new_lc_tip {
                    // First Block or we're new tip of the longest chain
                    self.longest_chain_queue.roll_forward(block.hash());
                    self.utxoset.roll_forward(&block);
                    self.fork_tree.insert(block.hash(), block).unwrap();
                    AddBlockEvent::AcceptedAsLongestChain
                // We are not on the longest chain, need to find the commone ancestor
                } else {
                    // TODO give the ForkTuple some named fields and use those instead here
                    let old_chain = fork_tuple.1;
                    let new_chain = fork_tuple.2;
                    if self.is_longer_chain(&new_chain, &old_chain) {
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        // Unwind the old chain
                        old_chain.blocks.iter().for_each(|block_hash| {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                        });
                        // Wind up the new chain

                        new_chain.blocks.iter().rev().for_each(|block_hash| {
                            let block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_forward(block.hash());
                            self.utxoset.roll_forward(&block);
                        });

                        AddBlockEvent::AcceptedAsNewLongestChain
                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        self.utxoset.roll_forward_on_potential_fork(&block);
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        AddBlockEvent::Accepted
                    }
                }
            }
        }
    }

    fn validate_block(&self, block: &Block, fork_tuple: &ForkTuple) -> bool {
        // !block.are_sigs_valid() | block.are_slips_spendable()

        // how do we validate blocks?
        // validate block here first

        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.previous_block_hash();
        if previous_block_hash == &[0; 32] && block.id() == 0 {
            return true;
        }

        // If the previous block hash doesn't exist in the ForkTree, it's rejected
        if !self.fork_tree.contains_block_hash(previous_block_hash) {
            println!("BLOCK ALREADY IN FORK TREE");
            return false;
        }
        if self
            .fork_tree
            .block_by_hash(previous_block_hash)
            .unwrap()
            .id()
            + 1
            != block.id()
        {
            println!("ID NOT INCREMENTED PROPERLY");
            return false;
        }

        // TODO -- include checks on difficulty and paysplit here to validate

        // Validate the burnfee
        let previous_block = self.fork_tree.block_by_hash(previous_block_hash).unwrap();
        if block.start_burnfee()
            != BurnFee::burn_fee_adjustment_calculation(
                previous_block.start_burnfee() as f64,
                block.timestamp(),
                previous_block.timestamp(),
            ) as f64
        {
            println!("BURNFEE IS WRONG");
            return false;
        }

        // TODO: This should probably be >= but currently we are producing mock blocks very
        // quickly and this won't pass.
        if previous_block.timestamp() > block.timestamp() {
            println!("INCORRECT TIMESTAMPS");
            return false;
        }

        let golden_ticket_count = block.transactions().iter().fold(0, |acc, tx| {
            if tx.core.broadcast_type() == &TransactionType::GoldenTicket {
                acc + 1
            } else {
                acc
            }
        });
        if golden_ticket_count > 1 {
            println!("TOO MANY GOLDEN TICKETS");
            return false;
        }

        let transactions_valid = block
            .transactions()
            .iter()
            .all(|tx| self.validate_transaction(previous_block, block, tx, fork_tuple));

        transactions_valid
    }

    fn validate_transaction(
        &self,
        previous_block: &Block,
        block: &Block,
        tx: &Transaction,
        fork_tuple: &ForkTuple,
    ) -> bool {
        match tx.core.broadcast_type() {
            TransactionType::Normal => {
                if tx.core.inputs().len() == 0 && tx.core.outputs().len() == 0 {
                    return true;
                }

                if let Some(address) = self.utxoset.get_receiver_for_slips(tx.core.inputs()) {
                    let serialize_tx: Vec<u8> = tx.core.clone().into();
                    if !verify_bytes_message(&serialize_tx[..], &tx.signature(), address) {
                        println!("SIGNATURE IS NOT VALID");
                        return false;
                    };

                    // validate our slips
                    let inputs_are_valid = tx.core.inputs().iter().all(|input| {
                        return !self
                            .utxoset
                            .is_slip_spent_at_block(input, block, fork_tuple);
                    });

                    if !inputs_are_valid {
                        println!("INPUTS ARE NOT FVALID");
                        return false;
                    }

                    // valuidate that inputs are unspent
                    let input_amt: u64 = tx
                        .core
                        .inputs()
                        .iter()
                        .map(|input| {
                            self.utxoset
                                .output_slip_from_slip_id(input)
                                .unwrap()
                                .amount()
                        })
                        .sum();

                    let output_amt: u64 =
                        tx.core.outputs().iter().map(|output| output.amount()).sum();

                    if input_amt < output_amt {
                        println!("INPUTS DO NOT MATCH OUTPUTS");
                        return false;
                    }

                    return true;
                } else {
                    println!("SLIPS ARE NOT FROM SAME RECEIVER");
                    // no input slips, thus no output slips.
                    return false;
                }
            }
            TransactionType::GoldenTicket => {
                // need to validate the golden ticket correctly
                let golden_ticket = GoldenTicket::from(tx.core.message().clone());

                // println!("TARGET: {:?}", golden_ticket.target);
                // println!("PREVIOUS BLOCK: {:?}", previous_block.hash());

                if golden_ticket.target != previous_block.hash() {
                    println!("INVALID TARGET");
                    return false;
                }

                return true;
            }
        }
    }

    fn is_longer_chain(&self, new_chain: &ChainFork, old_chain: &ChainFork) -> bool {
        new_chain.blocks.len() > old_chain.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;
    use crate::test_utilities;
    
    #[test]
    #[ignore]
    fn add_block_test() {
        let keypair = Keypair::new();
        let (mut blockchain, mut slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 1200);

        println!("BLOCKCHAIN CREATED");

        let block = test_utilities::make_mock_block(&keypair, [0; 32], 0, slips.pop().unwrap().0);
        let mut prev_block_hash = block.hash().clone();
        let mut prev_block_id = block.id();
        let first_block_hash = block.hash().clone();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        for _n in 0..5 {
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        }
        // make a fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _n in 0..4 {
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let block = test_utilities::make_mock_block(
            &keypair,
            prev_block_hash,
            prev_block_id + 1,
            slips.pop().unwrap().0,
        );
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make another fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _n in 0..5 {
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let block = test_utilities::make_mock_block(
            &keypair,
            prev_block_hash,
            prev_block_id + 1,
            slips.pop().unwrap().0,
        );
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        // println!("{:?}", result);
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
        prev_block_hash = prev_block.hash().clone();
        prev_block_id = prev_block.id();

        for _n in 0..2 {
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            let result: AddBlockEvent = blockchain.add_block(block.clone());
            //println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }

        let block = test_utilities::make_mock_block(
            &keypair,
            prev_block_hash,
            prev_block_id + 1,
            slips.pop().unwrap().0,
        );
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make another fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        //println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        println!("RUNNING EPOCH");
        // run 1000 blocks to test the epoch a bit
        for _n in 0..1000 {
            println!("{}", _n);
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            blockchain.add_block(block.clone());
            //println!("{} {:?}", block.id(), result);
        }

        // TODO fix this problem:
        // If you build on a fork which has an ancestor which is older than 2x EPOCH_LENGTH, the
        // fork tree will happily accept the block but fork_chains will try to find an ancestor
        // which is no longer present in the longest_chain_queue. The correct solution to this is
        // to expire forks(delete all blocks from fork_tree) which have a common ancestor that is
        // expiring(falling off the longest_chain_queue). Once this is done, the following code
        // should simply return ParentNotFound instead of causing a panic.
        // prev_block_hash = block.hash().clone();
        // prev_block_id = block.id();
        // let block = test_utilities::make_mock_block(prev_block_hash, prev_block_id + 1);
        // let result: AddBlockEvent = blockchain.add_block(block.clone());
    }
}
