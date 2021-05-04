// mod block;
// mod blockchain;
// mod slip;
// mod transaction;
// mod utxoset;

use std::{thread::sleep, time::Duration};

use tokio;
use tokio::sync::mpsc;

use saito_rust::helper::{create_timestamp, format_timestamp};
use saito_rust::burnfee::BurnFee;
// use saito_rust::mempool::Mempool;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);

    let work_available = 0;
    let last_block_ts = create_timestamp() - 10000;

    tokio::spawn(async move {
        // sleep for 5 secs to simulate lottery game, then send a golden ticket
        sleep(Duration::from_millis(5000));
        tx.send("GOLDEN_TICKET").await;
    });

    loop {
        while let Some(message) = rx.recv().await {
            if message == "GOLDEN_TICKET" {
                let timestamp = create_timestamp();
                let work_needed = BurnFee::return_work_needed(
                    last_block_ts,
                    timestamp,
                    10.0,
                );
                println!(
                    "TS: {} -- WORK ---- {:?} -- {:?} --- TX COUNT {:?}",
                    format_timestamp(timestamp),
                    work_needed,
                    work_available,
                    0
                );

                if work_needed > work_available {
                    // Mempool::bundle_block();
                }
            }
        }
    }
}
