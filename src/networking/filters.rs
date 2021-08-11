use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::networking::handlers::post_transaction_handler;
use crate::wallet::Wallet;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::body;
use warp::{Filter, Reply};

use super::handlers::{
    get_block_handler, get_block_handler_json, post_block_handler, ws_upgrade_handler,
};
use super::peer::Peers;

pub fn ws_upgrade_route_filter(
    clients: &Peers,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("wsopen")
        .and(warp::ws())
        .and(with_peers_filter(clients.clone()))
        .and(with_wallet(wallet_lock))
        .and(with_mempool(mempool_lock))
        .and(with_blockchain(blockchain_lock))
        .and_then(ws_upgrade_handler)
}

pub fn get_block_route_filter(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Copy {
    warp::path("block").and(warp::path::param().and_then(get_block_handler))
}

pub fn get_json_block_route_filter(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Copy {
    warp::path("block")
        .and(warp::path("json"))
        .and(warp::path::param())
        .and_then(get_block_handler_json)
}

pub fn post_block_route_filter(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("sendblockheader"))
        .and(warp::path::end())
        .and(body::aggregate())
        .and_then(post_block_handler)
}

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

fn with_peers_filter(peers: Peers) -> impl Filter<Extract = (Peers,), Error = Infallible> + Clone {
    warp::any().map(move || peers.clone())
}
fn with_wallet(
    wallet_lock: Arc<RwLock<Wallet>>,
) -> impl Filter<Extract = (Arc<RwLock<Wallet>>,), Error = Infallible> + Clone {
    warp::any().map(move || wallet_lock.clone())
}
fn with_mempool(
    mempool_lock: Arc<RwLock<Mempool>>,
) -> impl Filter<Extract = (Arc<RwLock<Mempool>>,), Error = Infallible> + Clone {
    warp::any().map(move || mempool_lock.clone())
}
fn with_blockchain(
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> impl Filter<Extract = (Arc<RwLock<Blockchain>>,), Error = Infallible> + Clone {
    warp::any().map(move || blockchain_lock.clone())
}
