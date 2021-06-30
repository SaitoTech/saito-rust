use crate::block::Block;
use crate::blockring::BlockRing;
use crate::crypto::{SaitoHash, SaitoUTXOSetKey};
use crate::storage::Storage;
use crate::time::create_timestamp;

use ahash::AHashMap;

#[derive(Debug)]
pub struct Blockchain {
    pub utxoset: AHashMap<SaitoUTXOSetKey, u64>,
    pub blockring: BlockRing,
    pub blocks: AHashMap<SaitoHash, Block>,
}

impl Blockchain {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Blockchain {
            utxoset: AHashMap::new(),
            blockring: BlockRing::new(),
            blocks: AHashMap::new(),
        }
    }

    pub fn add_block(&mut self, block: Block) {
        println!(" ... add_block start: {:?}", create_timestamp());
        println!(" ... txs in block: {:?}", block.transactions.len());

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
        //println!("Block Hash: {:?}", block.get_hash());

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
                new_chain.push(new_chain_hash);
                new_chain_hash = self
                    .blocks
                    .get(&new_chain_hash)
                    .unwrap()
                    .get_previous_block_hash();
                if new_chain_hash == [0; 32] {
                    break;
                }
                if self.blocks.get(&new_chain_hash).unwrap().get_lc() {
                    shared_ancestor_found = true;
                }
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
        }

        //println!("new chain: {:?}", new_chain);
        //println!("old chain: {:?}", old_chain);

        //
        // at this point we should have a shared ancestor
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
            println!(" ... start validate:  {:?}", create_timestamp());
            let does_new_chain_validate = self.validate(new_chain, old_chain);
            println!(" ... finish validate: {:?}", create_timestamp());
            if does_new_chain_validate {
                self.add_block_success(block_hash);
            } else {
                self.add_block_failure();
            }
        } else {
            self.add_block_failure();
        }

        println!("TOTAL INDEX OF BLOCKS");
        self.blockring.print_lc();
    }

    pub fn add_block_success(&mut self, block_hash: SaitoHash) {
        //
        // TODO -
        //
        // block is longest-chain
        //
        // mutable update is hell -- we can do this but have to have
        // manually checked that the entry exists in order to pull
        // this trick. we did this check before validating.
        //

        self.blocks.get_mut(&block_hash).unwrap().set_lc(true);
        let storage = Storage::new();
        storage.write_block_to_disk(self.blocks.get(&block_hash).unwrap());
    }
    pub fn add_block_failure(&mut self) {}

    pub fn get_latest_block(&self) -> Option<&Block> {
        let block_hash = self.blockring.get_longest_chain_block_hash();
        self.blocks.get(&block_hash)
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
            return false;
        }

        let mut old_bf = 0;
        let mut new_bf = 0;
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
        let mut does_block_validate = false;

        {
            let mut block = self.blocks.get_mut(&new_chain[current_wind_index]).unwrap();

            //
            // validate the block
            //
            block.validate_pre_calculations();
        }

        let block = self.blocks.get(&new_chain[current_wind_index]).unwrap();
        does_block_validate = block.validate(self);

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
