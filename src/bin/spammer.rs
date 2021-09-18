/*!
# Saito Transaction Spammer

TODO: Fill in these docs

*/
use saito_rust::{
    blockchain::Blockchain, mempool::Mempool, miner::Miner, networking::network::Network,
    transaction::Transaction, util::format_url_string, wallet::Wallet,
};

use clap::{App, Arg};

use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, RwLock};

use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;

use tracing::{event, Level};

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    tracing_subscriber::fmt::init();
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
        .arg(
            Arg::with_name("transactions")
                .short("txs")
                .long("transactions")
                .takes_value(true)
                .help("Number of transactins per block"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("b")
                .long("bytes")
                .takes_value(true)
                .help("Size of transation message in bytes"),
        )
        .get_matches();

    let config_name = matches.value_of("config").unwrap_or("config");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_name))
        .unwrap();

    let txs_to_generate: i32 = match matches.value_of("transactions") {
        Some(num) => num.parse::<i32>().unwrap(),
        None => 10,
    };

    let bytes_per_tx: i32 = match matches.value_of("bytes") {
        Some(size) => size.parse::<i32>().unwrap(),
        None => 1024,
    };

    let wallet_lock = Arc::new(RwLock::new(Wallet::new("test/testwallet", Some("asdf"))));
    let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));

    let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    let miner_lock = Arc::new(RwLock::new(Miner::new(wallet_lock.clone())));

    let network = Network::new(
        settings.clone(),
        wallet_lock.clone(),
        mempool_lock.clone(),
        blockchain_lock.clone(),
    );

    let publickey;
    let privatekey;

    {
        let wallet = wallet_lock.read().await;
        publickey = wallet.get_publickey();
        privatekey = wallet.get_privatekey();
    }

    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let host: [u8; 4] = settings.get::<[u8; 4]>("network.host").unwrap();
        let port: u16 = settings.get::<u16>("network.port").unwrap();

        let server_transaction_url =
            format!("http://{}/sendtransaction", format_url_string(host, port),);

        event!(Level::INFO, "{:?}", server_transaction_url);

        loop {
            let mut transactions: Vec<Transaction> = vec![];

            event!(Level::INFO, "TXS TO GENERATE: {:?}", txs_to_generate);

            for _i in 0..txs_to_generate {
                let mut transaction =
                    Transaction::generate_transaction(wallet_lock.clone(), publickey, 5000, 5000)
                        .await;
                transaction.set_message(
                    (0..bytes_per_tx)
                        .into_par_iter()
                        .map(|_| rand::random::<u8>())
                        .collect(),
                );

                // sign ...
                //transaction.generate_metadata();
                transaction.sign(privatekey);

                // add some test hops ...
                transaction
                    .add_hop_to_path(wallet_lock.clone(), publickey)
                    .await;
                transaction
                    .add_hop_to_path(wallet_lock.clone(), publickey)
                    .await;

                //println!("TRANSACTION: {:?}", transaction);

                transactions.push(transaction);
            }

            for tx in transactions {
                let bytes: Vec<u8> = tx.serialize_for_net();
                let result = client
                    .post(&server_transaction_url[..])
                    .body(bytes)
                    .send()
                    .await;
                match result {
                    Ok(_response) => {
                        // println!("response {:?}", response);
                    }
                    Err(error) => {
                        println!("Error sending tx to node: {}", error);
                    }
                }
            }
            sleep(Duration::from_millis(4000));
        }
    });

    run(mempool_lock, blockchain_lock, miner_lock, network).await?;

    Ok(())
}

pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    miner_lock: Arc<RwLock<Miner>>,
    network: Network,
) -> saito_rust::Result<()> {
    let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);
    tokio::select! {
        res = saito_rust::mempool::run(
            mempool_lock.clone(),
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
            broadcast_channel_receiver,
        ) => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        },
        res = saito_rust::blockchain::run(
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
            broadcast_channel_sender.subscribe()
        ) => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        },
        res = saito_rust::miner::run(
            miner_lock.clone(),
            broadcast_channel_sender.clone(),
            broadcast_channel_sender.subscribe()
        ) => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        },
        res = network.run_server() => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        },
        res = network.run() => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        },
    }
    println!("exiting..?");
    Ok(())
}
