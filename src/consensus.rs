use crate::{
    blockchain::Blockchain,
    crypto::hash,
    golden_ticket::{generate_golden_ticket_transaction, generate_random_data},
    keypair::Keypair,
    mempool::Mempool,
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
    /// Run consensus
    async fn _run(&mut self) -> crate::Result<()> {
        let (saito_message_tx, mut saito_message_rx) = broadcast::channel(32);
        let block_tx = saito_message_tx.clone();
        let miner_tx = saito_message_tx.clone();
        let mut miner_rx = saito_message_tx.subscribe();

        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let mut mempool = Mempool::new(keypair.clone());
        let mut blockchain = Blockchain::new();

        tokio::spawn(async move {
            loop {
                saito_message_tx
                    .send(SaitoMessage::TryBundle)
                    .expect("error: TryBundle message failed to send");
                sleep(Duration::from_millis(1000));
            }
        });

        tokio::spawn(async move {
            loop {
                while let Ok(message) = miner_rx.recv().await {
                    // simulate lottery game with creation of golden_ticket_transaction
                    match message {
                        SaitoMessage::Block { blk } => {
                            let golden_tx = generate_golden_ticket_transaction(
                                hash(&generate_random_data()),
                                &blk,
                                &keypair.read().unwrap(),
                            );
                            miner_tx
                                .send(SaitoMessage::Transaction { payload: golden_tx })
                                .unwrap();
                        }
                        _ => {}
                    }
                }
            }
        });

        loop {
            while let Ok(message) = saito_message_rx.recv().await {
                if let Some(block) = mempool.process(message, blockchain.get_latest_block_index()) {
                    blockchain.add_block(block.clone());
                    block_tx
                        .send(SaitoMessage::Block { payload: block })
                        .expect("Err: Could not send new block");
                }
            }
        }
    }
}
