
use crate::{
  block::Block,
  crypto::Sha256Hash,
  time::create_timestamp,
  types::SaitoMessage,
};
use ahash::AHashMap;
use std::collections::HashMap;
use tokio::sync::{broadcast};

// saving to disk
//use std::fs::{self, File};
//use std::io::prelude::*;
//use std::io::LineWriter;
use std::fs::File;
use std::io::{BufWriter, Write};
use rand::Rng;


#[derive(Debug)]
pub struct BlockIndex {
    id: u64,
    hashes: Vec<[u8;32]>,
}
impl BlockIndex {
    pub fn new() -> Self {
        BlockIndex {
	    id: 0,
    	    hashes: vec![],
        }
    }
}


#[derive(Debug)]
pub struct Blockchain {

    utxoset: AHashMap<[u8;47], u64>,
    blocks: HashMap<Sha256Hash, Block>,
    index: HashMap<u64, BlockIndex>,
    broadcast_channel_sender:   Option<broadcast::Sender<SaitoMessage>>,

    lc_hash: [u8; 32],
    previous_lc_hash: [u8; 32],

}

impl Blockchain {

    pub fn new() -> Self {
        Blockchain {
            broadcast_channel_sender: None,
    	    lc_hash: [0;32],
    	    previous_lc_hash: [0;32],
            utxoset: AHashMap::new(),
            blocks: HashMap::new(),
            index: HashMap::new(),
        }
    }


    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }


    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {

println!(" ... add_block start: {:?}", create_timestamp());

	//
	// start by extracting some variables that we will need
	// to use repeatedly in order to avoid the need to access
	// them from within our hashmap / index after we have 
	// transferred ownership of the block to that structure.
	//
	let block_hash = block.get_hash();
	let block_id = block.get_id();


	//
	// sanity checks
	//
	//println!("Block Hash: {:?}", block.get_hash());


	//
	// prepare to Add Block to Blockchain
	//
	// assume that this block will form the longest-chain and update
	// our lc_hash and previous_lc_hash variables to reflect this. If 
	// validation or update fails, we will revert to the previous tip
	// of the longest-chain.
	//
	self.previous_lc_hash = self.lc_hash;
	self.lc_hash = block.get_hash();

	//
        // we explicitly save the block to disk here, as serializing
	// it after insertion into the hashmap is impossible. it would
	// be better to only save after minimal validation, otherwise
	// we may have to remove and then re-insert or... what else?
	//
        let file = File::create("test_saving_block_to_disk.txt")
			.expect("Unable to create file");


	//let mut transaction = vec![];
	//let message_size = 1024000;
	//transaction = (0..message_size).map(|_| { rand::random::<u8>() }).collect();
        let mut file = BufWriter::new(file);
//        for i in 0..100 {
//	    let mut transaction = [1;1024000];
//            file.write_all(&transaction)
//			.expect("Unable to write data");
//	        }
//        file.write_all(&transaction)

        file.write_all(&bincode::serialize(&block).unwrap())
			.expect("Unable to write data");
        file.flush()
			.expect("Problems flushing file");

println!(" ... indexing start:  {:?}", create_timestamp());

	//
	// insert block into hashmap and index
	//
	if !self.blocks.contains_key(&block_hash) {
	    self.blocks.insert(block_hash, block);
	}
	if self.index.contains_key(&block_id) {
	    if self.index.get(&block_id).unwrap().hashes.contains(&block_hash) {
	    } else {
		//
	        // mutable update is hell -- we can do this but have to have 
	        // manually checked that the entry exists in order to pull 
	        // this trick.
		//
	        self.index.get_mut(&block_id).unwrap().hashes.push(block_hash);
	    }
        } else {
	    //
	    // create a new BlockIndex which will store the hash along with
	    // the block_id. this permits O(1) lookup of all of the blocks
	    // at any block depth.
	    //
	    let mut new_block_index = BlockIndex::new();
	    new_block_index.id = block_id;
	    new_block_index.hashes.push(block_hash);
	    self.index.insert(block_id, new_block_index);
	}

println!(" ... ancestor search: {:?}", create_timestamp());
	//
	// now we find the shared ancestor
	//
        let mut new_chain: Vec<[u8;32]> = Vec::new();
        let mut old_chain: Vec<[u8;32]> = Vec::new();

	let mut shared_ancestor_found = false;
	let mut shared_ancestor_hash = [0;32];
	let mut new_chain_hash = block_hash;
	let mut old_chain_hash = self.previous_lc_hash;


	//
	// track new chain down to last lc=true ancestor
	//
	while !shared_ancestor_found {

	    if self.blocks.contains_key(&new_chain_hash) {
		new_chain.push(new_chain_hash);
		new_chain_hash = self.blocks.get(&new_chain_hash).unwrap().get_previous_block_hash();
		if new_chain_hash == [0;32] {
		    break;
		}
		if self.blocks.get(&new_chain_hash).unwrap().get_lc() {
		    shared_ancestor_found = true;
		    shared_ancestor_hash = self.blocks.get(&new_chain_hash).unwrap().get_hash();
		}
	    } else {
		break;
	    }

	}

println!(" ... chain generates: {:?}", create_timestamp());

	//
	// track old chain down to shared ancestor
	//
	if shared_ancestor_found {
	    loop {
	    
		if (new_chain_hash == old_chain_hash) {
		    break;
		}
	        if self.blocks.contains_key(&old_chain_hash) {
		    old_chain.push(old_chain_hash);
		    old_chain_hash = self.blocks.get(&old_chain_hash).unwrap().get_previous_block_hash();
		    if old_chain_hash == [0;32] {
		        break;
		    }
		    if (new_chain_hash == old_chain_hash) {
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
        let am_i_the_longest_chain = self.is_new_chain_the_longest_chain(&new_chain, &old_chain);


	//
	// validate
        //
        if am_i_the_longest_chain {

println!(" ... start validate:  {:?}", create_timestamp());
            let does_new_chain_validate = self.validate(new_chain, old_chain);
println!(" ... finish validate: {:?}", create_timestamp());
            if does_new_chain_validate {
println!("SUCCESS");
                self.add_block_success();
            } else {
println!("FAILURE 1");
                self.add_block_failure();
            }
println!("does the new chain validate: {}", does_new_chain_validate);
        } else {
println!("FAILURE 2");
            self.add_block_failure();
        }
   

	//
	// Resume Bundling
	//
        if !self.broadcast_channel_sender.is_none() {
           self.broadcast_channel_sender.as_ref().unwrap()
                        .send(SaitoMessage::StartBundling)
                        .expect("error: Mempool TryBundle Block message failed to send");
        }
    }


    pub fn add_block_success(&mut self) {

	//
	// block is longest-chain
	//
	// mutable update is hell -- we can do this but have to have 
	// manually checked that the entry exists in order to pull 
	// this trick. we did this check before validating.
	//
	self.blocks.get_mut(&self.lc_hash).unwrap().set_lc(true);

    }
    pub fn add_block_failure(&mut self) {

        // revert
        self.lc_hash = self.previous_lc_hash;

    }


    //
    // return latest block, or disconnected head, inserting
    // if needed. TODO - some design questions around return
    // of read-only references to blocks if they do not 
    // necessarily exist.
    //
    pub fn get_latest_block_hash(&self) -> [u8;32] {
        if self.blocks.contains_key(&self.lc_hash) {
            return self.blocks.get(&self.lc_hash).unwrap().get_hash();
        } else {
	    return [0;32];
	}
    }
    pub fn get_latest_block_id(&self) -> u64 {
        if self.blocks.contains_key(&self.lc_hash) {
            return self.blocks.get(&self.lc_hash).unwrap().get_id();
        } else {
	    return 0;
	}
    }


    //
    // TODO - return 1 for new_chain, 2 for old_chain
    //
    pub fn is_new_chain_the_longest_chain(&mut self, new_chain: &Vec<[u8;32]>, old_chain: &Vec<[u8;32]>) -> bool {
        return true;
    }

    pub fn validate(&mut self, new_chain: Vec<[u8;32]>, old_chain: Vec<[u8;32]>) -> bool {
        if old_chain.len() > 0 {
            return self.unwind_chain(&new_chain, &old_chain, old_chain.len()-1);
        } else if new_chain.len() > 0 {
            return self.wind_chain(&new_chain, &old_chain, 0, false);
        }
        return true;
    }


    pub fn wind_chain(&mut self, new_chain: &Vec<[u8;32]>, old_chain: &Vec<[u8;32]>, current_wind_index: usize, wind_failure: bool) -> bool {

        // validate the block
        let block = &self.blocks[&new_chain[current_wind_index]];
        let does_block_validate = block.validate(&self.utxoset);

        if does_block_validate {
println!(" ... on_chain_reorg:  {:?}", create_timestamp());
            block.on_chain_reorganization(&mut self.utxoset, true);
            if current_wind_index == (new_chain.len()-1) {
                return true;
            }
            return self.wind_chain(new_chain, old_chain, (current_wind_index+1), false);
        } else {
            return false;
        }

        return false;
    }

    // TODO - not properly implemented
    pub fn unwind_chain(&mut self, new_chain: &Vec<[u8;32]>, old_chain: &Vec<[u8;32]>, current_unwind_index: usize) -> bool {
        //block.on_chain_reorganization(&mut self.utxoset, 0);
        println!("UNWIND CHAIN {}", current_unwind_index);
        return true;
    }


}


