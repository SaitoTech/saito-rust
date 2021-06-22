use crate::{
    mempool::Mempool,
};
use std::{
    future::Future,
    sync::{Arc},
};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use tracing::{span, Level};


//
// Broadcast Messages sent between components in the 
//
pub enum SaitoMessage {
    StartBundling,
    StopBundling,
}


/// The consensus state which exposes a run method
/// initializes Saito state
struct Consensus {
    _notify_shutdown: broadcast::Sender<()>,
    _shutdown_complete_rx: mpsc::Receiver<()>,
    _shutdown_complete_tx: mpsc::Sender<()>,
}


/// Run the Saito consensus runtime
pub async fn run(shutdown: impl Future) -> crate::Result<()> {

    // 
    // handle shutdown messages using broadcast channel
    //
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
                eprintln!("{:?}", err);
            }
        },
        _ = shutdown => {
            println!("Shutting down!")
        }
    }

    Ok(())
}



impl Consensus {

    /// Run consensus
    async fn _run(&mut self) -> crate::Result<()> {

 	//
        // create inter-module broadcast channels
        //
        let (broadcast_channel_sender, mut broadcast_channel_receiver) = broadcast::channel(32);

        //
        // all objects requiring multithread read / write access are
        // wrapped in Tokio::RwLock for read().await / write().await
        // access. This requires cloning the lock and that clone
        // being sent into the async threads rather than the original
        //
        // major classes get a clone of the broadcast channel sender
        // on initialization so they can broadcast cross-system messages
        //
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(
	    broadcast_channel_sender.clone()
	)));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(
	    broadcast_channel_sender.clone()
	)));


        //
        // start mempool bundling activity
        //
        broadcast_channel_sender
            .send(SaitoMessage::StartBundling)
            .expect("error: Consensus StartBundling message failed to send");



        loop {
            while let Ok(message) = saito_message_rx.recv().await {
                match message {
                }
            }
        }
    }
}
