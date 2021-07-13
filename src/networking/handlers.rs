use crate::crypto::{SaitoHash, SaitoPublicKey, hash, verify};
use crate::networking::socket;
use crate::storage::Storage;
use crate::time::create_timestamp;
use crate::wallet::Wallet;
use tokio::sync::RwLock;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use crate::transaction::Transaction;
use crate::networking::network::{CHALLENGE_EXPIRATION_TIME, CHALLENGE_SIZE, HandshakeChallenge, Result};
use warp::{Buf, Rejection, Reply};
use super::network::{Client, Clients};

pub async fn ws_handler(ws: warp::ws::Ws, token_header_value: HeaderValue, clients: Clients) -> std::result::Result<impl Reply, Rejection> {

    let mut hex_id: [u8; 64] = [0; 64];
    hex_id.clone_from_slice(&token_header_value.as_bytes()[..64]);
    println!("token_header_value {:?}", token_header_value);
    println!("hex_id {:?}", hex_id);
    let mut id: SaitoHash = [0; 32];
    match hex::decode_to_slice(hex_id, &mut id as &mut [u8]) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(warp::reject())
        },
        _ => {}
    };

    println!("id {:?}", id);
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| socket::client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn handshake_complete_handler(hyper_bytes: Bytes, addr: Option<SocketAddr>, clients: Clients) -> Result<impl Reply> {
    println!("handshake_complete_handler");
    println!("{:?}", hyper_bytes.len());
    let user_id = 1;
    let pubkey: SaitoPublicKey = [1;33];
    let bytes = hyper_bytes[..].to_vec();
    println!("{:?}", bytes.len());
    let (challenge, my_sig, their_sig) = HandshakeChallenge::deserialize_with_both_sigs(&bytes);

    if addr.is_none() {
        return Err(warp::reject());
    }
    let peer_octets: [u8; 4] = match addr.unwrap().ip() {
        IpAddr::V4(ip4) => ip4.octets(),
        _ => return Err(warp::reject()),
    };
    if challenge.challengie_ip_address() != peer_octets {
        return Err(warp::reject());
    }
    if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
        return Err(warp::reject());
    }
    if !verify(&hash(&bytes[..CHALLENGE_SIZE+64].to_vec()), their_sig, challenge.challengie_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        return Err(warp::reject());
    }
    if !verify(&hash(&bytes[..CHALLENGE_SIZE].to_vec()), my_sig, challenge.challenger_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        return Err(warp::reject());
    }
    let key = hash(&bytes);

    println!("key {:?}", key);
    clients.write().await.insert(
        key.clone(),
        Client {
            user_id,
            pubkey: pubkey,
            sender: None,
        },
    );
    println!("handshake_complete_handler complete");
    Ok(hex::encode(key.to_vec()))
}

pub async fn handshake_init_handler(raw_query_str: String, addr: Option<SocketAddr>, wallet_lock: Arc<RwLock<Wallet>>) -> std::result::Result<impl Reply, Rejection> {
    // TODO get these from the wallet

    let wallet = wallet_lock.read().await;
    let my_pubkey = wallet.get_publickey();
    let my_privkey = wallet.get_privatekey();
    let mut hex_pubkey: [u8; 66] = [0; 66];
    hex_pubkey.clone_from_slice(raw_query_str[2..68].as_bytes());

    let mut peer_pubkey: SaitoPublicKey = [0u8; 33];
    match hex::decode_to_slice(hex_pubkey, &mut peer_pubkey as &mut [u8]) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        Err(_e) => return Err(warp::reject()),
        _ => {}
    };

    if addr.is_none() {
        return Err(warp::reject());
    }
    // TODO configure the node's IP somewhere...
    let my_octets: [u8; 4] = [42, 42, 42, 42];
    let peer_octets: [u8; 4] = match addr.unwrap().ip() {
        IpAddr::V4(ip4) => ip4.octets(),
        _ => panic!("Saito Handshake does not support IPV6"),
    };
    
    let challenge = HandshakeChallenge::new(my_octets, peer_octets, my_pubkey, peer_pubkey);
    let serialized_challenge = challenge.serialize_with_sig(my_privkey);
    Ok(serialized_challenge)
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
            Ok(vec![])
        }
    }
}
