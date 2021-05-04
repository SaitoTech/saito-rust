use std::{thread::sleep, time::Duration};
use std::sync::{Arc, RwLock};

use tokio;
use tokio::sync::{mpsc, broadcast};

use saito_rust::golden_ticket::GoldenTicket;
use saito_rust::types::SaitoMessage;
use saito_rust::mempool::Mempool;
use saito_rust::blockchain::Blockchain;
use saito_rust::wallet::Wallet;

use saito_rust::crypto::generate_keys;

#[tokio::main]
async fn main() {
    println!("RUNNING SAITO!");
    let (tx, mut rx) = mpsc::channel(32);

    // let (net_tx, mut net_rx) = broadcast::channel(32);

    // generate keys for our wallet
    let (_privatekey, _publickey) = generate_keys();

    // sleep for 5 secs to simulate lottery game, then send a golden ticket
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(5000));
            let message = SaitoMessage::GoldenTicket {
                payload: GoldenTicket::new(1, [0; 32], [0; 32], _publickey)
            };
            tx.send(message).await;
        }
    });

    let wallet = Arc::new(RwLock::new(Wallet::new()));
    let mut mempool = Mempool::new();
    let mut blockchain = Blockchain::new();

    loop {
        while let Some(message) = rx.recv().await {
            mempool.process(message);

            let latest_block_header = blockchain.get_latest_block_header();

            if mempool.can_bundle_block(latest_block_header.clone()) {
                let block = mempool.bundle_block(&wallet, latest_block_header);
                println!("{:?}", block.clone());
                blockchain.add_block(block.clone(), &wallet);
            }
        }
    }
}
