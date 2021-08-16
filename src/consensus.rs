use crate::crypto::SaitoHash;
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::networking::network::Network;
use crate::networking::peer::{OutboundPeer, PeersDB};
use crate::wallet::Wallet;
use crate::{blockchain::Blockchain, mempool::Mempool, transaction::Transaction};
use clap::{App, Arg};
use std::{future::Future, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
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
    TestMessage,
    TestMessage2,
    TestMessage3,
    BlockchainNewLongestChainBlock { hash: SaitoHash, difficulty: u64 },
    BlockchainAddBlockSuccess { hash: SaitoHash },
    BlockchainAddBlockFailure { hash: SaitoHash },
    MinerNewGoldenTicket { ticket: GoldenTicket },
    MempoolNewBlock { hash: SaitoHash },
    MempoolNewTransaction { transaction: Transaction },
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

        let matches = App::new("Saito Runtime")
            .about("Runs a Saito Node")
            .arg(
                Arg::with_name("key_path")
                    .short("k")
                    .long("key_path")
                    .default_value("keyfile")
                    .takes_value(true)
                    .help("Path to encrypted key file"),
            )
            .arg(
                Arg::with_name("password")
                    .short("p")
                    .long("password")
                    .takes_value(true)
                    .help("amount to send"),
            )
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .takes_value(true)
                    .help("config file name"),
            )
            .get_matches();

        let config_name = match matches.value_of("config") {
            Some(name) => name,
            None => "config",
        };

        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(config_name))
            .unwrap();

        let key_path = matches.value_of("key_path").unwrap();
        let password = matches.value_of("password");
        //
        // all objects requiring multithread read / write access are
        // wrapped in Tokio::RwLock for read().await / write().await
        // access. This requires cloning the lock and that clone
        // being sent into the async threads rather than the original
        //
        // major classes get a clone of the broadcast channel sender
        // which is assigned to them on Object::run so they can
        // broadcast cross-system messages. See SaitoMessage ENUM above
        // for information on cross-system notifications.
        //
        let wallet_lock = Arc::new(RwLock::new(Wallet::new(key_path, password)));

        if let Some(ref sub_matches) = matches.subcommand_matches("client") {
            let server: String = match sub_matches.value_of("server") {
                Some(server) => String::from(server),
                None => String::from("ws://127.0.0.1:3000/wsopen"),
            };
            OutboundPeer::new(&server, wallet_lock.clone()).await;
        } else {
            let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
            let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
            let miner_lock = Arc::new(RwLock::new(Miner::new(wallet_lock.clone())));
            let peers_db_lock = Arc::new(RwLock::new(PeersDB::new()));

            let network = Network::new(wallet_lock.clone(), peers_db_lock.clone(), settings);

            tokio::select! {
                res = crate::mempool::run(
                    mempool_lock.clone(),
                    blockchain_lock.clone(),
                    broadcast_channel_sender.clone(),
                    broadcast_channel_receiver
                ) => {
                    if let Err(err) = res {
                        eprintln!("{:?}", err)
                    }
                },
                res = crate::blockchain::run(
                    blockchain_lock.clone(),
                    broadcast_channel_sender.clone(),
                    broadcast_channel_sender.subscribe()
                ) => {
                    if let Err(err) = res {
                        eprintln!("{:?}", err)
                    }
                },
                res = crate::miner::run(
                    miner_lock.clone(),
                    broadcast_channel_sender.clone(),
                    broadcast_channel_sender.subscribe()
                ) => {
                    if let Err(err) = res {
                        eprintln!("{:?}", err)
                    }
                },
                res = network.run(
                    mempool_lock.clone(),
                    blockchain_lock.clone(),
                    broadcast_channel_sender.clone(),
                    broadcast_channel_sender.subscribe()
                ) => {
                    if let Err(err) = res {
                        eprintln!("{:?}", err)
                    }
                }
                _ = self._shutdown_complete_tx.closed() => {
                    println!("Shutdown message complete")
                }
            }
        }

        Ok(())
    }
}
