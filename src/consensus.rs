use crate::{
    block::Block,
    blockchain::{AddBlockEvent, BLOCKCHAIN_GLOBAL},
    crypto::hash_bytes,
    golden_ticket::{generate_golden_ticket_transaction, generate_random_data},
    keypair::Keypair,
    mempool::Mempool,
    network::Network,
    types::SaitoMessage,
    utxoset::UtxoSet,
};
use std::{
    future::Future,
    sync::{Arc, Mutex, RwLock},
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

    let network = Network {};

    tokio::select! {
        res = consensus._run() => {
            if let Err(err) = res {
                // TODO -- implement logging/tracing
                // https://github.com/orgs/SaitoTech/projects/5#card-61344938
                eprintln!("{:?}", err);
            }
        },
        res2 = network.start() => {
            if let Err(err) = res2 {
                eprintln!("server error: {}", err);
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
        {
            let blockchain_mutex = Arc::clone(&BLOCKCHAIN_GLOBAL);
            let mut blockchain = blockchain_mutex.lock().unwrap();

            let mut paths: Vec<_> = blockchain
                .storage
                .list_files_in_blocks_dir()
                .map(|r| r.unwrap())
                .collect();

            paths.sort_by_key(|dir| dir.path());
            for path in paths {
                let bytes = blockchain
                    .storage
                    .read(&path.path().to_str().unwrap())
                    .unwrap();
                let block = Block::from(bytes);

                let block_hash = block.hash().clone();
                match blockchain.add_block(block) {
                    AddBlockEvent::AcceptedAsLongestChain
                    | AddBlockEvent::AcceptedAsNewLongestChain => {
                        blockchain.get_block_by_hash(&block_hash).unwrap();
                    }
                    fail_message => {
                        println!("{:?}", fail_message)
                    }
                }
            }
        }
        let (saito_message_tx, mut saito_message_rx) = broadcast::channel(32);

        let block_tx = saito_message_tx.clone();
        let miner_tx = saito_message_tx.clone();

        let keypair = Arc::new(RwLock::new(Keypair::new()));
        let utxoset = Arc::new(Mutex::new(UtxoSet::new()));

        let mut mempool = Mempool::new(keypair.clone(), utxoset);
        
        tokio::spawn(async move {
            loop {
                saito_message_tx
                    .send(SaitoMessage::TryBundle)
                    .expect("error: TryBundle message failed to send");
                sleep(Duration::from_millis(1000));
            }
        });

        loop {
            while let Ok(message) = saito_message_rx.recv().await {
                match message {
                    SaitoMessage::NewBlock { payload } => {
                        let golden_tx = generate_golden_ticket_transaction(
                            hash_bytes(&generate_random_data()),
                            payload,
                            &keypair.read().unwrap(),
                        );

                        miner_tx
                            .send(SaitoMessage::Transaction { payload: golden_tx })
                            .unwrap();
                    }
                    SaitoMessage::TryBundle => {
                        if let Some(block) = mempool.process(message) {
                            let blockchain_mutex = Arc::clone(&BLOCKCHAIN_GLOBAL);
                            let mut blockchain = blockchain_mutex.lock().unwrap();
                            let block_hash = block.hash().clone();
                            
                            match blockchain.add_block(block) {
                                AddBlockEvent::AcceptedAsLongestChain
                                | AddBlockEvent::AcceptedAsNewLongestChain => {
                                    block_tx
                                        .send(SaitoMessage::NewBlock {
                                            payload: block_hash,
                                        })
                                        .unwrap();
                                }
                                fail_message => {
                                    println!("WE MISSED LONGEST CHAIN, WHAT HAPPENED?");
                                    println!("{:?}", fail_message)
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
