use saito_rust::{
    blockchain::Blockchain,
    crypto::{generate_random_bytes, hash, SaitoPrivateKey, SaitoPublicKey},
    mempool::Mempool,
    miner::Miner,
    slip::Slip,
    transaction::Transaction,
    wallet::Wallet,
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
            let transactions =
                generate_transactions(publickey, privatekey, txs_to_generate, bytes_per_tx);
            for tx in transactions.iter() {
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

    run(mempool_lock, blockchain_lock, miner_lock).await?;

    Ok(())
}

pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    miner_lock: Arc<RwLock<Miner>>,
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
        res = saito_rust::network::run(
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

pub fn generate_transactions(
    publickey: SaitoPublicKey,
    privatekey: SaitoPrivateKey,
    txs_to_generate: u32,
    bytes_per_tx: u32,
) -> Vec<Transaction> {
    (0..txs_to_generate)
        .into_par_iter()
        .map(|_| {
            let mut transaction = Transaction::new();
            transaction.set_message(
                (0..bytes_per_tx)
                    .into_par_iter()
                    .map(|_| rand::random::<u8>())
                    .collect(),
            );

            // as fake transactions, we set the UUID arbitrarily
            let mut input1 = Slip::new();
            input1.set_publickey(publickey);
            input1.set_amount(1000000);
            input1.set_uuid(hash(&generate_random_bytes(32)));

            let mut output1 = Slip::new();
            output1.set_publickey(publickey);
            output1.set_amount(1000000);
            output1.set_uuid([0; 32]);

            transaction.add_input(input1);
            transaction.add_output(output1);

            // sign ...
            transaction.sign(privatekey);
            transaction
        })
        .collect()
}
