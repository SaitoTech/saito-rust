use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::mempool::Mempool;
use crate::network::PEERS_DB_GLOBAL;
use crate::wallet::Wallet;
use crate::network::Network;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use warp::{body, Filter, Reply};

use super::handlers::{get_block_handler, post_transaction_handler, ws_upgrade_handler};

/// websocket upgrade filter.
pub fn ws_upgrade_route_filter(
    network_lock: Arc<RwLock<Network>>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("wsopen")
        .and(warp::ws())
        .and(with_network(network_lock))
        .and_then(ws_upgrade_handler)
}
/// get block filter.
/// TODO remove this? I believe we want ot use the socket for everything...
pub fn get_block_route_filter(
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("block").and(
        warp::path::param()
            .and(with_blockchain(blockchain_lock))
            .and_then(get_block_handler),
    )
}

/// POST tx filter.
/// TODO remove this? I believe we want ot use the socket for everything...
pub fn post_transaction_route_filter(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("sendtransaction"))
        .and(warp::path::end())
        .and(body::aggregate())
        .and(with_mempool(mempool_lock))
        .and(with_blockchain(blockchain_lock))
        .and_then(post_transaction_handler)
}

/// inject network lock
fn with_network(
    network_lock: Arc<RwLock<Network>>,
) -> impl Filter<Extract = (Arc<RwLock<Network>>,), Error = Infallible> + Clone {
    warp::any().map(move || network_lock.clone())
}
/// inject peers db lock
fn with_mempool(
    mempool_lock: Arc<RwLock<Mempool>>,
) -> impl Filter<Extract = (Arc<RwLock<Mempool>>,), Error = Infallible> + Clone {
    warp::any().map(move || mempool_lock.clone())
}
/// inject blockchain lock
fn with_blockchain(
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> impl Filter<Extract = (Arc<RwLock<Blockchain>>,), Error = Infallible> + Clone {
    warp::any().map(move || blockchain_lock.clone())
}

