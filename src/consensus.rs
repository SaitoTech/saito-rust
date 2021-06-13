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

	//
	// inter-module broadcast channels
        //
	let (broadcast_channel_sender, mut broadcast_channel_receiver) = broadcast::channel(32);
	//let blockchain_channel_sender    = saito_message_tx.clone();
	//let mut blockchain_channel_receiver  = saito_message_tx.subscribe();
	//let miner_channel_sender         = saito_message_tx.clone();
	//let mut miner_channel_receiver       = saito_message_tx.subscribe();
	//let mempool_channel_sender       = saito_message_tx.clone();
	//let mut mempool_channel_receiver     = saito_message_tx.subscribe();
        //let miner_tx = saito_message_tx.clone();
        //let mut miner_rx = saito_message_tx.subscribe();


	//
	//
	// all objects requiring multithread read / write access are
	// wrapped in Tokio::RwLock for read().await / write().await
	// access. This requires cloning the lock and that clone 
	// being sent into the async threads rather than the original
	//
	let blockchain_lock = Arc::new(RwLock::new(Blockchain::new()));
	let mempool_lock = Arc::new(RwLock::new(Mempool::new()));


	//
	// manually initialize components. we will flesh out the constructors
	// to avoid this once we have a better sense of what data is needed
	// universally in object creation.
	//
	{
	    let mut blockchain = blockchain_lock.write().await;
	    let mut mempool = mempool_lock.write().await;

	    let mempool_channel_sender = broadcast_channel_sender.clone();
	    mempool.set_broadcast_channel_sender(mempool_channel_sender);

	    let blockchain_channel_sender = broadcast_channel_sender.clone();
	    blockchain.set_broadcast_channel_sender(blockchain_channel_sender);

	}



	//
	// start mempool bundling activity
	//
	{

           broadcast_channel_sender
                        .send(SaitoMessage::StartBundling)
                        .expect("error: Consensus StartBundling message failed to send");

	   //let mut mempool = mempool_lock.write().await;
	   //let mempool_lock_clone = mempool_lock.clone();
	   //mempool.start_bundling(mempool_lock_clone).await;
	}

	


	//
	// SaitoMessage processing
	//
	// one of the main mechanisms for cross-channel communications
	// are the broadcasting of SaitoMessage indicators across
	// blockchain components. This message processing happens in the
	// main thread.
	//
	loop {
            while let Ok(message) = broadcast_channel_receiver.recv().await {
                match message {

                    SaitoMessage::StartBundling => {

			// should be write
	   		let mempool = mempool_lock.write().await;
	                let mempool_lock_clone = mempool_lock.clone();
                        mempool.start_bundling(mempool_lock_clone).await;

                    }

                    SaitoMessage::StopBundling => {

			// can be read
	   		let mempool = mempool_lock.read().await;
                        mempool.stop_bundling();

                    }

                    SaitoMessage::Block { payload } => {

			//
			// get write access to necessary components
			//
	   		let mut mempool = mempool_lock.write().await;
	   		let mut blockchain = blockchain_lock.write().await;

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
        }
    }
}	


