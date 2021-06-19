use crate::{
    slip::Slip,
    blockchain::Blockchain,
    block::Block,
    crypto::{hash, verify, verify_bytes_message, Sha256Hash, Signature},
    keypair::Keypair,
    transaction::Transaction,
    types::SaitoMessage,
};
use std::collections::HashMap;
use crate::time::create_timestamp;
use std::{
    time::Duration,
};
use tokio::time;


use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use std::sync::{Arc};
use std::thread;

// auto TX generation
use rand::Rng;

lazy_static! {
    // use RwLock for timer 
    pub static ref BUNDLER_ACTIVE: RwLock<u64> = RwLock::new(0);
}


pub struct Mempool {

    transactions: Vec<Transaction>,
    blocks: HashMap<Sha256Hash, Block>,
    broadcast_channel_sender:   Option<broadcast::Sender<SaitoMessage>>,

}


impl Mempool {

    pub fn new() -> Self {
        Mempool {
            blocks: HashMap::new(),
            transactions: vec![],
	    broadcast_channel_sender: None,
        }
     }

    pub fn add_block(&mut self, block: Block) {
	if !self.blocks.contains_key(&block.get_hash()) {
//println!("Inserting block: {:?}", block);
	    self.blocks.insert(block.get_hash(), block);
	} else {
	    println!("Mempool already contains block: {:?}", block.get_hash());
	}
    }

    pub fn get_block(&mut self, hash: [u8;32]) -> Option<Block> {

        let block = self.blocks.remove(&hash);
	match block {
	    Some(block) => {
		return Some(block);
	    }
	    None => {
		return None;
	    }
	}

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
    pub async fn start_bundling(&self, mempool_lock: Arc<RwLock<Mempool>>, blockchain_lock: Arc<RwLock<Blockchain>>) {

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
			let blockchain_lock_clone = blockchain_lock.clone();
			mempool.stop_bundling().await;
			mempool.bundle_block(blockchain_lock_clone).await;
		    }
		    {
			// update external reference
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

    pub async fn bundle_block(&mut self, blockchain_lock: Arc<RwLock<Blockchain>>) {

	//
	// create the block and add it to our blocks vector
	//
	let blockchain = blockchain_lock.read().await;
	let previous_block_hash = blockchain.get_latest_block_hash();
	let previous_block_id = blockchain.get_latest_block_id();

	let mut block = self.generate_mempool_block(previous_block_id, previous_block_hash);

	block.set_hash();

	let block_hash = block.get_hash();

	self.add_block(block);

	if !self.broadcast_channel_sender.is_none() {
	   self.broadcast_channel_sender.as_ref().unwrap() 
			.send(SaitoMessage::Block { payload: block_hash })
                        .expect("error: Mempool TryBundle Block message failed to send");
	}
    }



    pub fn generate_mempool_block(&self, previous_block_id: u64, previous_block_hash: [u8;32]) -> Block {

        let mut block = Block::new();
	let message_size = 10240;
	block.set_id(previous_block_id);
	block.set_timestamp(create_timestamp());
	block.set_previous_block_hash(previous_block_hash);

	for i in 0..100000 {

println!("Creating Transaction {:?}", i);

            let mut transaction = Transaction::new();
	    transaction.set_message( (0..message_size).map(|_| { rand::random::<u8>() }).collect() );

	    let mut input1 = Slip::new();
	    input1.set_publickey(Keypair::new().publickey().serialize());
	    input1.set_amount(1000000);
	    input1.set_uuid([1;32]);
	    transaction.add_input(input1);

	    let mut output1 = Slip::new();
	    output1.set_publickey(Keypair::new().publickey().serialize());
	    output1.set_amount(1000000);
	    output1.set_uuid([1;32]);
	    transaction.add_output(output1);

	    let mut output2 = Slip::new();
	    output2.set_publickey(Keypair::new().publickey().serialize());
	    output2.set_amount(1000000);
	    output2.set_uuid([1;32]);
	    transaction.add_output(output2);

let keypair = Keypair::new();
transaction.sign_transaction(&keypair);

let msg2 = transaction.get_signature_source();
let sig2 = transaction.get_signature();
let pub2 = keypair.publickey().serialize();

if !verify(&msg2, sig2, pub2) {
    println!("message verifies not");
} else {
    println!("message verifies");

}

	    block.add_transaction(transaction);

	}

	block.set_hash();

	return block;

    }


}


