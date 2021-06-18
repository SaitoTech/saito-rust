use tracing::Level;

use crate::block::Block;
use crate::burnfee::BurnFee;
use crate::constants;
use crate::crypto::{verify_bytes_message, Sha256Hash};
use crate::forktree::ForkTree;
use crate::golden_ticket::GoldenTicket;
use crate::longest_chain_queue::LongestChainQueue;
use crate::storage::Storage;
use crate::time::TracingTimer;
use crate::transaction::{Transaction, TransactionType};
use crate::utxoset::UtxoSet;

use std::sync::Arc;
use std::sync::Mutex;

use rayon::prelude::*;

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
    pub async fn add_block(&mut self, block: Block) -> AddBlockEvent {
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        let mut tracing_tracker = TracingTimer::new();
        let is_first_block = block.previous_block_hash() == &[0u8; 32]
            && !self.contains_block_hash(block.previous_block_hash());
        if self.contains_block_hash(&(block.hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(block.previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            event!(
                Level::TRACE,
                "  COMPUTE BASIC BLOCK STATUS: {:?}",
                tracing_tracker.time_since_last()
            );
            let fork_chains: ForkChains = self.find_fork_chains(&block);
            event!(
                Level::TRACE,
                "                COMPUTE FORK: {:?}",
                tracing_tracker.time_since_last()
            );
            if !self.validate_block(&block, &fork_chains) {
                AddBlockEvent::InvalidBlock
            } else {
                event!(
                    Level::TRACE,
                    "             VALIDATED BLOCK: {:?}",
                    tracing_tracker.time_since_last()
                );

                let latest_block_hash = &self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = !latest_block_hash.is_none()
                    && &latest_block_hash.unwrap() == block.previous_block_hash();
                if is_first_block || is_new_lc_tip {
                    event!(
                        Level::TRACE,
                        "                  SIMPLE ADD: {:?}",
                        tracing_tracker.time_since_last()
                    );
                    // First Block or we'e new tip of the longest chain
                    self.longest_chain_queue.roll_forward(block.hash());
                    event!(
                        Level::TRACE,
                        "                  LC ROLLFWD: {:?}",
                        tracing_tracker.time_since_last()
                    );
                    self.utxoset.roll_forward(&block);
                    event!(
                        Level::TRACE,
                        "                UTXO ROLLFWD: {:?}",
                        tracing_tracker.time_since_last()
                    );
                    self.fork_tree.insert(block.hash(), block.clone()).unwrap();
                    event!(
                        Level::TRACE,
                        "            FORKTREE FOLLFWD: {:?}",
                        tracing_tracker.time_since_last()
                    );
                    self.storage.roll_forward(&block).await;
                    event!(
                        Level::TRACE,
                        "             STORAGE ROLLFWD: {:?}",
                        tracing_tracker.time_since_last()
                    );

                    AddBlockEvent::AcceptedAsLongestChain
                // We are not on the longest chain, need to find the commone ancestor
                } else {
                    if self.is_longer_chain(&fork_chains.new_chain, &fork_chains.old_chain) {
                        self.fork_tree.insert(block.hash(), block).unwrap();

                        // Unwind the old chain
                        for block_hash in fork_chains.old_chain.iter() {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            self.utxoset.roll_back(block);
                            self.storage.roll_back(&block);
                        }

                        // Wind up the new chain
                        for block_hash in fork_chains.new_chain.iter().rev() {
                            let block: &Block = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_forward(block.hash());
                            self.utxoset.roll_forward(block);
                            self.storage.roll_forward(block).await;
                        }

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
        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.previous_block_hash();
        if previous_block_hash == &[0; 32] && block.id() == 0 {
            return true;
        }
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
        // Validate the burnfee
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
            .par_iter()
            .all(|tx| self.validate_transaction(previous_block, tx, fork_chains));

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
                        return false;
                    };

                    // validate our slips
                    let inputs_are_valid = tx.core.inputs().iter().all(|input| {
                        if fork_chains.old_chain.len() == 0 {
                            return self
                                .utxoset
                                .is_slip_spendable_at_block(input, previous_block.id());
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
    use crate::block::BlockCore;
    use crate::block::TREASURY;
    use crate::keypair::Keypair;
    use crate::slip::OutputSlip;
    use crate::slip::SlipID;
    use crate::slip::SlipType;
    use crate::test_utilities;
    use crate::test_utilities::MockTimestampGenerator;
    use crate::time::create_timestamp;
    use crate::transaction::TransactionCore;
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
    #[tokio::test]
    async fn double_spend_on_fork_test() {
        let keypair = Keypair::new();

        let mut mock_timestamp_generator = MockTimestampGenerator::new();
        let (mut blockchain, mut slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * EPOCH_LENGTH).await;
        let block = blockchain.latest_block().unwrap();

        let mut prev_block_hash = block.hash();
        let mut prev_block_id = block.id();
        let mut prev_burn_fee = block.start_burnfee();
        let mut prev_timestamp = block.timestamp();
        let first_block_hash = block.hash();
        let first_block_id = block.id();
        let first_burn_fee = block.start_burnfee();
        let first_timestamp = block.timestamp();

        for _ in 0..3 {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));

            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
            assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        }
        // make a fork
        let timestamp = mock_timestamp_generator.next() + 1;
        let block_y1 = Block::new(BlockCore::new(
            first_block_id + 1,
            timestamp,
            first_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(first_burn_fee, timestamp, first_timestamp),
            0.0,
            &mut vec![],
        ));

        let result: AddBlockEvent = blockchain.add_block(block_y1.clone()).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        // make a valid tx to add to block y2
        let slip_pair = slips.pop().unwrap();
        let to_slip = OutputSlip::new(
            *keypair.public_key(),
            SlipType::Normal,
            slip_pair.1.amount(),
        );
        let tx = Transaction::create_signature(
            TransactionCore::new(
                create_timestamp(),
                vec![slip_pair.0],
                vec![to_slip],
                TransactionType::Normal,
                vec![],
            ),
            &keypair,
        );
        let double_spent_attempt_input = SlipID::new(tx.hash(), 0);
        let double_spent_attempt_to_slip = OutputSlip::new(
            *keypair.public_key(),
            SlipType::Normal,
            slip_pair.1.amount(),
        );

        let double_spent_attempt_tx = Transaction::create_signature(
            TransactionCore::new(
                create_timestamp(),
                vec![double_spent_attempt_input],
                vec![double_spent_attempt_to_slip],
                TransactionType::Normal,
                vec![],
            ),
            &keypair,
        );
        let mut txs = vec![tx];
        let timestamp = mock_timestamp_generator.next();
        let block_y2 = Block::new(BlockCore::new(
            block_y1.id() + 1,
            timestamp,
            block_y1.hash(),
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(
                block_y1.start_burnfee(),
                timestamp,
                block_y1.timestamp(),
            ),
            0.0,
            &mut txs,
        ));

        let result: AddBlockEvent = blockchain.add_block(block_y2.clone()).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        let timestamp = mock_timestamp_generator.next();
        let block_z2 = Block::new(BlockCore::new(
            block_y1.id() + 1,
            timestamp,
            block_y1.hash(),
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(
                block_y1.start_burnfee(),
                timestamp,
                block_y1.timestamp(),
            ),
            0.0,
            &mut vec![],
        ));
        let result: AddBlockEvent = blockchain.add_block(block_z2.clone()).await;

        assert_eq!(result, AddBlockEvent::Accepted);

        let mut double_spent_attempt_txs = vec![double_spent_attempt_tx.clone()];

        let timestamp = mock_timestamp_generator.next();
        let block_z3 = Block::new(BlockCore::new(
            block_z2.id() + 1,
            timestamp,
            block_z2.hash(),
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(
                block_z2.start_burnfee(),
                timestamp,
                block_z2.timestamp(),
            ),
            0.0,
            &mut double_spent_attempt_txs,
        ));

        let result: AddBlockEvent = blockchain.add_block(block_z3.clone()).await;
        // z3 is rejected because it contains an unspendable transaction from y2
        assert_eq!(result, AddBlockEvent::InvalidBlock);

        let timestamp = mock_timestamp_generator.next();
        let block_y3 = Block::new(BlockCore::new(
            block_y2.id() + 1,
            timestamp,
            block_y2.hash(),
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(
                block_y2.start_burnfee(),
                timestamp,
                block_y2.timestamp(),
            ),
            0.0,
            &mut double_spent_attempt_txs,
        ));

        let result: AddBlockEvent = blockchain.add_block(block_y3.clone()).await;
        // y3 also spends from y2 but is accepted because y2 is it's parent
        assert_eq!(result, AddBlockEvent::Accepted);
    }

    #[tokio::test]
    async fn add_block_test() {
        let keypair = Keypair::new();
        let mut mock_timestamp_generator = MockTimestampGenerator::new();
        let (mut blockchain, _slips) =
            test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * EPOCH_LENGTH).await;

        let block = blockchain.latest_block().unwrap();
        let mut prev_block_hash = block.hash();
        let mut prev_block_id = block.id();
        let mut prev_burn_fee = block.start_burnfee();
        let mut prev_timestamp = block.timestamp();
        let first_block_hash = block.hash();
        let first_block_id = block.id();
        let first_burn_fee = block.start_burnfee();
        let first_timestamp = block.timestamp();

        for n in 0..5 as i32 {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));

            prev_block_hash = block.hash().clone();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
            // println!("{:?}", result);
            assert_eq!(blockchain.latest_block().unwrap().id(), (n + 1) as u64);
            assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
        }
        // make a fork
        let timestamp = mock_timestamp_generator.next() + 1;
        let block = Block::new(BlockCore::new(
            first_block_id + 1,
            timestamp,
            first_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(first_burn_fee, timestamp, first_timestamp),
            0.0,
            &mut vec![],
        ));
        prev_block_hash = block.hash();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();

        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;

        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _ in 0..4 as i32 {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));
            prev_block_hash = block.hash();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
            // println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            prev_block_id + 1,
            timestamp,
            prev_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
            0.0,
            &mut vec![],
        ));
        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
        // println!("{:?}", result);
        assert_eq!(blockchain.latest_block().unwrap().hash(), block.hash());
        assert_eq!(blockchain.latest_block().unwrap().id(), block.id());

        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make another fork
        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            first_block_id + 1,
            timestamp,
            first_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(first_burn_fee, timestamp, first_timestamp),
            0.0,
            &mut vec![],
        ));
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();
        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        for _n in 0..5 as i32 {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));
            prev_block_hash = block.hash();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
            // println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }
        // new longest chain tip on top of the fork
        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            prev_block_id + 1,
            timestamp,
            prev_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
            0.0,
            &mut vec![],
        ));
        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
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
        prev_burn_fee = prev_block.start_burnfee();
        prev_timestamp = prev_block.timestamp();

        for _ in 0..2 as i32 {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));
            prev_block_hash = block.hash();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
            // println!("{:?}", result);
            assert_eq!(result, AddBlockEvent::Accepted);
        }

        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            prev_block_id + 1,
            timestamp,
            prev_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
            0.0,
            &mut vec![],
        ));
        prev_block_hash = block.hash();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();
        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        // make another block because why not
        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            prev_block_id + 1,
            timestamp,
            prev_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
            0.0,
            &mut vec![],
        ));
        let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        // make another fork
        let timestamp = mock_timestamp_generator.next();
        let block = Block::new(BlockCore::new(
            first_block_id + 1,
            timestamp,
            first_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(first_burn_fee, timestamp, first_timestamp),
            0.0,
            &mut vec![],
        ));
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();
        let result = blockchain.add_block(block.clone()).await;
        // println!("{:?}", result);
        assert_eq!(result, AddBlockEvent::Accepted);

        // run past 2x EPOCH_LENGTH blocks to test the epoch a bit
        for _n in 0..(2 * EPOCH_LENGTH + 1) {
            let timestamp = mock_timestamp_generator.next();
            let block = Block::new(BlockCore::new(
                prev_block_id + 1,
                timestamp,
                prev_block_hash,
                *keypair.public_key(),
                0,
                TREASURY,
                BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
                0.0,
                &mut vec![],
            ));
            prev_block_hash = block.hash();
            prev_block_id = block.id();
            prev_burn_fee = block.start_burnfee();
            prev_timestamp = block.timestamp();
            let result = blockchain.add_block(block.clone()).await;

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
