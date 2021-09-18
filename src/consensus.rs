use crate::crypto::SaitoHash;
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::networking::network::Network;
use crate::storage::Storage;
use crate::wallet::Wallet;
use crate::{blockchain::Blockchain, mempool::Mempool, transaction::Transaction};
use clap::{App, Arg};
use std::sync::Arc;
use tokio::signal;
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
pub async fn run() -> crate::Result<()> {
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
        _ = signal::ctrl_c() => {
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
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
	{
	    let mut wallet = wallet_lock.write().await;
	    wallet.load_keys(key_path, password);
	}

        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        // Load blocks from disk if configured
        let load_blocks_from_disk = match settings.get::<bool>("storage.load_blocks_from_disk") {
            Ok(can_load) => can_load,
            // we load by default
            Err(_) => true,
        };

        if load_blocks_from_disk {
            Storage::load_blocks_from_disk(blockchain_lock.clone()).await;
        }
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let miner_lock = Arc::new(RwLock::new(Miner::new(wallet_lock.clone())));
        let network = Network::new(
            settings,
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
        );
        tokio::select! {
            res = crate::mempool::run(
                mempool_lock.clone(),
                blockchain_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_receiver,
            ) => {
                if let Err(err) = res {
                    eprintln!("mempool err {:?}", err)
                }
            },
            res = crate::blockchain::run(
                blockchain_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_sender.subscribe()
            ) => {
                if let Err(err) = res {
                    eprintln!("blockchain err {:?}", err)
                }
            },
            res = crate::miner::run(
                miner_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_sender.subscribe()
            ) => {
                if let Err(err) = res {
                    eprintln!("miner err {:?}", err)
                }
            },
            res = network.run() => {
                if let Err(err) = res {
                    eprintln!("network err {:?}", err)
                }
            },
            res = network.run_server() => {
                if let Err(err) = res {
                    eprintln!("run_server err {:?}", err)
                }
            },
            _ = self._shutdown_complete_tx.closed() => {
                println!("Shutdown message complete")
            }
        }
        Ok(())
    }
}
