use crate::{
    block::Block,
    transaction::Transaction,
    types::SaitoMessage,
};
use std::{
    time::Duration,
};
use tokio::time;

use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use std::sync::{Arc};
use std::thread;

lazy_static! {
    // use RwLock for timer 
    pub static ref BUNDLER_ACTIVE: RwLock<u64> = RwLock::new(0);
}


pub struct Mempool {

    transactions: Vec<Transaction>,
    blocks: Vec<Block>,

}


impl Mempool {

    pub fn new() -> Self {
         Mempool {
             blocks: vec![],
             transactions: vec![],
         }
     }

    pub fn add_block(&mut self, block: Block) {

    }

    pub fn get_block(&mut self, hash: [u8;32]) -> Option<Block> {
	let block = Block::new();
	Some(block)
    }

    pub fn get_transaction(&mut self, hash: [u8;32]) -> Option<Transaction> {
	let transaction = Transaction::new();
	Some(transaction)
    }

    pub async fn start_bundling(&self, mempool: &Mempool) {

	let mut already_bundling = 0;
	let mut count = 10;

	//
	// write-access contained in closure to close 
	// access to it once finished, permitting 
	// stop_bundling to access it quickly if requested
	//
	{
	  let mut w = BUNDLER_ACTIVE.write().await;	
	  already_bundling = *w;
	  *w = 1;
	}

	if already_bundling > 0 {
	    println!("Mempool is already attempting to bundle blocks");
	    return;
	}

	{
	    let mut r = BUNDLER_ACTIVE.read().await;
	    already_bundling = *r;
	}

	//
        // spawn async thread to bundle
	//
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(500));
	    while already_bundling > 0 {
                interval.tick().await;
		if already_bundling > 0 {
		    count -= 1;
	            println!("Tick... {:?}", already_bundling);
                    interval.tick().await;

		    if count == 0 {
			mempool.stop_bundling(mempool);		
			//self.bundle_block();		
		    }
		    {
	                let mut r = BUNDLER_ACTIVE.read().await;
	                already_bundling = *r;
		    }
	        }
	    }
        });

    }

    pub async fn stop_bundling(&self, mempool: &Mempool) {
	let mut w = BUNDLER_ACTIVE.write().await;
	*w = 0;
	println!("We have set bundler active as inactive!");
    }

    pub fn bundle_block(&self) {

println!("Bundling Block");

    }





    pub fn processSaitoMessage(
         &mut self,
         message: SaitoMessage,
         mempool_sender_channel: &broadcast::Sender<(SaitoMessage)>,
    ) {

         match message {
             SaitoMessage::TryBundle => {
/***
		 if self.bundler_count == 0 {
		     self.bundler_count = 10;
		     mempool_sender_channel
			.send(SaitoMessage::Block { payload: [0;32] })
                        .expect("error: Mempool TryBundle Block message failed to send");
		 }
		 self.bundler_count -= 1;
***/
                 println!("This is a line printing in TryBundle 123");
	     }
             _ => (),
	}
    }

}



