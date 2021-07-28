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
    println!("client_connection");
    let (_client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, _client_rcv) = mpsc::unbounded_channel();

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
        // client_msg(id, msg, clients.clone(), wallet_lock.clone()).await;
    }

    clients.write().await.remove(&id);
    println!("{:?} disconnected", id);
}

async fn client_msg(id: SaitoHash, msg: Message, clients: Clients, wallet_lock: Arc<RwLock<Wallet>>) {
    println!("received message from {:?}: {:?}", id, msg);
    let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
    let command = String::from_utf8_lossy(api_message.message_name());

    println!("COMMAND {}", command);
    match &command.to_string()[..] {
        "SHAKINIT" => {
            tokio::spawn(async move {
                // sleep(Duration::from_millis(1000));
                // let serialized_handshake = socket_handshake_init(api_message, wallet_lock);
                if let Ok(serialized_handshake) = socket_handshake_init(api_message, wallet_lock).await {
                    let api_message = APIMessage {
                        message_name: String::from("SHAKCHLG").as_bytes().try_into().unwrap(),
                        message_id: 0,
                        message_data: serialized_handshake
                    };
                    let clients = clients.write().await;
                    let client = clients.get(&id).unwrap();
                    let _foo = client.sender.as_ref().unwrap().send(Ok(Message::binary(api_message.serialize())));
                }
            });
        },
        "SHAKCOMP" => {
            tokio::spawn(async move {
                if let Some(hash) = socket_handshake_complete(api_message, wallet_lock) {
                    // let api_message = APIMessage {
                    //     message_name: String::from("SHAKCHLG").as_bytes().try_into().unwrap(),
                    //     message_id: 0,
                    //     message_data: serialized_handshake
                    // };
                    let mut clients = clients.write().await;
                    let mut client = clients.get_mut(&id).unwrap();
                    client.has_handshake = true;
                    // let _foo = client.sender.as_ref().unwrap().send(Ok(Message::binary(api_message.serialize())));
                }
            });
        },
        _ => {}
    }
    // let req: Value = match from_str(&message) {
    //     Ok(v) => v,
    //     Err(e) => {
    //         eprintln!("error while parsing message request: {}", e);
    //         return;
    //     }
    // };

    // let topics_req: TopicsRequest = match from_str(&message) {
    //     Ok(v) => v,
    //     Err(e) => {
    //         eprintln!("error while parsing message to topics request: {}", e);
    //         return;
    //     }
    // };

    // let mut locked = clients.write().await;
    // if let Some(c) = locked.get_mut(id) {
    //   c.topic = topics_req.topics;
    // }

    // let mut locked = clients.write().await;
    // if let Some(c) = locked.get_mut(id) {
    //     if let Some(message_type) = req["type"].as_str() {
    //         match message_type {
    //             "handshake_init" => {
    //                 let message= req["message"].as_object().unwrap();

    //                 let peer_ip= message["ip"].as_array().unwrap();
    //                 let peer_octets= [
    //                     peer_ip[0].as_u64().unwrap() as u8,
    //                     peer_ip[1].as_u64().unwrap() as u8,
    //                     peer_ip[2].as_u64().unwrap() as u8,
    //                     peer_ip[3].as_u64().unwrap() as u8
    //                 ];
    //                 let peer_pubkey = message["publickey"].as_str().unwrap();

    //                 let wallet = wallet_lock.read().await;
    //                 let my_pubkey = wallet.get_publickey();
    //                 let my_privkey = wallet.get_privatekey();

    //                 let my_octets: [u8; 4] = [42, 42, 42, 42];

    //                 let challenge = HandshakeChallenge::new(my_octets, peer_octets, my_pubkey, peer_pubkey);
    //                 let serialized_challenge = challenge.serialize_with_sig(my_privkey);

    //                 if let Some(sender) = &c.sender {
    //                     let _ = sender.send(Ok(Message::text(serde_json::to_string(serialized_challenge))));
    //                 }

    //             },
    //             "handshake_complete" => {
    //                 let message= req["message"].as_object().unwrap();

    //                 let user_id = 1;
    //                 let pubkey: SaitoPublicKey = [1;33];

    //                 let (challenge, my_sig, their_sig) = HandshakeChallenge::deserialize_with_both_sigs(&bytes);

    //                 let peer_ip= message["ip"].as_array().unwrap();
    //                 let peer_octets= [
    //                     peer_ip[0].as_u64().unwrap() as u8,
    //                     peer_ip[1].as_u64().unwrap() as u8,
    //                     peer_ip[2].as_u64().unwrap() as u8,
    //                     peer_ip[3].as_u64().unwrap() as u8
    //                 ];
    //                 let peer_pubkey = message["publickey"].as_str().unwrap();
    //                 let peer_sig = message["sig"].as_str().unwrap();

    //                 if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
    //                     // return Err(warp::reject());
    //                 }
    //                 // if !verify(&hash(&bytes[..CHALLENGE_SIZE+64].to_vec()), their_sig, challenge.challengie_pubkey()) {
    //                 //     // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
    //                 //     return Err(warp::reject());
    //                 // }
    //                 // if !verify(&hash(&bytes[..CHALLENGE_SIZE].to_vec()), my_sig, challenge.challenger_pubkey()) {
    //                 //     // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
    //                 //     return Err(warp::reject());
    //                 // }

    //                 c.has_handshake = true;
    //             },
    //             _ => {},
    //         }
    //     }
    // }
}

/*
    The serialized handshake init message shoudl fit this format
        challenger_ip           4 bytes(IP as 4 bytes)
        challenger_pubkey       33 bytes (SECP256k1 compact form)
*/
pub async fn socket_handshake_init(message: APIMessage, wallet_lock: Arc<RwLock<Wallet>>) -> crate::Result<Vec<u8>> {
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
    // let peer_octets: [u8; 4] = match addr.unwrap().ip() {
    //     IpAddr::V4(ip4) => ip4.octets(),
    //     _ => panic!("Saito Handshake does not support IPV6"),
    // };

    let challenge = HandshakeChallenge::new(my_octets, peer_octets, my_pubkey, peer_pubkey);
    let serialized_challenge = challenge.serialize_with_sig(my_privkey);
    Ok(serialized_challenge)
}

pub fn socket_handshake_complete(message: APIMessage, _wallet_lock: Arc<RwLock<Wallet>>) -> Option<SaitoHash> {
    let bytes = message.message_data;
    println!("{:?}", bytes.len());
    // let pubkey: SaitoPublicKey = [1;33];

    let (challenge, my_sig, their_sig) = HandshakeChallenge::deserialize_with_both_sigs(&bytes);

    // if addr.is_none() {
    //     return Err(warp::reject());
    // }

    // let peer_octets: [u8; 4] = match addr.unwrap().ip() {
    //     IpAddr::V4(ip4) => ip4.octets(),
    //     _ => return Err(warp::reject()),
    // };
    // if challenge.challengie_ip_address() != peer_octets {
    //     return Err(warp::reject());
    // }
    if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
        println!("Error validating timestamp for handshake complete");
        return None;
    }
    if !verify(&hash(&bytes[..CHALLENGE_SIZE+64].to_vec()), their_sig, challenge.challengie_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        // return Err("ERROR WITH SIG VALIDATION");
        println!("Error with validating challengie sig");
        return None;
    }
    if !verify(&hash(&bytes[..CHALLENGE_SIZE].to_vec()), my_sig, challenge.challenger_pubkey()) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        println!("Error with validating challenger sig");
        return None;
    }

    Some(hash(&bytes))

    // println!("key {:?}", key);
    // clients.write().await.insert(
    //     key.clone(),
    //     Client {
    //         has_handshake: true,
    //         pubkey: Some(pubkey),
    //         topics: vec![],
    //         sender: None,
    //     },
    // );

    // Ok(true)
}