// use crate::{Client, Clients};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::from_str;
//use serde_json::from_str;
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};

use crate::{crypto::{SaitoHash, SaitoPublicKey, hash, verify}, networking::network::{CHALLENGE_EXPIRATION_TIME, Client, Clients, HandshakeChallenge}, time::create_timestamp};
use serde_json::{Value};

#[derive(Deserialize, Debug)]
pub struct TopicsRequest {
    topics: Vec<String>,
}

pub async fn client_connection(ws: WebSocket, id: SaitoHash, clients: Clients, mut client: Client) {
    println!("client_connection");
    let (_client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, _client_rcv) = mpsc::unbounded_channel();

    // tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
    //     if let Err(e) = result {
    //         eprintln!("error sending websocket msg: {}", e);
    //     }
    // }));

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
        client_msg(&id, msg, &clients).await;
    }

    clients.write().await.remove(&id);
    println!("{:?} disconnected", id);
}

async fn client_msg(id: &SaitoHash, msg: Message, clients: &Clients) {
    println!("received message from {:?}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    let req: Value = match from_str(&message) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error while parsing message request: {}", e);
            return;
        }
    };

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

    let mut locked = clients.write().await;
    if let Some(c) = locked.get_mut(id) {
        if let Some(message_type) = req["type"].as_str() {
            match message_type {
                "handshake_init" => {
                    let message= req["message"].as_object().unwrap();

                    let peer_ip= message["ip"].as_array().unwrap();
                    let peer_octets= [
                        peer_ip[0].as_u64().unwrap() as u8,
                        peer_ip[1].as_u64().unwrap() as u8,
                        peer_ip[2].as_u64().unwrap() as u8,
                        peer_ip[3].as_u64().unwrap() as u8
                    ];
                    let peer_pubkey = message["publickey"].as_str().unwrap();

                    let wallet = wallet_lock.read().await;
                    let my_pubkey = wallet.get_publickey();
                    let my_privkey = wallet.get_privatekey();

                    let my_octets: [u8; 4] = [42, 42, 42, 42];

                    let challenge = HandshakeChallenge::new(my_octets, peer_octets, my_pubkey, peer_pubkey);
                    let serialized_challenge = challenge.serialize_with_sig(my_privkey);

                    if let Some(sender) = &c.sender {
                        let _ = sender.send(Ok(Message::text(serde_json::to_string(serialized_challenge))));
                    }

                },
                "handshake_complete" => {
                    let message= req["message"].as_object().unwrap();

                    let user_id = 1;
                    let pubkey: SaitoPublicKey = [1;33];

                    let (challenge, my_sig, their_sig) = HandshakeChallenge::deserialize_with_both_sigs(&bytes);

                    let peer_ip= message["ip"].as_array().unwrap();
                    let peer_octets= [
                        peer_ip[0].as_u64().unwrap() as u8,
                        peer_ip[1].as_u64().unwrap() as u8,
                        peer_ip[2].as_u64().unwrap() as u8,
                        peer_ip[3].as_u64().unwrap() as u8
                    ];
                    let peer_pubkey = message["publickey"].as_str().unwrap();
                    let peer_sig = message["sig"].as_str().unwrap();

                    if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
                        // return Err(warp::reject());
                    }
                    // if !verify(&hash(&bytes[..CHALLENGE_SIZE+64].to_vec()), their_sig, challenge.challengie_pubkey()) {
                    //     // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
                    //     return Err(warp::reject());
                    // }
                    // if !verify(&hash(&bytes[..CHALLENGE_SIZE].to_vec()), my_sig, challenge.challenger_pubkey()) {
                    //     // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
                    //     return Err(warp::reject());
                    // }

                    c.has_handshake = true;
                },
                _ => {},
            }
        }
        
    }
}
