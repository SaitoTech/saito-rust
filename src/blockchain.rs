use crate::block::Block;
use crate::shashmap::Shashmap;

/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the material that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug, Clone)]
pub struct Blockchain {

    /// Vector of blocks
    blocks: Vec<Block>,

    /// Hashmap of slips used by the network
    shashmap: Shashmap,

    // longest-chain position
    pos: usize,
    last_pos: usize,

}

pub struct BlockHeader {

    // longest-chain position
    pos: usize,
    block_id: u64,
    burnfee: u64,

}
impl BlockHeader {
    pub fn new(pos: usize, block: &Block) -> Self {
        BlockHeader {
	    pos: pos,
	    block_id: block.block_id(),
	    burnfee: block.burnfee(),
	}
    }
}


impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {

            blocks: vec![],
            shashmap: Shashmap::new(),

	    pos: 0,
	    last_pos: 0,

        }
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, mut block: Block) {

	//
	// TODO
	//
	// data structure does not permit blocks to build
	// off each other as mempool does not have a way
	// to know the previous_block_hash at this point
	// so we are manually setting this block as the
	// predecessor to the previous block
	//
	if (self.blocks.len() > 0) {
	    let previous_blocks_length = self.blocks.len();
	    block.set_previous_block_hash(self.blocks[previous_blocks_length-1].hash());
	    println!("previous block hash of new block now set to: {:?}", self.blocks[previous_blocks_length-1].hash());
	}

println!("Adding Block with Hash: {:?}", block.hash());

	// initial sanity checks


	// prepare for failure
	self.last_pos = self.pos;


	// cheat and just add to end	
        self.blocks.push(block);
	self.pos = self.blocks.len()-1;


	// create two chains
	let mut new_pos = self.pos;
	let mut old_pos = self.last_pos;
	let mut shared_ancestor_pos = 0;

        let mut new_chain: Vec<BlockHeader> = Vec::new();
        let mut old_chain: Vec<BlockHeader> = Vec::new();

	loop {

println!("loop...");
println!("new pos: {}", new_pos);
println!("old pos: {}", old_pos);

	    if new_pos > old_pos {

		let target_hash = self.blocks[new_pos].previous_block_hash();

		let mut found_parent_block = false;
		while !found_parent_block {
		    new_pos -= 1;
		    if self.blocks[new_pos].hash() == target_hash || new_pos == 0 {
		        found_parent_block = true;
		    }
		}

		let mut newbh = BlockHeader::new(new_pos, &self.blocks[new_pos]);
		newbh.pos = new_pos;
		newbh.burnfee = 1;
	        new_chain.push(newbh);

	    } else if new_pos == old_pos || new_pos == 0 || old_pos == 0 {

		shared_ancestor_pos = new_pos;

		break;

	    } else {

		let target_hash = self.blocks[old_pos].previous_block_hash();

		let mut found_parent_block = false;
		while !found_parent_block {
		    old_pos -= 1;
		    if self.blocks[old_pos].hash() == target_hash || old_pos == 0 {
		        found_parent_block = true;
		    }
		}

		let mut oldbh = BlockHeader::new(old_pos, &self.blocks[old_pos]);
		oldbh.pos = old_pos;
		oldbh.burnfee = 1;
	        old_chain.push(oldbh);

	    }
	}

println!("New Block at position: {}", self.pos);
println!("New Chain length: {}", new_chain.len());
println!("Old Chain length: {}", old_chain.len());
println!("Shared Ancestor at position: {}", shared_ancestor_pos);


	// at this point we should have a shared ancestor

    }


	    // 

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
    //     let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipBroadcastType::Normal, 0);
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
    //     //assert_eq!(blockchain.shashmap.slip_block_id(&slip), Some(&-1));
    // }
}
