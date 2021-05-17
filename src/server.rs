use std::future::Future;
use tokio::sync::{
    broadcast,
    broadcast::{Receiver, Sender},
    mpsc,
};
/// The server state which exposes a run method
/// initializes pre-connection state
struct Server {
    /// Broadcasts a shutdown signal to all active connections.
    _notify_shutdown: broadcast::Sender<()>,
    /// Used as part of the graceful shutdown process to wait for client
    /// connections to complete processing.
    _shutdown_complete_rx: mpsc::Receiver<()>,
    _shutdown_complete_tx: mpsc::Sender<()>,
}

/// Run the Saito server
pub async fn run(shutdown: impl Future) -> crate::Result<()> {
    // When the provided `shutdown` future completes, we must send a shutdown
    // message to all active connections. We use a broadcast channel for this
    // purpose. The call below ignores the receiver of the broadcast pair, and when
    // a receiver is needed, the subscribe() method on the sender is used to create
    // one.
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

    let mut server = Server {
        _notify_shutdown: notify_shutdown,
        _shutdown_complete_tx: shutdown_complete_tx,
        _shutdown_complete_rx: shutdown_complete_rx,
    };

    tokio::select! {
        res = server._run() => {
            if let Err(err) = res {
                // TODO -- implement logging/tracing
                println!("{:?}", err);
            }
        },
        _ = shutdown => {
            println!("shutting down")
        }
    }

    Ok(())
}

impl Server {
    /// Run the server
    async fn _run(&mut self) -> crate::Result<()> {
        let (_tx, mut rx): (Sender<bool>, Receiver<bool>) = broadcast::channel(1);

        loop {
            while let Ok(message) = rx.recv().await {
                println!("{:?}", message);
            }
        }
    }
}
