use std::{sync::Arc, thread::sleep, time::Duration};

use futures::{FutureExt, StreamExt};
// use crate::{Client, Clients};

use std::convert::TryInto;
use serde::Deserialize;
//use serde_json::from_str;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use crate::{crypto::{SaitoPublicKey, SaitoHash, verify, hash}, networking::network::{APIMessage, CHALLENGE_EXPIRATION_TIME, CHALLENGE_SIZE, Client, Clients, HandshakeChallenge}, time::{create_timestamp}, wallet::Wallet};

#[derive(Deserialize, Debug)]
pub struct TopicsRequest {
    topics: Vec<String>,
}
pub async fn client_connection(ws: WebSocket, id: SaitoHash, clients: Clients, mut client: Client, wallet_lock: Arc<RwLock<Wallet>>) {
    println!("client_connection");
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    println!("client channel created");
    client.sender = Some(client_sender);
    println!("client sender set");
    clients.write().await.insert(id.clone(), client);

    println!("{:?} connected", id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {:?}): {}", id.clone(), e);
                break;
            }
        };
        client_msg(id, msg, clients.clone(), wallet_lock.clone()).await;
    }

    clients.write().await.remove(&id);
    println!("{:?} disconnected", id);
}
pub async fn client_connection_old(ws: WebSocket, id: SaitoHash, clients: Clients, mut client: Client) {
    let (_client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, _client_rcv) = mpsc::unbounded_channel();

    client.sender = Some(client_sender);
    clients.write().await.insert(id.clone(), client);

    while let Some(result) = client_ws_rcv.next().await {
        match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {:?}): {}", id.clone(), e);
                break;
            }
        };
        // client_msg(id, msg, clients.clone(), wallet_lock.clone()).await;
    }

    clients.write().await.remove(&id);
    println!("{:?} disconnected", id);
}

async fn client_msg(id: SaitoHash, msg: Message, clients: Clients, wallet_lock: Arc<RwLock<Wallet>>) {
    let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
    let command = String::from_utf8_lossy(api_message.message_name());
    match &command.to_string()[..] {
        "SHAKINIT" => {
            tokio::spawn(async move {
                if let Ok(serialized_handshake_challenge) = new_handshake_challenge(&api_message, wallet_lock).await {
                    let api_message_response = APIMessage {
                        message_name: String::from("RESULT__").as_bytes().try_into().unwrap(),
                        message_id: api_message.message_id,
                        message_data: serialized_handshake_challenge
                    };
                    let clients = clients.write().await;
                    let client = clients.get(&id).unwrap();
                    let _foo = client.sender.as_ref().unwrap().send(Ok(Message::binary(api_message_response.serialize())));
                }
            });
        },
        "SHAKCOMP" => {
            tokio::spawn(async move {
                if let Some(_hash) = socket_handshake_complete(&api_message, wallet_lock) {
                    let api_message_response = APIMessage {
                        message_name: String::from("RESULT__").as_bytes().try_into().unwrap(),
                        message_id: api_message.message_id,
                        message_data: String::from("OK").as_bytes().try_into().unwrap(),
                    };
                    let mut clients = clients.write().await;
                    let mut client = clients.get_mut(&id).unwrap();
                    client.has_handshake = true;
                    let _foo = client.sender.as_ref().unwrap().send(Ok(Message::binary(api_message_response.serialize())));
                }
            });
        },
        _ => {}
    }
}

/*
    The serialized handshake init message shoudl fit this format
        challenger_ip           4 bytes(IP as 4 bytes)
        challenger_pubkey       33 bytes (SECP256k1 compact form)
*/
pub async fn new_handshake_challenge(message: &APIMessage, wallet_lock: Arc<RwLock<Wallet>>) -> crate::Result<Vec<u8>> {
    let wallet = wallet_lock.read().await;
    let my_pubkey = wallet.get_publickey();
    let my_privkey = wallet.get_privatekey();

    // let mut hex_pubkey: [u8; 66] = [0; 66];
    // hex_pubkey.clone_from_slice(raw_query_str[2..68].as_bytes());

    // let mut peer_pubkey: SaitoPublicKey = [0u8; 33];
    // match hex::decode_to_slice(hex_pubkey, &mut peer_pubkey as &mut [u8]) {
    //     // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
    //     Err(_e) => return Err(warp::reject()),
    //     _ => {}
    // };

    let mut peer_octets: [u8; 4] = [0; 4];
    peer_octets[0..4].clone_from_slice(&message.message_data[0..4]);
    let peer_pubkey: SaitoPublicKey = message.message_data[4..37].try_into().unwrap();

    // TODO configure the node's IP somewhere...
    let my_octets: [u8; 4] = [42, 42, 42, 42];

    // TODO get the IP of this socket connection somehow and validate it..
    // let peer_octets: [u8; 4] = match addr.unwrap().ip() {
    //     IpAddr::V4(ip4) => ip4.octets(),
    //     _ => panic!("Saito Handshake does not support IPV6"),
    // };

    let challenge = HandshakeChallenge::new(my_octets, peer_octets, my_pubkey, peer_pubkey);
    let serialized_challenge = challenge.serialize_with_sig(my_privkey);
    
    Ok(serialized_challenge)
}

pub fn socket_handshake_complete(message: &APIMessage, _wallet_lock: Arc<RwLock<Wallet>>) -> Option<SaitoHash> {
    
    let (challenge, my_sig, their_sig) = HandshakeChallenge::deserialize_with_both_sigs(&message.message_data());
    if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
        println!("Error validating timestamp for handshake complete");
        return None;
    }
    if !verify(&hash(&message.message_data[..CHALLENGE_SIZE+64].to_vec()), their_sig, challenge.challengie_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        // return Err("ERROR WITH SIG VALIDATION");
        println!("Error with validating challengie sig");
        return None;
    }
    if !verify(&hash(&message.message_data[..CHALLENGE_SIZE].to_vec()), my_sig, challenge.challenger_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        println!("Error with validating challenger sig");
        return None;
    }

    Some(hash(&message.message_data))
}