use super::peer::{Peer, Peers};
use crate::blockchain::Blockchain;
use crate::crypto::{hash, SaitoHash};
use crate::mempool::Mempool;
use crate::networking::network::Result;
use crate::networking::socket;
use crate::storage::Storage;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use base58::ToBase58;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use warp::reject::Reject;
use warp::reply::Response;
use warp::{Buf, Rejection, Reply};

#[derive(Debug)]
struct Invalid;
impl Reject for Invalid {}

#[derive(Debug)]
struct AlreadyExists;
impl Reject for AlreadyExists {}

struct Message {
    msg: String,
}

impl warp::Reply for Message {
    fn into_response(self) -> warp::reply::Response {
        Response::new(format!("message: {}", self.msg).into())
    }
}

pub async fn ws_upgrade_handler(
    ws: warp::ws::Ws,
    peers: Peers,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> std::result::Result<impl Reply, Rejection> {
    let id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
    //    let key = hash(&bytes);

    println!("id {:?}", id);
    let peer = Peer {
        has_handshake: true,
        pubkey: None,
        sender: None,
    };
    Ok(ws.on_upgrade(move |socket| {
        socket::peer_connection(
            socket,
            id,
            peers,
            peer,
            wallet_lock,
            mempool_lock,
            blockchain_lock,
        )
    }))
}

pub async fn post_transaction_handler(
    mut body: impl Buf,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Result<impl Reply> {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    let mut tx = Transaction::deserialize_from_net(buffer);
    let blockchain = blockchain_lock.read().await;
    tx.generate_metadata(tx.inputs[0].get_publickey());
    if tx.validate(&blockchain.utxoset) {
        let response = std::str::from_utf8(&tx.get_signature().to_base58().as_bytes())
            .unwrap()
            .to_string();

        //let response = String::from("OK");
        let mut mempool = mempool_lock.write().await;
        let add_tx_result = mempool.add_transaction(tx).await;
        match add_tx_result {
            crate::mempool::AddTransactionResult::Accepted => Ok(Message { msg: response }),
            crate::mempool::AddTransactionResult::Exists => {
                Err(warp::reject::custom(AlreadyExists))
            }
            crate::mempool::AddTransactionResult::Invalid => {
                panic!("This appears unused, implement if needed");
            }
            crate::mempool::AddTransactionResult::Rejected => {
                panic!("This appears unused, implement if needed");
            }
        }
    } else {
        Err(warp::reject::custom(Invalid))
    }
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

    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash, &mut block_hash).expect("Failed to parse hash");

    match Storage::stream_block_from_disk(block_hash).await {
        Ok(block_bytes) => Ok(block_bytes),
        Err(_err) => {
            eprintln!("{:?}", _err);
            return Err(warp::reject());
        }
    }
}

pub async fn get_block_handler_json(str_block_hash: String) -> Result<impl Reply> {
    println!("GET BLOCK");

    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash, &mut block_hash).expect("Failed to parse hash");

    match Storage::stream_json_block_from_disk(block_hash).await {
        Ok(json_data) => Ok(warp::reply::json(&json_data)),
        Err(_err) => {
            eprintln!("{:?}", _err);
            return Err(warp::reject());
        }
    }
}
