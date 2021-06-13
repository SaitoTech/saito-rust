use crate::{
    block::Block,
    transaction::Transaction,
    types::SaitoMessage,
};
use crate::time::create_timestamp;
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
    broadcast_channel_sender:   Option<broadcast::Sender<SaitoMessage>>,

}


impl Mempool {

    pub fn new() -> Self {
        Mempool {
            blocks: vec![],
            transactions: vec![],
	    broadcast_channel_sender: None,
        }
     }

    pub fn add_block(&mut self, block: Block) {

    }

    pub fn get_block(&mut self, hash: [u8;32]) -> Option<Block> {
	let mut block = Block::new();
	block.set_timestamp(create_timestamp());
	block.set_hash();
	Some(block)
    }

    pub fn get_transaction(&mut self, hash: [u8;32]) -> Option<Transaction> {
	let transaction = Transaction::new();
	Some(transaction)
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<(SaitoMessage)>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    //
    // the functions start_bundling() and stop_bundling() control whether the
    // mempool attempts to create blocks.
    //
    // a static RwLock permits multiple threads to change the bundling status of
    // the mempool while minimizing memory leaks in the event of software shutdown
    // or restart.
    //
    pub async fn start_bundling(&self, mempool_lock: Arc<RwLock<Mempool>>) {

	let mut already_bundling;
	let mut count = 5;

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
	            println!("Tick... {:?}", count);
                    interval.tick().await;

		    if count == 0 {
			let mut mempool = mempool_lock.write().await;
			mempool.stop_bundling().await;
			mempool.bundle_block();		
		    }
		    {
	                let mut r = BUNDLER_ACTIVE.read().await;
	                already_bundling = *r;
		    }
	        }
	    }
        });

    }

    pub async fn stop_bundling(&self) {
	let mut w = BUNDLER_ACTIVE.write().await;
	*w = 0;
    }

    pub fn bundle_block(&self) {
	if !self.broadcast_channel_sender.is_none() {
	   self.broadcast_channel_sender.as_ref().unwrap() 
			.send(SaitoMessage::Block { payload: [0;32] })
                        .expect("error: Mempool TryBundle Block message failed to send");
	}
    }

}


