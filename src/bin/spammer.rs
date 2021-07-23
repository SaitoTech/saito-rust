use saito_rust::{
    blockchain::Blockchain, mempool::Mempool, miner::Miner, transaction::Transaction,
    wallet::Wallet, networking::network::Network,
};

use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, RwLock};

use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut txs_to_generate = 10;
    let mut bytes_per_tx = 1024;

    if !args.is_empty() {
        txs_to_generate = args[1].parse().unwrap();
        if args.len() > 1 {
            bytes_per_tx = args[2].parse().unwrap();
        }
    }

    let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
    let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    let miner_lock = Arc::new(RwLock::new(Miner::new(wallet_lock.clone())));
    let network = Network::new(wallet_lock.clone());

    let publickey;
    let privatekey;

    {
        let wallet = wallet_lock.read().await;
        publickey = wallet.get_publickey();
        privatekey = wallet.get_privatekey();
    }

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        // sleep(Duration::from_millis(5000));
        loop {
            let mut transactions: Vec<Transaction> = vec![];

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
                transaction.sign(privatekey);

                // add some test hops ...
                transaction
                    .add_hop_to_path(wallet_lock.clone(), publickey)
                    .await;
                transaction
                    .add_hop_to_path(wallet_lock.clone(), publickey)
                    .await;

                transactions.push(transaction);
            }

            for tx in transactions {
                let bytes: Vec<u8> = tx.serialize_for_net();
                let _res = client
                    .post("http://localhost:3030/transactions")
                    .body(bytes)
                    .send()
                    .await;
            }
            sleep(Duration::from_millis(2000));
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
            broadcast_channel_receiver
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
        res = network.run(
            broadcast_channel_sender.clone(),
            broadcast_channel_sender.subscribe()
        ) => {
            if let Err(err) = res {
                eprintln!("{:?}", err)
            }
        }
    }

    Ok(())
}
