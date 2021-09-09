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

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    //let args: Vec<String> = std::env::args().collect();

    Ok(())
}
