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
    sync::{Arc, RwLock},
    thread::sleep,
    time::Duration,
};
use tokio::sync::{broadcast, mpsc};

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

	// initialize modules
	let mut blockchain = Blockchain::new();
        let mut keypair = Arc::new(RwLock::new(Keypair::new()));
	let mut miner = Miner::new(keypair.read().unwrap().publickey().clone());
	let mut mempool = Mempool::new();

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


        tokio::spawn(async move {
            loop {
                saito_message_tx
                    .send(SaitoMessage::TryBundle)
                    .expect("error: TryBundle message failed to send");
                sleep(Duration::from_millis(1000));
            }
        });




	// does this just prevent the main loop closing?

	loop {
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
        }
    }

}
