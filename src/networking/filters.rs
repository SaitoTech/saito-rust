use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::mempool::Mempool;
use crate::network::PEERS_DB_GLOBAL;
use crate::wallet::Wallet;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use warp::{body, Filter, Reply};

use super::handlers::{get_block_handler, post_transaction_handler, ws_upgrade_handler};
use crate::peer::PeersDB;

/// websocket upgrade filter.
pub fn ws_upgrade_route_filter(
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("wsopen")
        .and(warp::ws())
        .and(with_peers_filter())
        .and(with_wallet(wallet_lock))
        .and(with_mempool(mempool_lock))
        .and(with_blockchain(blockchain_lock))
        .and(with_broadcast_channel_sender(broadcast_channel_sender))
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

/// inject peers db lock
/// TODO Can this just be deleted? we should be able to just get the Peers DB from lazy_static global object PEERS_DB_GLOBAL
fn with_peers_filter() -> impl Filter<Extract = (Arc<RwLock<PeersDB>>,), Error = Infallible> + Clone
{
    warp::any().map(move || PEERS_DB_GLOBAL.clone())
}

/// inject wallet lock
fn with_wallet(
    wallet_lock: Arc<RwLock<Wallet>>,
) -> impl Filter<Extract = (Arc<RwLock<Wallet>>,), Error = Infallible> + Clone {
    warp::any().map(move || wallet_lock.clone())
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

/// inject blockchain lock
fn with_broadcast_channel_sender(
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
) -> impl Filter<Extract = (broadcast::Sender<SaitoMessage>,), Error = Infallible> + Clone {
    warp::any().map(move || broadcast_channel_sender.clone())
}
