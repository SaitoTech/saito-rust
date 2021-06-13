use crate::{
  block::Block,
  types::SaitoMessage,
  forktree::ForkTree,
};

use tokio::sync::{broadcast};


#[derive(Debug)]
pub struct Blockchain {

    blocks: ForkTree,
    broadcast_channel_sender:   Option<broadcast::Sender<SaitoMessage>>,

    lc_hash: [u8; 32],
    previous_lc_hash: [u8; 32],

}


impl Blockchain {

    pub fn new() -> Self {
        Blockchain {
            broadcast_channel_sender: None,
	    blocks: ForkTree::new(),
    	    lc_hash: [0;32],
    	    previous_lc_hash: [0;32],
        }
    }


    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }


    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {

	//
	// Sanity Checks
	//
println!("Block Hash: {:?}", block.get_hash());

	//
	// Add to Blockchain
	//
	// We assume here that this block will form the end of the longest
	// chain. If this is not the case we will reset lc_hash to its 
	// original value before this function ends.
	//
	self.previous_lc_hash = self.lc_hash;
	self.lc_hash = block.get_hash();
        self.blocks.insert(block.get_hash(), block);


	//
	// Find Shared Ancestor
	//
        let mut new_chain: Vec<[u8;32]> = Vec::new();
        let mut old_chain: Vec<[u8;32]> = Vec::new();
	let mut shared_ancestor_found = 0;

	while shared_ancestor_found == 0 {
	    shared_ancestor_found = 1;
	}

println!("Shared Ancestor Found!");



	//
	//
	//



	//
	// Resume Bundling
	//
        if !self.broadcast_channel_sender.is_none() {
           self.broadcast_channel_sender.as_ref().unwrap()
                        .send(SaitoMessage::StartBundling)
                        .expect("error: Mempool TryBundle Block message failed to send");
        }
    }

}


