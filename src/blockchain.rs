use crate::block::Block;
use crate::burnfee::BurnFee;
use crate::constants;
use crate::crypto::{verify_bytes_message, Sha256Hash};
use crate::forktree::ForkTree;
use crate::golden_ticket::GoldenTicket;
use crate::longest_chain_queue::LongestChainQueue;
use crate::storage::Storage;
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

pub struct ForkChains {
    pub ancestor_block: Block,
    pub new_chain: Vec<Sha256Hash>,
    pub old_chain: Vec<Sha256Hash>,
}
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
    // storage
    pub storage: Storage,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            utxoset: UtxoSet::new(),
            longest_chain_queue: LongestChainQueue::new(),
            fork_tree: ForkTree::new(),
            storage: Storage::new(String::from(constants::BLOCKS_DIR)),
        }
    }
    pub fn new_mock(blocks_dir: String) -> Self {
        Blockchain {
            utxoset: UtxoSet::new(),
            longest_chain_queue: LongestChainQueue::new(),
            fork_tree: ForkTree::new(),
            storage: Storage::new(blocks_dir),
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

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    fn find_fork_chains(&self, block: &Block) -> ForkChains {
        let mut old_chain = vec![];
        let mut new_chain = vec![];

        let mut target_block = block;
        let mut search_completed = false;

        while !search_completed {
            if target_block.id() == 0
                || self
                    .longest_chain_queue
                    .contains_hash_by_block_id(target_block.hash(), target_block.id())
            {
                search_completed = true;
            } else {
                new_chain.push(target_block.hash());
                match self
                    .fork_tree
                    .block_by_hash(target_block.previous_block_hash())
                {
                    Some(previous_block) => target_block = previous_block,
                    None => {
                        search_completed = true;
                    }
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
                old_chain.push(block.hash());
                i = i - 1;
            }
        }
        ForkChains {
            ancestor_block: ancestor_block,
            old_chain: old_chain,
            new_chain: new_chain,
        }
    }
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
            let fork_chains: ForkChains = self.find_fork_chains(&block);
            if !self.validate_block(&block, &fork_chains) {
                AddBlockEvent::InvalidBlock
            } else {
                println!("FINISHED VALIDATING");
                let latest_block_hash = &self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = !latest_block_hash.is_none()
                    && &latest_block_hash.unwrap() == block.previous_block_hash();
                println!("IS FIRST BLOCK: {:?}", is_first_block);
                println!("IS LC TIP: {:?}", is_new_lc_tip);
                if is_first_block || is_new_lc_tip {
                    // First Block or we'e new tip of the longest chain
                    println!("LONGEST CHAIN QUEUE");
                    self.longest_chain_queue.roll_forward(block.hash());

                    println!("UTXOSET ROLL FORWARD");
                    self.utxoset.roll_forward(&block);

                    println!("FORK_TREE INSERT");
                    self.fork_tree.insert(block.hash(), block.clone()).unwrap();
                    self.storage.roll_forward(&block);
                    println!("Block accepted, written to disk");
                    AddBlockEvent::AcceptedAsLongestChain
                // We are not on the longest chain, need to find the commone ancestor
                } else {
                    if self.is_longer_chain(&fork_chains.new_chain, &fork_chains.old_chain) {
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        // Unwind the old chain
                        fork_chains.old_chain.iter().for_each(|block_hash| {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                            self.storage.roll_back(&block);
                        });
                        // Wind up the new chain
                        fork_chains.new_chain.iter().rev().for_each(|block_hash| {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_forward(block.hash());
                            self.utxoset.roll_forward(block);
                            self.storage.roll_forward(&block);
                        });

                        AddBlockEvent::AcceptedAsNewLongestChain
                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        self.utxoset.roll_forward_on_fork(&block);
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        AddBlockEvent::Accepted
                    }
                }
            }
        }
    }

    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub async fn add_block_async(&mut self, block: Block) -> AddBlockEvent {
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        let is_first_block = block.previous_block_hash() == &[0u8; 32]
            && !self.contains_block_hash(block.previous_block_hash());
        if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            let fork_chains: ForkChains = self.find_fork_chains(&block);
            if !self.validate_block(&block, &fork_chains) {
                AddBlockEvent::InvalidBlock
            } else {
                println!("FINISHED VALIDATING");
                let latest_block_hash = &self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = !latest_block_hash.is_none()
                    && &latest_block_hash.unwrap() == block.previous_block_hash();
                println!("IS FIRST BLOCK: {:?}", is_first_block);
                println!("IS LC TIP: {:?}", is_new_lc_tip);
                if is_first_block || is_new_lc_tip {
                    // First Block or we'e new tip of the longest chain
                    println!("LONGEST CHAIN QUEUE");
                    self.longest_chain_queue.roll_forward(block.hash());

                    println!("UTXOSET ROLL FORWARD");
                    self.utxoset.roll_forward(&block);

                    println!("FORK_TREE INSERT");
                    self.fork_tree.insert(block.hash(), block.clone()).unwrap();
                    self.storage.roll_forward_async(&block).await;
                    println!("Block accepted, written to disk");
                    AddBlockEvent::AcceptedAsLongestChain
                // We are not on the longest chain, need to find the commone ancestor
                } else {
                    if self.is_longer_chain(&fork_chains.new_chain, &fork_chains.old_chain) {
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        // Unwind the old chain
                        fork_chains.old_chain.iter().for_each(|block_hash| {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                            self.storage.roll_back(&block);
                        });
                        // Wind up the new chain
                        // let tasks = fork_chains.new_chain.iter().rev().map(|block_hash| {
                        fork_chains.new_chain.iter().rev().for_each(|block_hash| {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_forward(block.hash());
                            self.utxoset.roll_forward(block);
                            self.storage.roll_forward(&block)
                            // tokio::spawn(async {
                            //     let storage = Storage::new(blocks_dir);
                            //     storage.roll_forward_async(&block).await
                            // })
                        });

                        // for task in tasks {
                        //     task.await.unwrap();
                        // }

                        AddBlockEvent::AcceptedAsNewLongestChain
                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        self.utxoset.roll_forward_on_fork(&block);
                        self.fork_tree.insert(block.hash(), block).unwrap();
                        AddBlockEvent::Accepted
                    }
                }
            }
        }
    }

    fn validate_block(&self, block: &Block, fork_chains: &ForkChains) -> bool {
        println!("VALIDATING BLOCK");
        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.previous_block_hash();
        if previous_block_hash == &[0; 32] && block.id() == 0 {
            return true;
        }

        println!("CHECKING BLOCK HASH");
        // If the previous block hash doesn't exist in the ForkTree, it's rejected
        if !self.fork_tree.contains_block_hash(previous_block_hash) {
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
            return false;
        }

        // TODO -- include checks on difficulty and paysplit here to validate

        println!("VALIDATE BURNFEE");
        // Validate the burnfee
        // TODO put this back, it's breaking add_block_test. test probably just needs to be updated
        let previous_block = self.fork_tree.block_by_hash(previous_block_hash).unwrap();
        if block.start_burnfee()
            != BurnFee::burn_fee_adjustment_calculation(
                previous_block.start_burnfee() as f64,
                block.timestamp(),
                previous_block.timestamp(),
            ) as f64
        {
            return false;
        }

        println!("CHECK WORK NEEDED");
        // validate the fees match the work required to make a block
        // let work_available: u64 = block
        //     .transactions()
        //     .iter()
        //     .map(|tx| self.utxoset.transaction_routing_fees(tx))
        //     .sum();
        // if work_available
        //     < BurnFee::return_work_needed(
        //         previous_block.start_burnfee() as f64,
        //         block.timestamp(),
        //         previous_block.timestamp(),
        //     )
        // {
        //     return false;
        // }

        // TODO: This should probably be >= but currently we are producing mock blocks very
        // quickly and this won't pass.
        if previous_block.timestamp() > block.timestamp() {
            return false;
        }

        println!("GOLDEN TICKET COUNT");
        let golden_ticket_count = block.transactions().iter().fold(0, |acc, tx| {
            if tx.core.broadcast_type() == &TransactionType::GoldenTicket {
                acc + 1
            } else {
                acc
            }
        });
        if golden_ticket_count > 1 {
            return false;
        }
        let transactions_valid = block
            .transactions()
            .iter()
            .all(|tx| self.validate_transaction(previous_block, tx, fork_chains));

        println!("FINISHED VALIDATING");
        transactions_valid
    }

    fn validate_transaction(
        &self,
        previous_block: &Block,
        tx: &Transaction,
        fork_chains: &ForkChains,
    ) -> bool {
        match tx.core.broadcast_type() {
            TransactionType::Normal => {
                if tx.core.inputs().len() == 0 && tx.core.outputs().len() == 0 {
                    return true;
                }

                if let Some(address) = self.utxoset.get_receiver_for_inputs(tx.core.inputs()) {
                    if !verify_bytes_message(&tx.hash(), &tx.signature(), address) {
                        println!("SIGNATURE IS NOT VALID");
                        return false;
                    };

                    // validate our slips
                    let inputs_are_valid = tx.core.inputs().iter().all(|input| {
                        if fork_chains.old_chain.len() == 0 {
                            return self
                                .utxoset
                                .is_slip_spendable_at_block(input, previous_block);
                        } else {
                            return self
                                .utxoset
                                .is_slip_spendable_at_fork_block(input, fork_chains);
                        }
                    });

                    if !inputs_are_valid {
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
                        return false;
                    }

                    return true;
                } else {
                    // no input slips, thus no output slips.
                    return false;
                }
            }
            TransactionType::GoldenTicket => {
                // need to validate the golden ticket correctly
                let golden_ticket = GoldenTicket::from(tx.core.message().clone());
                if golden_ticket.target != previous_block.hash() {
                    return false;
                }
                return true;
            }
        }
    }

    fn is_longer_chain(&self, new_chain: &Vec<Sha256Hash>, old_chain: &Vec<Sha256Hash>) -> bool {
        new_chain.len() > old_chain.len()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::keypair::Keypair;
    use crate::test_utilities;
    include!(concat!(env!("OUT_DIR"), "/constants.rs"));

    fn teardown() -> std::io::Result<()> {
        let dir_path = String::from("data/test/blocks/");
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }
        std::fs::File::create("./src/data/blocks/empty")?;

        Ok(())
    }

    // #[ignore]
    #[test]
    fn add_block_test() {
        let keypair = Keypair::new();
        let (mut blockchain, mut slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * EPOCH_LENGTH);

        //let block = test_utilities::make_mock_block(&keypair, [0; 32], 0, slips.pop().unwrap().0);
        let block = blockchain.latest_block().unwrap();
        let mut prev_block_hash = block.hash().clone();
        let mut prev_block_id = block.id();
        let first_block_hash = block.hash().clone();
        // let result: AddBlockEvent = blockchain.add_block(block.clone());
        // // println!("{:?}", result);
        // assert_eq!(result, AddBlockEvent::AlreadyKnown);

        println!("prev_block_id {}", prev_block_id);
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
            // println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        }
        // make a fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        // println!("{:?}", result);
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
            // println!("{:?}", result);
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

        // make another fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        // println!("{:?}", result);
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
            // println!("{:?}", result);
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
            // println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }

        let block = test_utilities::make_mock_block(
            &keypair,
            prev_block_hash,
            prev_block_id + 1,
            slips.pop().unwrap().0,
        );
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        let block = test_utilities::make_mock_block(
            &keypair,
            prev_block_hash,
            prev_block_id + 1,
            slips.pop().unwrap().0,
        );
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        // make another fork
        let block =
            test_utilities::make_mock_block(&keypair, first_block_hash, 1, slips.pop().unwrap().0);
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        // run past 2x EPOCH_LENGTH blocks to test the epoch a bit
        for _n in 0..(2 * EPOCH_LENGTH + 1) {
            let block = test_utilities::make_mock_block(
                &keypair,
                prev_block_hash,
                prev_block_id + 1,
                slips.pop().unwrap().0,
            );
            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            let result = blockchain.add_block(block.clone());
            // println!("{} {:?}", block.id(), result);
            // weak assertion is okay here, we just want to make sure nothing panics
            assert!(
                result == AddBlockEvent::Accepted
                    || result == AddBlockEvent::AcceptedAsLongestChain
                    || result == AddBlockEvent::AcceptedAsNewLongestChain
            );
        }

        teardown().expect("Teardown failed");
    }
}
