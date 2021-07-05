use crate::block::Block;
use crate::blockring::BlockRing;
use crate::consensus::SaitoMessage;
use crate::crypto::{SaitoHash, SaitoUTXOSetKey};
use crate::storage::Storage;
use crate::time::create_timestamp;
use crate::wallet::Wallet;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use ahash::AHashMap;

#[derive(Debug)]
pub struct Blockchain {
    pub utxoset: AHashMap<SaitoUTXOSetKey, u64>,
    pub blockring: BlockRing,
    pub blocks: AHashMap<SaitoHash, Block>,
    pub wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
}

impl Blockchain {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new(wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        Blockchain {
            utxoset: AHashMap::new(),
            blockring: BlockRing::new(),
            blocks: AHashMap::new(),
            wallet_lock,
            broadcast_channel_sender: None,
        }
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    pub async fn add_block(&mut self, block: Block) {
        println!(
            " ... add_block {} start: {:?}",
            block.get_id(),
            create_timestamp()
        );
        println!(" ... hash: {:?}", block.get_hash());
        //println!(" ... txs in block: {:?}", block.transactions.len());
        //println!(" ... w/ prev bhsh: {:?}", block.get_previous_block_hash());

        //
        // start by extracting some variables that we will use
        // repeatedly in the course of adding this block to the
        // blockchain and our various indices.
        //
        let block_hash = block.get_hash();
        let block_id = block.get_id();
        let previous_block_hash = self.blockring.get_longest_chain_block_hash();

        //
        // sanity checks
        //
        if self.blocks.contains_key(&block_hash) {
            println!("ERROR: block exists in blockchain {:?}", block.get_hash());
            return;
        }

        //
        // pre-validation
        //
        // this would be a great place to put in a prevalidation check
        // once we are finished implementing Saito Classic. Goal would
        // be a fast form of lite-validation just to determine that it
        // is worth going through the more general effort of evaluating
        // this block for consensus.
        //

        //
        // save block to disk
        //
        // we have traditionally saved blocks to disk AFTER validating them
        // but this can slow down block propagation. So it may be sensible
        // to start a save earlier-on in the process so that we can relay
        // the block faster serving it off-disk instead of fetching it
        // repeatedly from memory. Exactly when to do this is left as an
        // optimization exercise.
        //

        //
        // insert block into hashmap and index
        //
        // the blockring is a BlockRing which lets us know which blocks (at which depth)
        // form part of the longest-chain. We also use the BlockRing to track information
        // on network congestion (how many block candidates exist at various depths and
        // in the future potentially the amount of work on each viable fork chain.
        //
        // we are going to transfer ownership of the block into the HashMap that stores
        // the block next, so we insert it into our BlockRing first as that will avoid
        // needing to borrow the value back for insertion into the BlockRing.
        //
        if !self
            .blockring
            .contains_block_hash_at_block_id(block_id, block_hash)
        {
            self.blockring.add_block(&block);
        }
        //
        // blocks are stored in a hashmap indexed by the block_hash. we expect all
        // all block_hashes to be unique, so simply insert blocks one-by-one on
        // arrival if they do not exist.
        //
        if !self.blocks.contains_key(&block_hash) {
            self.blocks.insert(block_hash, block);
        }

        //
        // find shared ancestor of new_block with old_chain
        //
        let mut new_chain: Vec<[u8; 32]> = Vec::new();
        let mut old_chain: Vec<[u8; 32]> = Vec::new();
        let mut shared_ancestor_found = false;
        let mut new_chain_hash = block_hash;
        let mut old_chain_hash = previous_block_hash;

        while !shared_ancestor_found {
            if self.blocks.contains_key(&new_chain_hash) {
                if self.blocks.get(&new_chain_hash).unwrap().get_lc() {
                    shared_ancestor_found = true;
                    break;
                } else {
                    if new_chain_hash == [0; 32] {
                        break;
                    }
                }
                new_chain.push(new_chain_hash);
                new_chain_hash = self
                    .blocks
                    .get(&new_chain_hash)
                    .unwrap()
                    .get_previous_block_hash();
            } else {
                break;
            }
        }

        //
        // and get existing current chain for comparison
        //
        if shared_ancestor_found {
            loop {
                if new_chain_hash == old_chain_hash {
                    break;
                }
                if self.blocks.contains_key(&old_chain_hash) {
                    old_chain.push(old_chain_hash);
                    old_chain_hash = self
                        .blocks
                        .get(&old_chain_hash)
                        .unwrap()
                        .get_previous_block_hash();
                    if old_chain_hash == [0; 32] {
                        break;
                    }
                    if new_chain_hash == old_chain_hash {
                        break;
                    }
                }
            }
        } else {

	    //
	    // we can hit this point in the code if we have a block without a parent, 
	    // in which case we want to process it without unwind/wind chain, or if
	    // we are adding our very first block, in which case we do want to process
	    // it.
	    //
	    // TODO more elegant handling of the first block and other non-longest-chain
	    // blocks.
	    //
            println!("We have added a block without a parent block... ");

        }


        //
        // at this point we should have a shared ancestor or not
        //
        // find out whether this new block is claiming to require chain-validation
        //
        let am_i_the_longest_chain = self.is_new_chain_the_longest_chain(&new_chain, &old_chain);

        //
        // if this is a potential longest-chain candidate, validate
        //

        //
        // validate
        //
        // blockchain validate "validates" the new_chain by unwinding the old
        // and winding the new, which calling validate on any new previously-
        // unvalidated blocks. When the longest-chain status of blocks changes
        // the function on_chain_reorganization is triggered in blocks and
        // with the BlockRing. We fail if the newly-preferred chain is not
        // viable.
        //
        if am_i_the_longest_chain {
            let does_new_chain_validate = self.validate(new_chain, old_chain);
            if does_new_chain_validate {

                self.add_block_success(block_hash).await;

        	//
        	// TODO
        	//
        	// mutable update is hell -- we can do this but have to have
        	// manually checked that the entry exists in order to pull
        	// this trick. we did this check before validating.
        	//
        	{
        	    self.blocks.get_mut(&block_hash).unwrap().set_lc(true);
        	}

                if !self.broadcast_channel_sender.is_none() {
                    self.broadcast_channel_sender
                        .as_ref()
                        .unwrap()
                        .send(SaitoMessage::BlockchainAddBlockSuccess { hash: block_hash })
                        .expect("error: BlockchainAddBlockSuccess message failed to send");

                    let difficulty = self.blocks.get(&block_hash).unwrap().get_difficulty();

                    self.broadcast_channel_sender
                        .as_ref()
                        .unwrap()
                        .send(SaitoMessage::BlockchainNewLongestChainBlock {
                            hash: block_hash,
                            difficulty,
                        })
                        .expect("error: BlockchainNewLongestChainBlock message failed to send");
                }
            } else {
                self.add_block_failure().await;

                if !self.broadcast_channel_sender.is_none() {
                    self.broadcast_channel_sender
                        .as_ref()
                        .unwrap()
                        .send(SaitoMessage::BlockchainAddBlockFailure { hash: block_hash })
                        .expect("error: BlockchainAddBlockFailure message failed to send");
                }
            }
        } else {
            self.add_block_failure().await;

            if !self.broadcast_channel_sender.is_none() {
                self.broadcast_channel_sender
                    .as_ref()
                    .unwrap()
                    .send(SaitoMessage::BlockchainAddBlockFailure { hash: block_hash })
                    .expect("error: BlockchainAddBlockFailure message failed to send");
            }
        }
    }

    pub async fn add_block_success(&mut self, block_hash: SaitoHash) {

	//
	// save to disk
	//
        let storage = Storage::new();
        storage.write_block_to_disk(self.blocks.get(&block_hash).unwrap());

        //
        // TODO - this is merely for testing, we do not intend
        // the routing client to process transactions in its
        // wallet.
        {
            println!(" ... start wallet: {}", create_timestamp());
            let mut wallet = self.wallet_lock.write().await;
            let block = self.blocks.get(&block_hash).unwrap();
            wallet.add_block(&block);
            println!(" ... finsh wallet: {}", create_timestamp());
        }

    }
    pub async fn add_block_failure(&mut self) {}

    pub fn get_latest_block(&self) -> Option<&Block> {
        let block_hash = self.blockring.get_longest_chain_block_hash();
        self.blocks.get(&block_hash)
    }

    pub fn get_latest_block_burnfee(&self) -> u64 {
        let block_hash = self.blockring.get_longest_chain_block_hash();
        let block = self.blocks.get(&block_hash);
        match block {
            Some(block) => {
                return block.get_burnfee();
            }
            None => {
                return 0;
            }
        }
    }

    pub fn get_latest_block_difficulty(&self) -> u64 {
        let block_hash = self.blockring.get_longest_chain_block_hash();
        let block = self.blocks.get(&block_hash);
        match block {
            Some(block) => {
                return block.get_difficulty();
            }
            None => {
                return 0;
            }
        }
    }

    pub fn get_latest_block_timestamp(&self) -> u64 {
        let block_hash = self.blockring.get_longest_chain_block_hash();
        let block = self.blocks.get(&block_hash);
        match block {
            Some(block) => {
                return block.get_timestamp();
            }
            None => {
                return 0;
            }
        }
    }

    pub fn get_latest_block_hash(&self) -> SaitoHash {
        self.blockring.get_longest_chain_block_hash()
    }

    pub fn get_latest_block_id(&self) -> u64 {
        self.blockring.get_longest_chain_block_id()
    }

    pub fn is_new_chain_the_longest_chain(
        &mut self,
        new_chain: &Vec<[u8; 32]>,
        old_chain: &Vec<[u8; 32]>,
    ) -> bool {

        if old_chain.len() > new_chain.len() {
            println!("ERROR 1");
            return false;
        }

        if self.blockring.get_longest_chain_block_id()
            >= self
                .blocks
                .get(&new_chain[new_chain.len() - 1])
                .unwrap()
                .get_id()
        {
            println!("{:?}", new_chain);
            println!("ERROR 2-1: {}", self.blockring.get_longest_chain_block_id());
            println!(
                "ERROR 2-2: {}",
                self.blocks
                    .get(&new_chain[new_chain.len() - 1])
                    .unwrap()
                    .get_id()
            );
            return false;
        }

        let mut old_bf: u64 = 0;
        let mut new_bf: u64 = 0;

        for hash in old_chain.iter() {
            old_bf += self.blocks.get(hash).unwrap().get_burnfee();
        }
        for hash in new_chain.iter() {
            new_bf += self.blocks.get(hash).unwrap().get_burnfee();
        }
        //
        // new chain must have more accumulated work AND be longer
        //
        old_chain.len() < new_chain.len() && old_bf <= new_bf
    }

    pub fn validate(&mut self, new_chain: Vec<[u8; 32]>, old_chain: Vec<[u8; 32]>) -> bool {
        if !old_chain.is_empty() {
            self.unwind_chain(&new_chain, &old_chain, old_chain.len() - 1, false)
        } else if !new_chain.is_empty() {
            self.wind_chain(&new_chain, &old_chain, 0, false)
        } else {
            true
        }
    }

    pub fn wind_chain(
        &mut self,
        new_chain: &Vec<[u8; 32]>,
        old_chain: &Vec<[u8; 32]>,
        current_wind_index: usize,
        wind_failure: bool,
    ) -> bool {
        //
        // if we are winding a non-existent chain with a wind_failure it
        // means our wind attempt failed and we should move directly into
        // add_block_failure() by returning false.
        //
        if wind_failure == true && new_chain.len() == 0 {
            return false;
        }

        {
            // yes, there is a warning here, but we need the mutable borrow to set the
            // tx.hash_for_signature information inside AFAICT
            let block = self.blocks.get_mut(&new_chain[current_wind_index]).unwrap();

            //
            // perform the pre-validation calculations needed to validate the block
            // below, in-parallel with read-only access.
            //
            block.pre_validation_calculations();
        }

        let block = self.blocks.get(&new_chain[current_wind_index]).unwrap();
        let does_block_validate = block.validate(&self, &self.utxoset);

        if does_block_validate {
            block.on_chain_reorganization(&mut self.utxoset, true);
            self.blockring
                .on_chain_reorganization(block.get_id(), block.get_hash(), true);

            if current_wind_index == (new_chain.len() - 1) {
                if wind_failure {
                    return false;
                }
                return true;
            }

            self.wind_chain(new_chain, old_chain, current_wind_index + 1, false)
        } else {
            //
            // tough luck, go back to the old chain
            //
            if current_wind_index == 0 {
                //
                // this is the first block we have tried to add
                // and so we can just roll out the older chain
                // again as it is known good.
                //
                // note that old and new hashes are swapped
                // and the old chain is set as null because
                // we won't move back to it. we also set the
                // resetting_flag to 1 so we know to fork
                // into addBlockToBlockchainFailure
                //
                // true -> force -> we had issues, is failure
                //
                self.wind_chain(old_chain, new_chain, 0, true)
            } else {
                let mut chain_to_unwind: Vec<[u8; 32]> = vec![];
                for hash in new_chain.iter().skip(current_wind_index) {
                    chain_to_unwind.push(*hash);
                }
                self.unwind_chain(old_chain, &chain_to_unwind, chain_to_unwind.len() - 1, true)
            }
        }
    }

    pub fn unwind_chain(
        &mut self,
        new_chain: &Vec<[u8; 32]>,
        old_chain: &Vec<[u8; 32]>,
        current_unwind_index: usize,
        wind_failure: bool,
    ) -> bool {
        let block = &self.blocks[&old_chain[current_unwind_index]];

        block.on_chain_reorganization(&mut self.utxoset, false);
        self.blockring
            .on_chain_reorganization(block.get_id(), block.get_hash(), false);

        if current_unwind_index == 0 {
            //
            // start winding new chain
            //
            self.wind_chain(new_chain, old_chain, 0, wind_failure)
        } else {
            //
            // continue unwinding
            //
            self.unwind_chain(new_chain, old_chain, current_unwind_index - 1, wind_failure)
        }
    }
}

// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
pub async fn run(
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let (_blockchain_channel_sender, mut blockchain_channel_receiver): (
        tokio::sync::mpsc::Sender<SaitoMessage>,
        tokio::sync::mpsc::Receiver<SaitoMessage>,
    ) = mpsc::channel(4);

    //
    // blockchain takes global broadcast channel
    //
    {
        let mut blockchain = blockchain_lock.write().await;
        blockchain.set_broadcast_channel_sender(broadcast_channel_sender.clone());
    }

    //
    // local broadcasting loop
    //
    //let test_sender = blockchain_channel_sender.clone();
    //tokio::spawn(async move {
    //    loop {
    //test_sender
    //    .send(MempoolMessage::TryBundleBlock)
    //    .await
    //    .expect("error: GenerateBlock message failed to send");
    //        sleep(Duration::from_millis(1000));
    //    }
    //});

    //
    // receive broadcast messages
    //
    loop {
        tokio::select! {

                //
                // local broadcast messages
                //
                    Some(message) = blockchain_channel_receiver.recv() => {
                        match message {
                            _ => {},
                        }
                    }

                //
                // global broadcast messages
                //
                    Ok(message) = broadcast_channel_receiver.recv() => {
                        match message {
                            SaitoMessage::MempoolNewBlock { hash: _hash } => {
                                println!("Blockchain aware of new block in mempool! -- we might use for this congestion tracking");
                            },

        //
        // TODO - delete - keeping as quick reference for multiple ways
        // to broadcast messages.
        //
        //                    SaitoMessage::TestMessage => {
        //             		println!("Blockchain RECEIVED TEST MESSAGE!");
        //			let blockchain = blockchain_lock.read().await;
        //
        //			broadcast_channel_sender.send(SaitoMessage::TestMessage2).unwrap();
        //
        //		        if !blockchain.broadcast_channel_sender.is_none() {
        //			    println!("blockchain broadcast channel sender exists!");
        //
        //	      		    blockchain.broadcast_channel_sender.as_ref().unwrap()
        //                        	.send(SaitoMessage::TestMessage3)
        //                        	.expect("error: Mempool TryBundle Block message failed to send");
        //        		}
        //                    },
                            _ => {},
                        }
                    }
                }
    }
}

#[cfg(test)]

mod tests {
    use crate::test_utilities::mocks::make_mock_block;
    use crate::miner::Miner;
    use super::*;

    #[tokio::test]
    async fn add_block_test_1() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mut blockchain = Blockchain::new(wallet_lock.clone());
        let mock_block_1 = make_mock_block(0, 10, [0; 32], 1);
        let mock_block_2 = make_mock_block(
            mock_block_1.get_timestamp(),
            mock_block_1.get_burnfee(),
            mock_block_1.get_hash(),
            mock_block_1.get_id() + 1,
        );
        // TODO: Add a test here that tries to insert a bad first block
        // I don't think we are doing any validation of the first block yet,
        // it should probably have a particular hash and block id = 1

        // Add our first block
        blockchain.add_block(mock_block_1.clone()).await;
        assert_eq!(mock_block_1.get_id(), blockchain.get_latest_block_id());
        assert_eq!(mock_block_1.get_hash(), blockchain.get_latest_block_hash());

        // Add our second block
        blockchain.add_block(mock_block_2.clone()).await;
        assert_eq!(mock_block_2.get_id(), blockchain.get_latest_block_id());
        assert_eq!(mock_block_2.get_hash(), blockchain.get_latest_block_hash());

        // Try to add block with wrong block id
        let invalid_block_wrong_block_id = make_mock_block(
            mock_block_1.get_timestamp(),
            mock_block_1.get_burnfee(),
            mock_block_1.get_hash(),
            mock_block_1.get_id() + 2,
        );
        blockchain
            .add_block(invalid_block_wrong_block_id.clone())
            .await;
        assert_eq!(mock_block_2.get_id(), blockchain.get_latest_block_id());
        assert_eq!(mock_block_2.get_hash(), blockchain.get_latest_block_hash());

        // Try to add block with wrong burnfee
        let invalid_block_wrong_burnfee = make_mock_block(
            mock_block_1.get_timestamp(),
            mock_block_1.get_burnfee() - 1,
            mock_block_1.get_hash(),
            mock_block_1.get_id() + 1,
        );
        blockchain
            .add_block(invalid_block_wrong_burnfee.clone())
            .await;
        assert_eq!(mock_block_2.get_id(), blockchain.get_latest_block_id());
        assert_eq!(mock_block_2.get_hash(), blockchain.get_latest_block_hash());

        // Try to add block with wrong previous hash
        let invalid_block_wrong_prev_hash = make_mock_block(
            mock_block_1.get_timestamp(),
            mock_block_1.get_burnfee(),
            [0; 32],
            mock_block_1.get_id() + 1,
        );
        blockchain
            .add_block(invalid_block_wrong_prev_hash.clone())
            .await;
        assert_eq!(mock_block_2.get_id(), blockchain.get_latest_block_id());
        assert_eq!(mock_block_2.get_hash(), blockchain.get_latest_block_hash());
    }

    #[tokio::test]
    async fn add_fork_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mut blockchain = Blockchain::new(wallet_lock.clone());
        let miner = Miner::new();

        let mock_block_1 = make_mock_block(0, 10, [0; 32], 1);
        blockchain.add_block(mock_block_1.clone()).await;

        let mut prev_block = mock_block_1.clone();

        for _n in 0..5 {
            let next_block = make_mock_block(
                prev_block.get_timestamp(),
                prev_block.get_burnfee(),
                prev_block.get_hash(),
                prev_block.get_id() + 1,
            );

            blockchain.add_block(next_block.clone()).await;

            assert_eq!(next_block.get_id(), blockchain.get_latest_block_id());
            assert_eq!(next_block.get_hash(), blockchain.get_latest_block_hash());
            prev_block = next_block;
        }

        assert_eq!(prev_block.get_id(), blockchain.get_latest_block_id());
        assert_eq!(prev_block.get_hash(), blockchain.get_latest_block_hash());
        // make a fork
        let longest_chain_block_id = prev_block.get_id();
        let longest_chain_block_hash = prev_block.get_hash();
        prev_block = mock_block_1.clone();
        // extend the fork to match the height of LC, the latest block id/hash shouldn't change yet...
        for _n in 0..5 {

            let next_block = make_mock_block(
                prev_block.get_timestamp(),
                prev_block.get_burnfee(),
                prev_block.get_hash(),
                prev_block.get_id() + 1,
            );

	    let golden_ticket_transaction = miner.mine_on_block_until_golden_ticket_found(prev_block);
	    next_block.add_transaction(golden_ticket_transaction);

            blockchain.add_block(next_block.clone()).await;

            assert_eq!(longest_chain_block_id, blockchain.get_latest_block_id());
            assert_eq!(longest_chain_block_hash, blockchain.get_latest_block_hash());
            prev_block = next_block;
        }
        // adding another block should now affect the LC
        let next_block = make_mock_block(
            prev_block.get_timestamp(),
            prev_block.get_burnfee(),
            prev_block.get_hash(),
            prev_block.get_id() + 1,
        );

        let golden_ticket_transaction = miner.mine_on_block_until_golden_ticket_found(prev_block);
        next_block.add_transaction(golden_ticket_transaction);

        blockchain.add_block(next_block.clone()).await;
        // TODO: These tests are failing
        assert_eq!(next_block.get_id(), blockchain.get_latest_block_id());
        assert_eq!(next_block.get_hash(), blockchain.get_latest_block_hash());
    }
}
