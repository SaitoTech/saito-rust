use std::{thread::sleep, time::Duration};
use std::sync::{Arc, RwLock};

use tokio::{self, sync::mpsc::{Sender, Receiver}};
use tokio::sync::{mpsc, broadcast};

use saito_rust::types::SaitoMessage;
use saito_rust::mempool::Mempool;
use saito_rust::block::Block;
use saito_rust::utxoset::UTXOSet;
use saito_rust::blockchain::Blockchain;
use saito_rust::wallet::Wallet;
use saito_rust::lottery::{Lottery, Miner};

#[tokio::main]
async fn main() {
    println!("RUNNING SAITO!");
    let (tx, mut rx) = mpsc::channel(32);
    let (target_tx, mut target_rx): (Sender<Block>, Receiver<Block>) = mpsc::channel(32);;

    let heart_beat_tx =  tx.clone();

    let wallet = Arc::new(RwLock::new(Wallet::new()));
    let mut utxoset= UTXOSet::new();
    let mut mempool = Mempool::new();
    let mut blockchain = Blockchain::new();
    let mut lottery = Lottery::new(Miner::new(), wallet.clone());

    tokio::spawn(async move {
        loop {
            heart_beat_tx.send(SaitoMessage::TryBundle { payload: true }).await;
            sleep(Duration::from_millis(1000));
        }
    });

    // sleep for 5 secs to simulate lottery game, then send a golden ticket
    tokio::spawn(async move {
        loop {
            while let Some(block) = target_rx.recv().await {
                println!("BLOCK RECEIVED! START MINING");
                let mut is_active = true;
                while is_active {
                    match lottery.play(block.clone()) {
                        Some(new_tx) => {
                            is_active = false;
                            println!("GOLDEN TICKET FOUND");
                            tx.send(SaitoMessage::Transaction { payload: new_tx }).await;
                        }
                        None => ()
                    }
                }
            }
        }
    });

    loop {
        while let Some(message) = rx.recv().await {
            mempool.process(message);

            let latest_block_header = blockchain.get_latest_block_header();

            if mempool.can_bundle_block(latest_block_header.clone()) {
                let block = mempool.bundle_block(&wallet, latest_block_header);
                println!("{:?}", block.clone());
                blockchain.add_block(block.clone(), &wallet);

                target_tx.send(block.clone()).await;
            }
        }
    }
}
