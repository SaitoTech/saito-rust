use crate::crypto::{hash, sign_blob, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::networking::message_types::handshake_challenge::HandshakeChallenge;
use crate::networking::network::CHALLENGE_SIZE;
use crate::storage::{Persistable, Storage};
use crate::wallet::Wallet;
use async_trait::async_trait;
use futures::stream::{SplitSink, SplitStream};
use macros::Persistable;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

use super::api_message::APIMessage;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};

pub type PeersDB = HashMap<SaitoHash, Box<dyn PeerTrait + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}

#[async_trait]
pub trait PeerTrait {
    // TODO Find a way to handle these commands as just [u8; 8] but with a simple way to
    // express "RESULT__", "SHAKINIT", etc without resorting to vec!['R' as u8, 'E' as u8, ...]
    async fn send(&mut self, command: &String, message_id: u32, message: Vec<u8>);
    async fn send_command(&mut self, command: &String, message: Vec<u8>);
    async fn send_response(&mut self, message_id: u32, message: Vec<u8>);
    async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>);
    fn set_has_completed_handshake(&mut self, has_completed_handshake: bool);
    fn get_has_completed_handshake(&mut self) -> bool;
    fn set_pubkey(&mut self, pub_key: SaitoPublicKey);
    fn get_pubkey(&mut self) -> SaitoPublicKey;
}

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Persistable)]
pub struct InboundPeer {
    pub has_handshake: bool,
    #[serde_as(as = "Option<[_; 33]>")]
    pub pubkey: Option<SaitoPublicKey>,
    #[serde(skip)]
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[async_trait]
impl PeerTrait for InboundPeer {
    async fn send(&mut self, _command: &String, _message_id: u32, _message: Vec<u8>) {
        // TODO implement this.
    }
    async fn send_command(&mut self, _command: &String, _message: Vec<u8>) {
        // TODO implement this.
        // self.requests
        //     .insert(self.request_count, String::from(command).as_bytes().try_into().unwrap());
        // self.request_count += 1;
        // self.send(command, self.request_count-1, message);
        //let _foo = self.write_sink.send(api_message.serialize().into()).await;
    }
    async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("RESULT__"), message_id, message)
            .await;
    }
    async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("ERROR___"), message_id, message)
            .await;
    }
    fn set_has_completed_handshake(&mut self, _has_completed_handshake: bool) {
        // TODO implement me!!
    }
    fn get_has_completed_handshake(&mut self) -> bool {
        // TODO implement me!!
        true
    }
    fn set_pubkey(&mut self, _pub_key: SaitoPublicKey) {
        // TODO implement me!!
    }
    fn get_pubkey(&mut self) -> SaitoPublicKey {
        // TODO implement me!!
        [0; 33]
    }
}
pub struct OutboundPeer {
    read_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    write_sink:
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::protocol::Message>,
    requests: HashMap<u32, [u8; 8]>,
    request_count: u32,
    wallet_lock: Arc<RwLock<Wallet>>,
}

#[async_trait]
impl PeerTrait for OutboundPeer {
    async fn send(&mut self, command: &String, message_id: u32, message: Vec<u8>) {
        let api_message = APIMessage::new(command, message_id, message);
        // TODO handle this unwrap more carefully:
        self.write_sink
            .send(api_message.serialize().into())
            .await
            .unwrap();
    }
    async fn send_command(&mut self, command: &String, message: Vec<u8>) {
        self.requests.insert(
            self.request_count,
            String::from(command).as_bytes().try_into().unwrap(),
        );
        self.request_count += 1;
        self.send(command, self.request_count - 1, message).await;
    }
    async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("RESULT__"), message_id, message)
            .await;
    }
    async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("ERROR___"), message_id, message)
            .await;
    }
    fn set_has_completed_handshake(&mut self, _has_completed_handshake: bool) {
        // TODO implement me!!
    }
    fn get_has_completed_handshake(&mut self) -> bool {
        // TODO implement me!!
        true
    }
    fn set_pubkey(&mut self, _pub_key: SaitoPublicKey) {
        // TODO implement me!!
    }
    fn get_pubkey(&mut self) -> SaitoPublicKey {
        // TODO implement me!!
        [0; 33]
    }
}

impl OutboundPeer {
    pub async fn new(peer: &str, wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        let url = url::Url::parse(&peer).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (write_sink, read_stream) = ws_stream.split();

        let mut saito_client = OutboundPeer {
            read_stream: read_stream,
            write_sink: write_sink,
            requests: HashMap::new(),
            request_count: 0,
            wallet_lock: wallet_lock,
        };
        let publickey: SaitoPublicKey;
        {
            let wallet = saito_client.wallet_lock.read().await;
            publickey = wallet.get_public_key();
        }

        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );

        saito_client
            .send_command(&String::from("SHAKINIT"), message_data)
            .await;

        while let Some(result) = saito_client.read_stream.next().await {
            let api_message = APIMessage::deserialize(&result.unwrap().into_data());
            saito_client.recv(&api_message).await;
        }
        saito_client
    }
    async fn recv(&mut self, api_message: &APIMessage) {
        let command_sent = self.requests.remove(&api_message.message_id);
        match command_sent {
            Some(command_name) => {
                match api_message.message_name_as_str().as_str() {
                    "RESULT__" => {
                        self.api_message_receive_response(command_name, api_message)
                            .await;
                    }
                    "ERROR___" => {
                        self.api_message_receive_error_response(command_name, api_message);
                    }
                    _ => {
                        // If anything other than RESULT__ or ERROR___ is sent here, there is a problem.
                        // The "client" side of the connection sends APIMessage and the "network/server" side responds.
                        // We should not handle anything other than RESULT__ and ERROR___ here.
                        println!(
                            "Unhandled command received by client... {}",
                            String::from_utf8_lossy(&command_name).to_string()
                        );
                    }
                }
            }
            None => {
                println!(
                    "Peer has sent a {} for a request we don't know about. Request ID: {}",
                    api_message.message_name_as_str(),
                    api_message.message_id
                );
            }
        }
    }
    async fn api_message_receive_response(
        &mut self,
        command_name: [u8; 8],
        response_api_message: &APIMessage,
    ) {
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKINIT" => {
                println!("GOT SHAKINIT RESPONSE");
                // let deserialize_challenge, signature) =
                let deserialize_challenge =
                    HandshakeChallenge::deserialize(&response_api_message.message_data);
                let signature = deserialize_challenge.opponent_node.sig.unwrap();
                let raw_challenge: [u8; CHALLENGE_SIZE] = response_api_message.message_data
                    [..CHALLENGE_SIZE]
                    .try_into()
                    .unwrap();

                println!("{:?}", deserialize_challenge);
                // TODO verify that this pubkey is actually the peer we are hoping to reach...
                let sig_is_valid = verify(
                    &hash(&raw_challenge.to_vec()),
                    signature,
                    deserialize_challenge.opponent_node.public_key,
                );
                if !sig_is_valid {
                    println!("Invalid signature sent in SHAKINIT");
                } else {
                    let privatekey: SaitoPrivateKey;
                    {
                        let wallet = self.wallet_lock.read().await;
                        privatekey = wallet.get_private_key();
                    }
                    let signed_challenge =
                        sign_blob(&mut response_api_message.message_data.to_vec(), privatekey)
                            .to_owned();

                    self.send_command(&String::from("SHAKCOMP"), signed_challenge)
                        .await;
                }
            }
            "SHAKCOMP" => {
                println!("GOT SHAKCOMP RESPONSE");
                println!(
                    "{}",
                    String::from_utf8_lossy(&response_api_message.message_data()).to_string()
                );
            }
            _ => {}
        }
    }
    fn api_message_receive_error_response(
        &mut self,
        command_name: [u8; 8],
        _response_api_message: &APIMessage,
    ) {
        println!(
            "Error response for command {}",
            String::from_utf8_lossy(&command_name).to_string()
        );
    }
}
