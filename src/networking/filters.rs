use crate::networking::handlers::post_transaction_handler;
use crate::wallet::Wallet;
use std::convert::{Infallible};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Reply};
use warp::body;

use super::handlers::{get_block_handler, get_block_handler_json, handshake_complete_handler, handshake_init_handler, post_block_handler, ws_handler};
use super::network::Clients;
// 
// cargo run --bin walletcli print
// 02579d6ff84f661297f38e3eb20953824cfc279fee903a746b3dccb534677fd81a
// curl 127.0.0.1:3030/handshakeinit\?a={pubkey} > test_challenge
// curl 127.0.0.1:3030/handshakeinit\?a=02579d6ff84f661297f38e3eb20953824cfc279fee903a746b3dccb534677fd81a > test_challenge
// cargo run --bin walletcli sign test_challenge signed_challenge
// curl --data-binary "@signed_challenge" -X POST http://127.0.0.1:3030/handshakecomplete
// wscat -H socket-token:{token} -c ws://127.0.0.1:3030/wsconnect
// wscat -H socket-token:bf096eae9d673a5295560a8bd1a3ddf166516c5c09a7e482f734ae92aade6b9b -c ws://127.0.0.1:3030/wsconnect
// wscat -H socket-token:83d1178b7e6080ddcf3f4a273a2ef1554ea060fd15f6db647660cbc99e1faf67 -c ws://127.0.0.1:3030/wsconnect
// wscat -H socket-token:e6e5477c79ff669cc3e3eb5e909dc115d30e5a1142e9d80a5e238636a74c69b8 -c ws://127.0.0.1:3030/wsconnect
// websocat ws://127.0.0.1:3030/wsconnect -H socket-token:$TOKEN -b readfile:test_challenge
// 
// 
// GET http handshakeinit
// GET http handshakecomplete
// GET ws wsconnect
// POST ws wsconnect
// GET http block
// POST http sendtransaction
// POST http sendblockheader
// 

pub fn ws_route_filter(clients: &Clients) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("wsconnect")
        .and(warp::ws())
        .and(warp::header::value("socket-token"))
        .and(with_clients_filter(clients.clone()))
        .and_then(ws_handler)
}

pub fn handshake_complete_route_filter(clients: &Clients) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("handshakecomplete")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(warp::addr::remote())
        .and(with_clients_filter(clients.clone()))
        .and_then(handshake_complete_handler)
}

pub fn handshake_init_route_filter(wallet_lock: Arc<RwLock<Wallet>>) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::path("handshakeinit")
        .and(warp::query::raw())
        //.and(warp::path::end())
        .and(warp::addr::remote())
        .and(with_wallet(wallet_lock))
        .and_then(handshake_init_handler)
}

pub fn get_block_route_filter() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Copy {
    warp::path("block")
        .and(warp::path::param().and_then(get_block_handler))
}

pub fn get_json_block_route_filter() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Copy {
    warp::path("block")
        .and(warp::path("json"))
        .and(warp::path::param())
        .and_then(get_block_handler_json)
}

pub fn post_block_route_filter() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("sendblockheader"))
        .and(warp::path::end())
        .and(body::aggregate())
        .and_then(post_block_handler)
}

pub fn post_transaction_route_filter() -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("sendtransaction"))
        .and(warp::path::end())
        .and(body::aggregate())
        .and_then(post_transaction_handler)
}

fn with_clients_filter(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}
fn with_wallet(mempool_lock: Arc<RwLock<Wallet>>) -> impl Filter<Extract = (Arc<RwLock<Wallet>>,), Error = Infallible> + Clone {
    warp::any().map(move || mempool_lock.clone())
}