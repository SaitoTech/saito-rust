use crate::block::Block;
use crate::crypto::{SaitoHash};

const EPOCH_LENGTH: u64 = 21000;
const RING_BUFFER_LENGTH: u64 = 2 * EPOCH_LENGTH;



//
// This is an index with shorthand information on the block_ids and hashes of the blocks
// in the longest-chain.
//
// The BlockRing is a fixed size Vector which can be made contiguous and theoretically 
// made available for fast-access through a slice with the same lifetime as the vector
// itself.
//
#[derive(Debug)]
pub struct RingItem {
    lc_pos: usize,	// which idx in the vectors below points to the longest-chain block
    block_hashes: Vec<SaitoHash>,
    block_ids: Vec<u64>,
}

impl RingItem {

    pub fn new() -> Self {
        Self {
            lc_pos: usize::MAX,
            block_hashes: vec![],
            block_ids: vec![],
        }
    }

    pub fn contains_block_hash(&mut self, hash : SaitoHash) -> bool {
        if self.block_hashes.iter().any(|&i| i == hash) { return true; }
	return false;
    }

    pub fn add_block(&mut self, block_id : u64, hash : SaitoHash) {
	self.block_hashes.push(hash);
	self.block_ids.push(block_id);
    }

    pub fn on_chain_reorganization(&mut self, hash : SaitoHash, lc : bool) {

	if !lc {
	    self.lc_pos = usize::MAX;
	} else {

	    //
	    // update new longest-chain
	    //
	    for i in 0..self.block_hashes.len() {
	        if self.block_hashes[i] == hash { 
	            self.lc_pos = i;
	        }
	    }

	    //
	    // remove any old indices
	    //
	    let current_block_id = self.block_ids[self.lc_pos];

	    let mut new_block_hashes: Vec<SaitoHash> = vec![];
	    let mut new_block_ids: Vec<u64> = vec![];

	    for i in 0..self.block_ids.len() {
	        if self.block_ids[i] < current_block_id { 
	            self.lc_pos = i;
	        } else {
		    new_block_hashes.push(self.block_hashes[i]);
		    new_block_ids.push(self.block_ids[i]);
		}
	    }

	    self.block_hashes = new_block_hashes;
	    self.block_ids = new_block_ids;

	}
    }
}


//
// TODO - we can optimize this.  
//
// The Blockchain Index is a self-contained data structure
//
#[derive(Debug)]
pub struct BlockRing {

    //
    // each ring_item is a point on our blockchain
    //
    // include Slice-VecDeque and have a slice that points to
    // contiguous entries for rapid lookups, inserts and updates?
    //
    block_ring: Vec<RingItem>,
    /// a ring of blocks, index is not the block_id.
    block_ring_lc_pos: usize,

}


impl BlockRing {

    /// Create new `BlockRing`
    pub fn new() -> Self {

	//
	// initialize the block-ring
	//
        let mut init_block_ring: Vec<RingItem> = vec![];
        for _i in 0..EPOCH_LENGTH { init_block_ring.push(RingItem::new()); }

        BlockRing {
            block_ring: init_block_ring,
            block_ring_lc_pos: usize::MAX,
        }
    }

    pub fn contains_block_hash_at_block_id(&mut self, block_id : u64 , block_hash : SaitoHash) -> bool {
	let insert_pos = block_id % RING_BUFFER_LENGTH;
	return self.block_ring[(insert_pos as usize)].contains_block_hash(block_hash);	
    }

    pub fn add_block(&mut self, block: &Block) {
	let insert_pos = block.get_id() % RING_BUFFER_LENGTH;
	self.block_ring[(insert_pos as usize)].add_block(block.get_id(), block.get_hash());	
    }

    pub fn on_chain_reorganization(&mut self, block_id: u64 , hash: SaitoHash, lc : bool) {
	let insert_pos = block_id % RING_BUFFER_LENGTH;
	self.block_ring[(insert_pos as usize)].on_chain_reorganization(hash, lc);
        if lc { self.block_ring_lc_pos = insert_pos as usize; }
    }

    pub fn get_longest_chain_block_hash_by_block_id(&self, id: u64) -> SaitoHash {
	let insert_pos = id % RING_BUFFER_LENGTH;
	let lc_pos = self.block_ring[(insert_pos as usize)].lc_pos;
	if lc_pos != usize::MAX {
	    return self.block_ring[(insert_pos as usize)].block_hashes[lc_pos];
	} else {
	    return [0; 32];
	}
    }

    pub fn get_longest_chain_block_hash(&self) -> SaitoHash {
	if self.block_ring[self.block_ring_lc_pos].lc_pos == usize::MAX { return [0; 32]; }
	let lc_pos = self.block_ring[self.block_ring_lc_pos].lc_pos;
	if lc_pos != usize::MAX {
	    return self.block_ring[self.block_ring_lc_pos].block_hashes[lc_pos];
	} else {
	    return [0; 32];
	}
    }

    pub fn get_longest_chain_block_id(&self) -> u64 {
	if self.block_ring[self.block_ring_lc_pos].lc_pos == usize::MAX { return 0; }
	let lc_pos = self.block_ring[self.block_ring_lc_pos].lc_pos;
	if lc_pos != usize::MAX {
	    return self.block_ring[self.block_ring_lc_pos].block_ids[lc_pos];
	} else {
	    return 0;
	}
    }


}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn longest_chain_queue_test() {
        assert_eq!(1, 1);
    }

}





