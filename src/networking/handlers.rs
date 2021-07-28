use crate::crypto::{SaitoHash, hash};
use crate::networking::socket;
use crate::storage::Storage;
use crate::wallet::Wallet;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::sync::Arc;
use crate::transaction::Transaction;
use crate::networking::network::Result;
use warp::{Buf, Rejection, Reply};
use super::network::{Client, Clients};

pub async fn ws_upgrade_handler(ws: warp::ws::Ws, clients: Clients, wallet_lock: Arc<RwLock<Wallet>>) -> std::result::Result<impl Reply, Rejection> {
    let id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
//    let key = hash(&bytes);

    println!("id {:?}", id);
    let client = Client {
        has_handshake: true,
        pubkey: None,
        topics: vec![],
        sender: None,
    };
    Ok(ws.on_upgrade(move |socket| socket::client_connection(socket, id, clients, client, wallet_lock)))
}
pub async fn post_transaction_handler(mut body: impl Buf) -> Result<impl Reply> {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    let tx = Transaction::deserialize_from_net(buffer);
    println!("{:?}", tx.get_signature());

    Ok(warp::reply())
}

pub async fn post_block_handler(mut body: impl Buf) -> Result<impl Reply> {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    let tx = Transaction::deserialize_from_net(buffer);
    println!("{:?}", tx.get_signature());

    Ok(warp::reply())
}

pub async fn get_block_handler(str_block_hash: String) -> Result<impl Reply> {
    println!("GET BLOCK");
    let storage = Storage::new();

    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash, &mut block_hash).expect("Failed to parse hash");

    match storage.stream_block_from_disk(block_hash).await {
        Ok(block_bytes) => Ok(block_bytes),
        Err(_err) => {
            eprintln!("{:?}", _err);
            return Err(warp::reject())
        }
    }
}

pub async fn get_block_handler_json(str_block_hash: String) -> Result<impl Reply> {
    println!("GET BLOCK");
    let storage = Storage::new();

    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash, &mut block_hash).expect("Failed to parse hash");

    match storage.stream_json_block_from_disk(block_hash).await {
        Ok(json_data) => Ok(warp::reply::json(&json_data)),
        Err(_err) => {
            eprintln!("{:?}", _err);
            return Err(warp::reject())
        }
    }
}