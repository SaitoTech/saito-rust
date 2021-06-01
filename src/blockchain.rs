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

}

impl Blockchain {

    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            blocks: vec![],
            shashmap: Shashmap::new(),
        }
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, block: Block) {

        self.blocks.push(block);
	println!("Added block. Blocks now: {}", self.blocks.len());

    }

    // /// Return latest `Block` in blockchain
    // pub fn get_latest_block(&mut self) -> &Block {
    // 
    // }
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
    //     let block = Block::new(Keypair::new().publickey().clone(), [0; 32]);
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
    //     let mut block = Block::new(keypair.publickey().clone(), [0; 32]);
    //     let mut transaction = Transaction::new(TransactionType::Normal);
    //     let to_slip = OutputSlip::new(keypair.publickey().clone(), SlipBroadcastType::Normal, 0);
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

