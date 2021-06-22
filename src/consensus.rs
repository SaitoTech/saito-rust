use crate::{blockchain::Blockchain, mempool::Mempool};
use std::{future::Future, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use tracing::{span, Level};

/// The consensus state which exposes a run method
/// initializes Saito state
struct Consensus {
    _notify_shutdown: broadcast::Sender<()>,
    _shutdown_complete_rx: mpsc::Receiver<()>,
    _shutdown_complete_tx: mpsc::Sender<()>,
}

/// The types of messages broadcast over the main
/// broadcast channel in normal operations.
#[derive(Clone, Debug)]
pub enum SaitoMessage {
    TryBundle,
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
        res = consensus.run() => {
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
    async fn run(&mut self) -> crate::Result<()> {
        //
        // create inter-module broadcast channels
        //
        let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);

        //
        // all objects requiring multithread read / write access are
        // wrapped in Tokio::RwLock for read().await / write().await
        // access. This requires cloning the lock and that clone
        // being sent into the async threads rather than the original
        //
        // major classes get a clone of the broadcast channel sender
        // on initialization so they can broadcast cross-system messages.
        // submission on init avoids the need for constant checks to see
        // whether the channels exist and unwrapping them when sending
        // messages, as well as setters.
        //
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(
            broadcast_channel_sender.clone(),
        )));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new()));

        tokio::select! {
            res = crate::mempool::run(
                mempool_lock,
                blockchain_lock,
                broadcast_channel_sender.clone(),
                broadcast_channel_receiver
            ) => {
                if let Err(err) = res {
                    eprintln!("{:?}", err)
                }
            },
            _ = self._shutdown_complete_tx.closed() => {
                println!("Shutdown message complete")
            }
        }

        Ok(())
    }
}
