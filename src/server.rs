
use std::{thread::sleep, time::Duration};
use std::sync::{Arc, RwLock, Mutex};

use tokio::{self, join, sync::mpsc::{Sender, Receiver}};
use tokio::sync::{mpsc, broadcast};

use crate::types::SaitoMessage;
use crate::mempool::Mempool;
use crate::utxoset::UTXOSet;
use crate::blockchain::Blockchain;
use crate::wallet::Wallet;
use crate::lottery::{Lottery, Miner};

struct Server {}

pub async fn run() -> crate::Result<()> {
    let mut server = Server {};

    tokio::select! {
        res = server.run() => {
            // catch any errors here and shutdown the process gracefully
            if let Err(err) = res {
                println!("{:?}", err);
            }
        }
        // TODO -- implement shutdown logic here
    }

    Ok(())
}

impl Server {
    async fn run(&mut self) -> crate::Result<()> {
        let (tx, mut rx) = broadcast::channel(32);
        let mempool_rx = tx.subscribe();
        let mut miner_rx = tx.subscribe();

        let heart_beat_tx =  tx.clone();
        let miner_tx =  tx.clone();
        let blockchain_tx =  tx.clone();

        let wallet = Wallet::new();
        let blockchain = Arc::new(Mutex::new(Blockchain::new(wallet.clone())));
        let mut utxoset= UTXOSet::new();
        let mut mempool = Mempool::new(wallet.clone(), mempool_rx, tx);
        let mut lottery = Lottery::new(Miner::new(), wallet.clone());

        tokio::spawn(async move {
            loop {
                heart_beat_tx.send(SaitoMessage::TryBundle { payload: true })
                    .expect("error: try bundle message failed to send");
                sleep(Duration::from_millis(1000));
            }
        });

        // sleep for 5 secs to simulate lottery game, then send a golden ticket
        tokio::spawn(async move {
            loop {
                while let Ok(message) = miner_rx.recv().await {
                    // simulate lottery game with creation of golden_ticket_transaction
                    match message {
                        SaitoMessage::NewTargetBlock { payload } => {
                            let golden_ticket_tx = lottery.create_solution(payload);
                            miner_tx
                              .send(SaitoMessage::Transaction { payload: golden_ticket_tx })
                              .unwrap();
                        },
                        _ => {}
                    }
                }
            }
        });

        let mempool_blkchain_clone = Arc::clone(&blockchain);
        tokio::spawn(async move {
            mempool.run(&mempool_blkchain_clone)
                .await
                .unwrap();
        });

        loop {
            while let Ok(message) = rx.recv().await {
                match message {
                    SaitoMessage::CandidateBlock { payload } => {
                        &blockchain
                          .lock()
                          .unwrap()
                          .add_block(payload.clone(), &mut utxoset);
                        blockchain_tx.send(SaitoMessage::NewTargetBlock { payload: payload.clone() })
                            .expect("Err: Could not send block to lottery game");
                    },
                    _ => {}
                }
            }
        }
    }
}