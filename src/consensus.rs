use crate::crypto::SaitoHash;
use crate::golden_ticket::GoldenTicket;
use crate::miner::Miner;
use crate::networking::network::Network;
use crate::storage::Storage;
use crate::test_utilities::test_manager::TestManager;
use crate::wallet::Wallet;
use crate::{blockchain::Blockchain, mempool::Mempool, transaction::Transaction};
use clap::{App, Arg};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};

///
/// Saito has the following system-wide messages which may be sent and received
/// over the main broadcast channel. Convention has the message begin with the
/// class that is broadcasting.
///
#[derive(Clone, Debug)]
pub enum SaitoMessage {
    // broadcast when the longest chain block changes
    BlockchainNewLongestChainBlock { hash: SaitoHash, difficulty: u64 },
    // broadcast when a block is successfully added
    BlockchainAddBlockSuccess { hash: SaitoHash },
    // broadcast when a block is unsuccessful at being added
    BlockchainAddBlockFailure { hash: SaitoHash },
    // broadcast when the miner finds a golden ticket
    MinerNewGoldenTicket { ticket: GoldenTicket },
    // broadcast when the mempool produces a new block
    NetworkNewBlock { hash: SaitoHash },
    // broadcast when the mempool adds a new transaction
    NetworkNewTransaction { transaction: Transaction },
}

///
/// The entry point to the Saito consensus runtime
///
pub async fn run() -> crate::Result<()> {
    //
    // handle shutdown messages w/ broadcast channel
    //
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
    let mut consensus = Consensus {
        _notify_shutdown: notify_shutdown,
        _shutdown_complete_tx: shutdown_complete_tx,
        _shutdown_complete_rx: shutdown_complete_rx,
    };

    //
    // initiate runtime and handle results
    //
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

//
// The consensus state exposes a run method that main
// calls to initialize Saito state and prepare for
// shutdown.
//
struct Consensus {
    _notify_shutdown: broadcast::Sender<()>,
    _shutdown_complete_rx: mpsc::Receiver<()>,
    _shutdown_complete_tx: mpsc::Sender<()>,
}

impl Consensus {
    //
    // Run consensus
    //
    async fn run(&mut self) -> crate::Result<()> {
        //
        // create main broadcast channel
        //
        // all major classes have send/receive access to the main broadcast
        // channel, and can communicate by sending the events listed in the
        // SaitoMessage list above.
        //
        let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);

        //
        // handle command-line arguments
        //
        let matches = App::new("Saito Runtime")
            .about("Runs a Saito Node")
            .arg(
                Arg::with_name("key_path")
                    .short("k")
                    .long("key_path")
                    .default_value("keyfile")
                    .takes_value(true)
                    .help("Path to local wallet"),
            )
            .arg(
                Arg::with_name("password")
                    .short("p")
                    .long("password")
                    .takes_value(true)
                    .help("Password to decrypt wallet"),
            )
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .takes_value(true)
                    .help("config file name"),
            )
	    //
	    // TODO - hook up with Arg
	    //
            //.arg(
            //    Arg::with_name("spammer")
            //        .short("s")
            //        .long("spammer")
            //        .help("enable tx spamming"),
            //)
            .get_matches();

        let config_name = match matches.value_of("config") {
            Some(name) => name,
            None => "config",
        };
	
	let is_spammer_enabled = true;
	//
	// TODO - hook up with Arg above
	//
        //if matches.is_present("spammer") {
	//   enable_spammer = true;
        //};

        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(config_name))
            .unwrap();

        //let key_path = matches.value_of("key_path").unwrap();
        //let password = matches.value_of("password");

        //
        // generate
        //
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));

	//
	// TODO - load wallet ONLY if keyfile and password provided
	//
        //{
        //    let mut wallet = wallet_lock.write().await;
        //    wallet.load_keys(key_path, password);
        //}


        //
        // load blocks from disk and check chain
        //
        Storage::load_blocks_from_disk(blockchain_lock.clone()).await;
	{
	    let blockchain = blockchain_lock.read().await;
	    blockchain.print();
	}

        //
        // instantiate core classes
        //
        // all major classes which require multithread read / write access are
        // wrapped in Tokio::RwLock for read().await / write().await access.
        // we will send a clone of this RwLock object in any object that will
        // require direct access when initializing the object below.
        //
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let miner_lock = Arc::new(RwLock::new(Miner::new(wallet_lock.clone())));
        let network_lock = Arc::new(RwLock::new(Network::new(
             settings,
             wallet_lock.clone(),
             mempool_lock.clone(),
             blockchain_lock.clone(),
        )));

	//
	// start test_manager spammer
	//
	if is_spammer_enabled {
	    let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());
	    test_manager.spam_mempool(mempool_lock.clone());
	}


        //
        // initialize core classes.
        //
        // all major classes get a clone of the broadcast channel sender and
        // broadcast channel receiver. They must receive this clone and assign
        // it to a local object so they have read/write access to cross-system
        // messages.
        //
        // The SaitoMessage ENUM above contains a list of all cross-
        // system notifications.
        //
        tokio::select! {

        //
        // Mempool
        //
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

        //
        // Blockchain
        //
            res = crate::blockchain::run(
                blockchain_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_sender.subscribe()
            ) => {
                if let Err(err) = res {
                    eprintln!("blockchain err {:?}", err)
                }
            },

        //
        // Miner
        //
            res = crate::miner::run(
                miner_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_sender.subscribe()
            ) => {
                if let Err(err) = res {
                    eprintln!("miner err {:?}", err)
                }
            },

        //
        // Network
        //
            res = crate::networking::network::run(
                network_lock.clone(),
                broadcast_channel_sender.clone(),
                broadcast_channel_sender.subscribe()
            ) => {
                if let Err(err) = res {
                    eprintln!("miner err {:?}", err)
                }
            },
        //
        // Other
        //
            _ = self._shutdown_complete_tx.closed() => {
                println!("Shutdown message complete")
            }
        }


        Ok(())
    }
}
