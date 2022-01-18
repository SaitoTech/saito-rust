use crate::block::BlockType;
use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::mempool::Mempool;
use crate::network::Result;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use base58::ToBase58;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use warp::reject::Reject;
use warp::reply::Response;
use warp::{Buf, Rejection, Reply};
use crate::network::Network;

#[derive(Debug)]
struct Invalid;
impl Reject for Invalid {}

#[derive(Debug)]
struct AlreadyExists;
impl Reject for AlreadyExists {}

/// It seems that Warp handlers must return a Result<impl Reply>.
/// It looks like this was used as a simple way to turn a String
/// into a warp::Reply. It may be possilbe to use use Response::new
/// directly, or this may not be needed if we get rid of the http
/// handlers defined below...
struct Message {
    msg: String,
}

impl warp::Reply for Message {
    fn into_response(self) -> warp::reply::Response {
        Response::new(format!("message: {}", self.msg).into())
    }
}

/// websocket upgrade handler. accepts an http connection and upgrades it to WebSocket.
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Upgrade
/// Thanks, Ryan Dahl!!
pub async fn ws_upgrade_handler(
    ws: warp::ws::Ws,
    network_lock: Arc<RwLock<Network>>,
) -> std::result::Result<impl Reply, Rejection> {
    Ok(ws.on_upgrade(move |socket| {
        Network::add_remote_peer(
            socket,
            network_lock,
        )
    }))
}
/// POST tx filter.
/// TODO remove this? I believe we want ot use the socket for everything...
/// There is a SNDTRANS command which does this, but is currently unused
/// Let's keep this around for now in case we want to resurrect the spammer...
/// Once SNDBLKHD is being actively used, this should be deleted.
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
    if tx.validate(&blockchain.utxoset, &blockchain.staking) {
        let response = std::str::from_utf8(&tx.get_signature().to_base58().as_bytes())
            .unwrap()
            .to_string();
        let mut mempool = mempool_lock.write().await;
        mempool.add_transaction(tx).await;
        Ok(Message { msg: response })
    } else {
        Err(warp::reject::custom(Invalid))
    }
}

/// get block handler.
// TODO remove this. For now it is just in place as a simple means to transfer blocks to saito-lite so we
// can test the ability to serialize/deserialize blocks.
pub async fn get_block_handler(
    str_block_hash: String,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Result<impl Reply> {
    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash.clone(), &mut block_hash).expect("Failed to parse hash");
    {
        let blockchain = blockchain_lock.read().await;
        let block = blockchain.get_block(&block_hash).await;
        match block {
            Some(block) => {
                let block_bytes = block.serialize_for_net(BlockType::Full);
                Ok(block_bytes)
            }
            None => Err(warp::reject()),
        }
    }
}
