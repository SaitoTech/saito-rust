// use crate::{Client, Clients};
use futures::StreamExt;
use serde::Deserialize;
//use serde_json::from_str;
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};

use crate::{crypto::SaitoHash, networking::network::{Client, Clients}};

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

async fn client_msg(id: &SaitoHash, msg: Message, _clients: &Clients) {
    println!("received message from {:?}: {:?}", id, msg);
    let _message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    // let topics_req: TopicsRequest = match from_str(&message) {
    //     Ok(v) => v,
    //     Err(e) => {
    //         eprintln!("error while parsing message to topics request: {}", e);
    //         return;
    //     }
    // };

    // let mut locked = clients.write().await;
    // if let Some(v) = locked.get_mut(id) {
    //     v.topics = topics_req.topics;
    // }
}
