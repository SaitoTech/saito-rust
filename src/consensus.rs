use crate::{
    blockchain::Blockchain,
    crypto::hash,
    mempool::Mempool,
    miner::{Miner},
    keypair::Keypair,
    types::SaitoMessage,
};
use std::{
    future::Future,
    sync::{Arc},
    thread::sleep,
};
    //time::Duration,
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, Interval, Duration};


/// The consensus state which exposes a run method
/// initializes Saito state
struct Consensus {
    /// Broadcasts a shutdown signal to all active components.
    _notify_shutdown: broadcast::Sender<()>,
    /// Used as part of the graceful shutdown process to wait for client
    /// connections to complete processing.
    _shutdown_complete_rx: mpsc::Receiver<()>,
    _shutdown_complete_tx: mpsc::Sender<()>,
}

/// Run the Saito consensus runtime
pub async fn run(shutdown: impl Future) -> crate::Result<()> {
    // When the provided `shutdown` future completes, we must send a shutdown
    // message to all active connections. We use a broadcast channel for this
    // purpose. The call below ignores the receiver of the broadcast pair, and when
    // a receiver is needed, the subscribe() method on the sender is used to create
    // one.
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let mut consensus = Consensus {
        _notify_shutdown: notify_shutdown,
        _shutdown_complete_tx: shutdown_complete_tx,
        _shutdown_complete_rx: shutdown_complete_rx,
    };

    tokio::select! {
        res = consensus._run() => {
            if let Err(err) = res {
                // TODO -- implement logging/tracing
                // https://github.com/orgs/SaitoTech/projects/5#card-61344938
                println!("{:?}", err);
            }
        },
        _ = shutdown => {
            println!("shutting down")
        }
    }

    Ok(())
}

impl Consensus {

    async fn _run(&mut self) -> crate::Result<()> {



/***
        let mut interval = time::interval(Duration::from_millis(1000));
        interval.tick().await;
println!("1");
        interval.tick().await;
println!("2");
        interval.tick().await;
println!("3");




	// initialize
	//let mut blockchain = Arc::new(RwLock::new(Blockchain::new()));
	//let rob1 = blockchain.clone();
	//let rob2 = blockchain.clone();
	//let mut mempool = Mempool::new();

	let blockchain = Arc::new(RwLock::new(Blockchain::new()));
	let mempool = Arc::new(RwLock::new(Mempool::new()));



/***

	mempool.start_bundling(&mempool).await;
println!("We have finished starting bundling...");


        let mut interval = time::interval(Duration::from_millis(3000));
        interval.tick().await;
println!("A");
        interval.tick().await;
println!("B");
        interval.tick().await;
println!("C");

	mempool.stop_bundling(&mempool).await;
println!("We have stopped bundling...");



	let lock = Arc::new(RwLock::new(Blockchain::new()));
	let lock_c = lock.clone();
	let lock_d = lock.clone();

	// first tokio thread
        tokio::spawn(async move {
            loop {
              //println!("loop 1 pre");
	      let mut lock_c_write = lock_c.write().await;
	      lock_c_write.updatestuff();
	      //lock_c_write.printstuff();
	      //let lock_c_write2 = lock_c_write.unwrap();
	      //lock_c.write.unwrap().printstuff();
              //println!("loop 1 post");
            }
        });

	// second tokio thread 
        tokio::spawn(async move {
            loop {
                //println!("loop 2 pre");
	        //let lock_d_read = lock_d.read().await;
	        //lock_d_read.printstuff();
                //println!("loop 2 post");
            }
        });

***/
/***
        let mut keypair = Arc::new(RwLock::new(Keypair::new()));
	let mut miner = Miner::new(keypair.read().unwrap().publickey().clone());
	let b1 = blockchain.read().unwrap();

	mempool.set_blockchain(b1);
        mempool.start_bundling();

	// inter-module channels
        let (saito_message_tx, mut saito_message_rx) = broadcast::channel(32);
	//let blockchain_channel_sender    = saito_message_tx.clone();
	//let mut blockchain_channel_receiver  = saito_message_tx.subscribe();
	//let miner_channel_sender         = saito_message_tx.clone();
	//let mut miner_channel_receiver       = saito_message_tx.subscribe();
	let mempool_channel_sender       = saito_message_tx.clone();
	//let mut mempool_channel_receiver     = saito_message_tx.subscribe();


        let miner_tx = saito_message_tx.clone();
        let mut miner_rx = saito_message_tx.subscribe();

	//
	// Async Thread sends message triggering TryBundle every second
	//
	// TODO - the mempool class should be able to independently manage the timer
	// and we should restrict our focus to sending messages telling it to start
	// and stop bundling.
	//
****/

        tokio::spawn(async move {
            loop {
		//b1.unwrap().printstuff();
/***
                saito_message_tx
                    .send(SaitoMessage::TryBundle)
                    .expect("error: TryBundle message failed to send");
***/
                //sleep(Duration::from_millis(1000));
            }
        });

/**
	let b2 = blockchain.write().unwrap();
        tokio::spawn(async move {

            loop {
	      b2.printstuff();
	      b2.updatestuff();
              sleep(Duration::from_millis(3000));
	    }
        });
***/



	// does this just prevent the main loop closing?

	loop {
/***
                while let Ok(message) = saito_message_rx.recv().await {
                    match message {
		        SaitoMessage::TryBundle { } => {
                  	    mempool.processSaitoMessage(message, &mempool_channel_sender);
		        }
                        SaitoMessage::Block { payload } => {
			    // transfer ownership of that block to me
                            let block = mempool.get_block(payload);
			    if block.is_none() {
				// bad block
			    } else {
				// send it to the blockchain
			    	blockchain.add_block(block.unwrap());
                            }
                        }
                        _ => {}
                   }
                }
***/
        }
    }

}
