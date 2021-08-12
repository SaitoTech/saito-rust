use std::{collections::HashMap, convert::TryInto};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use secp256k1::PublicKey;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{
    crypto::{hash, sign_blob, verify, SaitoPrivateKey, SaitoPublicKey},
    networking::{
        api_message::APIMessage, message_types::handshake_challenge::HandshakeChallenge,
        network::CHALLENGE_SIZE,
    },
    wallet::Wallet,
};

pub struct SaitoClient {
    read_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    write_sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    requests: HashMap<u32, [u8; 8]>,
    request_count: u32,
    wallet_lock: Arc<RwLock<Wallet>>,
}

impl SaitoClient {
    pub async fn new(peer: &str, wallet_lock: Arc<RwLock<Wallet>>) -> Self {
        let url = url::Url::parse(&peer).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (write_sink, read_stream) = ws_stream.split();

        let mut saito_client = SaitoClient {
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
            .send(String::from("SHAKINIT"), message_data)
            .await;

        while let Some(result) = saito_client.read_stream.next().await {
            let api_message = APIMessage::deserialize(&result.unwrap().into_data());
            saito_client.recv(&api_message).await;
        }
        saito_client
    }
    pub async fn send(&mut self, command: String, message: Vec<u8>) {
        let api_message = APIMessage::new(&command, self.request_count, message);
        self.requests
            .insert(self.request_count, api_message.message_name);
        self.request_count += 1;

        let _foo = self.write_sink.send(api_message.serialize().into()).await;
    }
    async fn recv(&mut self, api_message: &APIMessage) {
        let command_sent = self.requests.remove(&api_message.message_id);
        match command_sent {
            Some(command_name) => {
                match api_message.message_name_as_str().as_str() {
                    "RESULT__" => {
                        self.handle_response(command_name, api_message).await;
                    }
                    "ERROR___" => {
                        self.handle_error_response(command_name, api_message);
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
    async fn handle_response(&mut self, command_name: [u8; 8], response_api_message: &APIMessage) {
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKINIT" => {
                println!("GOT SHAKINIT RESPONSE");
                let (deserialize_challenge, signature) =
                    HandshakeChallenge::deserialize_with_sig(&response_api_message.message_data);
                let raw_challenge: [u8; CHALLENGE_SIZE] = response_api_message.message_data
                    [..CHALLENGE_SIZE]
                    .try_into()
                    .unwrap();

                println!("{:?}", deserialize_challenge);
                // TODO verify that this pubkey is actually the peer we are hoping to reach...
                let sig_is_valid = verify(
                    &hash(&raw_challenge.to_vec()),
                    signature,
                    deserialize_challenge.challengie_pubkey(),
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

                    self.send(String::from("SHAKCOMP"), signed_challenge).await;
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
    fn handle_error_response(&mut self, command_name: [u8; 8], _response_api_message: &APIMessage) {
        println!(
            "Error response for command {}",
            String::from_utf8_lossy(&command_name).to_string()
        );
    }
}
