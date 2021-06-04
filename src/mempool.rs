use crate::{
    block::Block,
    blockchain::Blockchain,
    burnfee::BurnFee,
    keypair::Keypair,
    time::{create_timestamp, format_timestamp},
    transaction::Transaction,
    types::SaitoMessage,
};

use tokio::sync::{broadcast, mpsc};
use std::sync::{Arc, RwLock};
use std::thread;

/// TODO -- 
pub const GENESIS_PERIOD: u64 = 21500;

/// The `Mempool` is the structure that collects blocks and transactions
/// and is control of discerning whether the node is allowed to create a block.
/// It bundles the block, the sends it to `Blockchain` to be added to the longest chain.
/// New `Block`s coming in over the network will hit the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {

    transactions: Vec<Transaction>,
    blocks: Vec<Block>,
    bundler_active: u64,
    bundler_count: u8,

}

impl Mempool {

    /// Creates new `Memppol`
    pub fn new() -> Self {
         Mempool {
             blocks: vec![],
             transactions: vec![],
	     bundler_active: 1,
	     bundler_count: 10,
         }
     }

    pub fn get_block(&mut self, hash: [u8;32]) -> Option<Block> {
	let block = Block::new();
	Some(block)
    }

    pub fn get_transaction(&mut self, hash: [u8;32]) -> Option<Transaction> {
	let transaction = Transaction::new();
	Some(transaction)
    }

    pub fn start_bundling(&mut self) {
	self.bundler_active = 1;
    }

    pub fn stop_bundling(&mut self) {
	self.bundler_active = 0;
    }

    pub fn processSaitoMessage(
         &mut self,
         message: SaitoMessage,
         mempool_sender_channel: &broadcast::Sender<(SaitoMessage)>,
    ) {

         match message {
             SaitoMessage::TryBundle => {
		 if self.bundler_count == 0 {
		     self.bundler_count = 10;
		     mempool_sender_channel
			.send(SaitoMessage::Block { payload: [0;32] })
                        .expect("error: Mempool TryBundle Block message failed to send");
		 }
		 self.bundler_count -= 1;
                 println!("This is a line printing in TryBundle: {}", self.bundler_count);
	     }
             _ => (),
	}
    }

}

#[cfg(test)]
mod tests {
    // 
    // use super::*;
    // use crate::keypair::Keypair;
    // use std::sync::{Arc, RwLock};
    //
    // #[test]
    // fn mempool_test() {
    //     assert_eq!(true, true);
    //     let keypair = Arc::new(RwLock::new(Keypair::new()));
    //     let mempool = Mempool::new(keypair);
    // 
    //     assert_eq!(mempool.work_available, 0);
    // }
    // #[test]
    // fn mempool_try_bundle_none_test() {
    //     let keypair = Arc::new(RwLock::new(Keypair::new()));
    //     let mut mempool = Mempool::new(keypair);
    // 
    //     let new_block = mempool.try_bundle(None);
    // 
    //     match new_block {
    //         Some(block) => {
    //             assert_eq!(block.id(), 0);
    //             assert_eq!(*block.previous_block_hash(), [0; 32]);
    //         }
    //         None => {}
    //     }
    // }
    // #[test]
    // fn mempool_try_bundle_some_test() {
    //     let keypair = Arc::new(RwLock::new(Keypair::new()));
    //     let mut mempool = Mempool::new(keypair);
    // 
    //     let prev_block = Block::new(Keypair::new().publickey().clone(), [0; 32]);
    //     let prev_block_index = &(prev_block.header().clone(), prev_block.hash());
    //     let new_block = mempool.try_bundle(Some(prev_block_index));
    // 
    //     match new_block {
    //         Some(_) => {}
    //         None => {
    //             assert_eq!(true, true)
    //         }
    //     }
    // }
}
