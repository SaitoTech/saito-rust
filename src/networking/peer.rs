use crate::blockchain::Blockchain;
use crate::crypto::{hash, sign_blob, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::{AddTransactionResult, Mempool};
use crate::networking::message_types::handshake_challenge::HandshakeChallenge;
use crate::networking::network::CHALLENGE_SIZE;
use crate::time::create_timestamp;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use async_trait::async_trait;
use futures::stream::{SplitSink, SplitStream};
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

use super::api_message::APIMessage;
use super::network::CHALLENGE_EXPIRATION_TIME;

use futures::{FutureExt, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};

pub type PeersDB = HashMap<SaitoHash, Box<dyn PeerTrait + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}

#[derive(Clone)]
pub struct BasePeer {
    has_completed_handshake: bool,
    public_key: Option<SaitoPublicKey>,
    requests: HashMap<u32, [u8; 8]>,
    request_count: u32,
    peers_db_lock: Arc<RwLock<PeersDB>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
}

#[async_trait]
pub trait PeerTrait {
    // TODO Find a way to handle these commands as just [u8; 8] but with a simple way to
    // express "RESULT__", "SHAKINIT", etc without resorting to vec!['R' as u8, 'E' as u8, ...]
    async fn send(&mut self, command: &String, message_id: u32, message: Vec<u8>);
    async fn send_command(&mut self, command: &String, message: Vec<u8>);
    async fn send_response(&mut self, message_id: u32, message: Vec<u8>);
    async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>);
    async fn handle_peer_command(&mut self, api_message: &APIMessage, peer_id: SaitoHash);
    fn set_has_completed_handshake(&mut self, has_completed_handshake: bool);
    fn get_has_completed_handshake(&self) -> bool;
    fn set_public_key(&mut self, pub_key: SaitoPublicKey);
    fn get_public_key(&self) -> Option<SaitoPublicKey>;
    async fn handle_peer_command_response(
        &mut self,
        command_name: [u8; 8],
        peer_id: SaitoHash,
        response_api_message: &APIMessage,
    );
    fn handle_peer_command_error_response(
        &mut self,
        command_name: [u8; 8],
        _response_api_message: &APIMessage,
    );
}

pub struct InboundPeer {
    pub base_peer: BasePeer,
    sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

pub struct OutboundPeer {
    pub base_peer: BasePeer,
    read_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    write_sink:
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::protocol::Message>,
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
    async fn handle_peer_command(&mut self, api_message_orig: &APIMessage, peer_id: SaitoHash) {
        let api_message = api_message_orig.clone();
        let wallet_lock = self.base_peer.wallet_lock.clone();
        let peers_db_lock = self.base_peer.peers_db_lock.clone();
        let mempool_lock = self.base_peer.mempool_lock.clone();
        let blockchain_lock = self.base_peer.blockchain_lock.clone();
        match api_message_orig.message_name_as_string().as_str() {
            "RESULT__" => {
                let command_sent = self.base_peer.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_command_response(command_name, peer_id, &api_message)
                            .await;
                    }
                    None => {
                        println!(
                            "Peer has sent a {} for a request we don't know about. Request ID: {}",
                            api_message.message_name_as_string(),
                            api_message.message_id
                        );
                    }
                }
            }
            "ERROR___" => {
                let command_sent = self.base_peer.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_command_error_response(command_name, &api_message);
                    }
                    None => {
                        println!(
                            "Peer has sent a {} for a request we don't know about. Request ID: {}",
                            api_message.message_name_as_string(),
                            api_message.message_id
                        );
                    }
                }
            }
            "SHAKINIT" => {
                tokio::spawn(async move {
                    if let Ok(serialized_handshake_challenge) =
                        new_handshake_challenge(&api_message, wallet_lock.clone()).await
                    {
                        let mut peers_db = peers_db_lock.write().await;
                        let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
                        // peer.send_response(api_message.message_id, serialized_handshake_challenge)
                        //     .await;:
                        peer.send_command(&String::from("SHAKCOMP"), serialized_handshake_challenge)
                            .await;
                        println!("INITIALIZING HANDSHAKE");
                    }
                });
            }
            "SHAKCOMP" => {
                tokio::spawn(async move {
                    match socket_handshake_verify(&api_message.message_data()) {
                        Some(deserialize_challenge) => {
                            socket_handshake_complete(
                                peers_db_lock.clone(),
                                peer_id,
                                deserialize_challenge.opponent_pubkey(),
                            )
                            .await;
                            let mut peers_db = peers_db_lock.write().await;
                            let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
                            peer.send_response(
                                api_message.message_id,
                                String::from("OK").as_bytes().try_into().unwrap(),
                            )
                            .await;
                            println!("COMPLETE!");
                        }
                        None => {
                            println!("Error verifying peer handshake signature");
                        }
                    }
                });
            }
            "REQBLOCK" => {
                tokio::spawn(async move {
                    let message_id = api_message.message_id;

                    let mut peers_db = peers_db_lock.write().await;
                    let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

                    if let Some(bytes) = socket_req_block(&api_message, blockchain_lock).await {
                        let message_data = String::from("OK").as_bytes().try_into().unwrap();
                        peer.send_response(message_id, message_data).await;
                        peer.send_command(&String::from("SNDBLOCK"), bytes).await;
                    } else {
                        let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
                        peer.send_error_response(message_id, message_data).await;
                    }
                });
            }
            "REQBLKHD" => {
                tokio::spawn(async move {
                    let message_id = api_message.message_id;
                    let mut peers_db = peers_db_lock.write().await;
                    let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

                    if let Some(bytes) =
                        socket_send_block_header(&api_message, blockchain_lock).await
                    {
                        let message_data = String::from("OK").as_bytes().try_into().unwrap();
                        peer.send_response(message_id, message_data).await;
                        peer.send_command(&String::from("SNDBLKHD"), bytes).await;
                    } else {
                        let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
                        peer.send_error_response(message_id, message_data).await;
                    }
                });
            }
            "REQCHAIN" => {
                tokio::spawn(async move {
                    let message_id = api_message.message_id;

                    let mut peers_db = peers_db_lock.write().await;
                    let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

                    if let Some(bytes) = socket_send_blockchain(&api_message, blockchain_lock).await
                    {
                        let message_data = String::from("OK").as_bytes().try_into().unwrap();
                        peer.send_response(message_id, message_data).await;
                        peer.send_command(&String::from("SNDBLKHD"), bytes).await;
                    } else {
                        let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
                        peer.send_error_response(message_id, message_data).await;
                    }
                });
            }
            "SNDTRANS" => {
                tokio::spawn(async move {
                    if let Some(tx) = socket_receive_transaction(api_message.clone()) {
                        let mut mempool = mempool_lock.write().await;
                        let mut peers_db = peers_db_lock.write().await;
                        let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
                        match mempool.add_transaction(tx).await {
                            AddTransactionResult::Accepted | AddTransactionResult::Exists => {
                                // TODO The tx is accepted, propagate it to all available peers
                                peer.send_response(
                                    api_message.message_id,
                                    String::from("OK").as_bytes().try_into().unwrap(),
                                )
                                .await;
                            }
                            AddTransactionResult::Invalid => {
                                peer.send_error_response(
                                    api_message.message_id,
                                    String::from("Invalid").as_bytes().try_into().unwrap(),
                                )
                                .await;
                            }
                            AddTransactionResult::Rejected => {
                                peer.send_error_response(
                                    api_message.message_id,
                                    String::from("Invalid").as_bytes().try_into().unwrap(),
                                )
                                .await;
                            }
                        }
                    }
                });
            }
            "SNDCHAIN" => {
                tokio::spawn(async move {
                    let _message_id = api_message.message_id;
                });
            }
            "SNDBLKHD" => {
                tokio::spawn(async move {
                    let _message_id = api_message.message_id;
                });
            }
            "SNDKYLST" => {
                tokio::spawn(async move {
                    let _message_id = api_message.message_id;
                });
            }
            _ => {
                println!(
                    "Unhandled command received by client... {}",
                    &api_message.message_name_as_string().as_str()
                );
            }
        }
    }
    async fn handle_peer_command_response(
        &mut self,
        command_name: [u8; 8],
        peer_id: SaitoHash,
        response_api_message: &APIMessage,
    ) {
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKINIT" => {
                // SHAKINIT response
                // We should sign the response and send a SHAKCOMP.
                // We want to reuse socket_handshake_verify, so we will sign first before
                // verifying the peer's signature
                let privatekey: SaitoPrivateKey;
                {
                    let wallet = self.base_peer.wallet_lock.read().await;
                    privatekey = wallet.get_private_key();
                }
                let signed_challenge =
                    sign_blob(&mut response_api_message.message_data.to_vec(), privatekey)
                        .to_owned();
                match socket_handshake_verify(&signed_challenge) {
                    Some(deserialize_challenge) => {
                        // TODO make this a peer method and remove the first argument
                        socket_handshake_complete(
                            self.base_peer.peers_db_lock.clone(),
                            peer_id,
                            deserialize_challenge.challenger_pubkey(),
                        )
                        .await;
                        let mut peers_db = self.base_peer.peers_db_lock.write().await;
                        let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
                        peer.send_command(&String::from("SHAKCOMP"), signed_challenge)
                            .await;
                        println!("SHAKECOMPLETE!");
                    }
                    None => {
                        println!("Error verifying peer handshake signature");
                    }
                }
            }
            "SHAKCOMP" => {
                println!(
                    "GOT SHAKCOMP RESPONSE {}",
                    String::from_utf8_lossy(&response_api_message.message_data()).to_string()
                );
            }
            "REQBLOCK" => {
                println!("GOT REQBLOCK RESPONSE");
            }
            "REQBLKHD" => {
                println!("GOT REQBLKHD RESPONSE");
            }
            "REQCHAIN" => {
                println!("GOT REQCHAIN RESPONSE");
            }
            "SNDTRANS" => {
                println!("GOT SNDTRANS RESPONSE");
            }
            "SNDCHAIN" => {
                println!("GOT SNDCHAIN RESPONSE");
            }
            "SNDBLKHD" => {
                println!("GOT SNDBLKHD RESPONSE");
            }
            "SNDKYLST" => {
                println!("GOT SNDKYLST RESPONSE");
            }
            _ => {}
        }
    }
    fn handle_peer_command_error_response(
        &mut self,
        command_name: [u8; 8],
        _response_api_message: &APIMessage,
    ) {
        println!(
            "Error response for command {}",
            String::from_utf8_lossy(&command_name).to_string()
        );
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKCOMP" => {
                // TODO delete the peer if there is an error here
            }
            _ => {}
        }
    }
    fn set_has_completed_handshake(&mut self, has_completed_handshake: bool) {
        self.base_peer.has_completed_handshake = has_completed_handshake;
    }
    fn get_has_completed_handshake(&self) -> bool {
        self.base_peer.has_completed_handshake
    }
    fn set_public_key(&mut self, public_key: SaitoPublicKey) {
        self.base_peer.public_key = Some(public_key)
    }
    fn get_public_key(&self) -> Option<SaitoPublicKey> {
        self.base_peer.public_key
    }
}
impl InboundPeer {
    pub fn new(
        peers_db_lock: Arc<RwLock<PeersDB>>,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> Self {
        InboundPeer {
            base_peer: BasePeer {
                has_completed_handshake: false,
                public_key: None,
                requests: HashMap::new(),
                request_count: 0,
                peers_db_lock,
                wallet_lock,
                mempool_lock,
                blockchain_lock,
            },
            sender: None,
        }
    }
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
        // TODO implement me
        self.base_peer.requests.insert(
            self.base_peer.request_count,
            String::from(command).as_bytes().try_into().unwrap(),
        );
        self.base_peer.request_count += 1;
        self.send(command, self.base_peer.request_count - 1, message)
            .await;
    }
    async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("RESULT__"), message_id, message)
            .await;
    }
    async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("ERROR___"), message_id, message)
            .await;
    }
    async fn handle_peer_command(&mut self, _api_message: &APIMessage, _peer_id: SaitoHash) {
        //TODO copy me from InboundPeer
    }
    fn set_has_completed_handshake(&mut self, _has_completed_handshake: bool) {
        // TODO implement me!!
    }
    fn get_has_completed_handshake(&self) -> bool {
        // TODO implement me!!
        true
    }
    fn set_public_key(&mut self, _public_key: SaitoPublicKey) {
        // self.public_key = public_key
    }
    fn get_public_key(&self) -> Option<SaitoPublicKey> {
        None
    }
    async fn handle_peer_command_response(
        &mut self,
        _command_name: [u8; 8],
        _peer_id: SaitoHash,
        _response_api_message: &APIMessage,
    ) {
    }
    fn handle_peer_command_error_response(
        &mut self,
        command_name: [u8; 8],
        _response_api_message: &APIMessage,
    ) {
    }
}

impl OutboundPeer {
    pub async fn new(
        peer_url: &str,
        peers_db_lock: Arc<RwLock<PeersDB>>,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> Self {
        let url = url::Url::parse(&peer_url).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (write_sink, read_stream) = ws_stream.split();

        let peer_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
        let mut peer = OutboundPeer {
            base_peer: BasePeer {
                has_completed_handshake: false,
                public_key: None,
                requests: HashMap::new(),
                request_count: 0,
                peers_db_lock,
                wallet_lock,
                mempool_lock,
                blockchain_lock,
            },
            read_stream: read_stream,
            write_sink: write_sink,
        };

        let publickey: SaitoPublicKey;
        {
            let wallet = peer.base_peer.wallet_lock.read().await;
            publickey = wallet.get_public_key();
        }

        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );

        println!("SENDING SHHAKEINIT");
        peer.send_command(&String::from("SHAKINIT"), message_data)
            .await;

        while let Some(result) = peer.read_stream.next().await {
            println!("NEW MESSAGE");
            let api_message = APIMessage::deserialize(&result.unwrap().into_data());
            peer.handle_peer_command(&api_message, peer_id).await;
        }

        peer
    }
}

pub async fn handle_inbound_peer_connection(
    ws: WebSocket,
    peers_db_lock: Arc<RwLock<PeersDB>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) {
    let mut peer = InboundPeer::new(
        peers_db_lock.clone(),
        wallet_lock.clone(),
        mempool_lock.clone(),
        blockchain_lock.clone(),
    );
    let (peer_ws_sender, mut peer_ws_rcv) = ws.split();
    let (peer_sender, peer_rcv) = mpsc::unbounded_channel();
    let peer_rcv = UnboundedReceiverStream::new(peer_rcv);
    tokio::task::spawn(peer_rcv.forward(peer_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    println!("peer channel created");
    peer.sender = Some(peer_sender);
    println!("peer sender set");

    let peer_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
    peers_db_lock
        .write()
        .await
        .insert(peer_id.clone(), Box::new(peer));

    println!("{:?} connected", peer_id);

    while let Some(result) = peer_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!(
                    "error receiving ws message for id: {:?}): {}",
                    peer_id.clone(),
                    e
                );
                break;
            }
        };
        println!("RECEIVING MESSAGE IN PEER CONNECTION");
        let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
        let mut peers_db = peers_db_lock.write().await;
        let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
        peer.handle_peer_command(&api_message, peer_id).await;
    }
    peers_db_lock.write().await.remove(&peer_id);
    println!("{:?} disconnected", peer_id);
}

pub async fn new_handshake_challenge(
    message: &APIMessage,
    wallet_lock: Arc<RwLock<Wallet>>,
) -> crate::Result<Vec<u8>> {
    let wallet = wallet_lock.read().await;
    let my_pubkey = wallet.get_public_key();
    let my_privkey = wallet.get_private_key();

    let mut peer_octets: [u8; 4] = [0; 4];
    peer_octets[0..4].clone_from_slice(&message.message_data[0..4]);
    let peer_pubkey: SaitoPublicKey = message.message_data[4..37].try_into().unwrap();

    // TODO configure the node's IP somewhere...
    let my_octets: [u8; 4] = [127, 0, 0, 1];

    // TODO get the IP of this socket connection somehow and validate it..
    // let peer_octets: [u8; 4] = match addr.unwrap().ip() {
    //     IpAddr::V4(ip4) => ip4.octets(),
    //     _ => panic!("Saito Handshake does not support IPV6"),
    // };

    let challenge = HandshakeChallenge::new((my_octets, my_pubkey), (peer_octets, peer_pubkey));
    let serialized_challenge = challenge.serialize_with_sig(my_privkey);

    Ok(serialized_challenge)
}

pub fn socket_handshake_verify(message_data: &Vec<u8>) -> Option<HandshakeChallenge> {
    let challenge = HandshakeChallenge::deserialize(message_data);
    if challenge.timestamp() < create_timestamp() - CHALLENGE_EXPIRATION_TIME {
        println!("Error validating timestamp for handshake complete");
        return None;
    }
    // we verify both signatures even though one is "ours". This function is called during both
    // "SHAKCOMP" and the "RESULT__" on the other end, so we just verify everything.

    if !verify(
        &hash(&message_data[..CHALLENGE_SIZE + 64].to_vec()),
        challenge.opponent_sig().unwrap(),
        challenge.opponent_pubkey(),
    ) {
        println!("Error with validating opponent sig");
        return None;
    }
    if !verify(
        &hash(&message_data[..CHALLENGE_SIZE].to_vec()),
        challenge.challenger_sig().unwrap(),
        challenge.challenger_pubkey(),
    ) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        println!("Error with validating challenger sig");
        return None;
    }

    Some(challenge)
}
// adds peer to peer DB
pub async fn socket_handshake_complete(
    peers_db_lock: Arc<RwLock<PeersDB>>,
    peer_id: SaitoHash,
    peer_public_key: SaitoPublicKey,
) {
    println!("HANDSHAKE COMPLETE, setting to completed");
    let mut peers_db = peers_db_lock.write().await;
    let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
    peer.set_has_completed_handshake(true);
    peer.set_public_key(peer_public_key);
}

pub fn socket_receive_transaction(message: APIMessage) -> Option<Transaction> {
    let tx = Transaction::deserialize_from_net(message.message_data);
    Some(tx)
}

pub async fn socket_req_block(
    api_message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Option<Vec<u8>> {
    let block_hash: SaitoHash = api_message.message_data[0..32].try_into().unwrap();
    let blockchain = blockchain_lock.read().await;

    match blockchain.get_block_sync(&block_hash) {
        Some(target_block) => Some(target_block.serialize_for_net()),
        None => None,
    }
}

pub async fn socket_send_block_header(
    api_message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Option<Vec<u8>> {
    let block_hash: SaitoHash = api_message.message_data[0..32].try_into().unwrap();
    let blockchain = blockchain_lock.read().await;

    match blockchain.get_block_sync(&block_hash) {
        Some(target_block) => {
            let block_header = target_block.get_header();
            Some(block_header.serialize_for_net())
        }
        None => None,
    }
}

pub async fn socket_send_blockchain(
    message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Option<Vec<u8>> {
    let block_hash: SaitoHash = message.message_data[0..32].try_into().unwrap();
    let _fork_id: SaitoHash = message.message_data[32..64].try_into().unwrap();

    let mut hashes: Vec<u8> = vec![];
    let blockchain = blockchain_lock.read().await;

    if let Some(target_block) = blockchain.get_latest_block() {
        let target_block_hash = target_block.get_hash();
        if target_block_hash != block_hash {
            hashes.extend_from_slice(&target_block_hash);
            let mut previous_block_hash = target_block.get_previous_block_hash();
            while !blockchain.get_block_sync(&previous_block_hash).is_none()
                && previous_block_hash != block_hash
            {
                if let Some(block) = blockchain.get_block_sync(&previous_block_hash) {
                    hashes.extend_from_slice(&block.get_hash());
                    previous_block_hash = block.get_previous_block_hash();
                }
            }
        }
        Some(hashes)
    } else {
        None
    }
}

// async fn handle_peer_command(base_peer: &BasePeer, api_message: &APIMessage, peer_id: SaitoHash) {
//     match api_message.message_name_as_string().as_str() {
//         "RESULT__" => {
//             let command_sent = base_peer.requests.remove(&api_message.message_id);
//             match command_sent {
//                 Some(command_name) => {
//                     handle_peer_command_response(command_name, peer_id, api_message)
//                     .await;
//                 }
//                 None => {
//                     println!(
//                         "Peer has sent a {} for a request we don't know about. Request ID: {}",
//                         api_message.message_name_as_string(),
//                         api_message.message_id
//                     );
//                 }
//             }
//         }
//         "ERROR___" => {
//             let command_sent = base_peer.requests.remove(&api_message.message_id);
//             match command_sent {
//                 Some(command_name) => {
//                     handle_peer_command_error_response(command_name, api_message);
//                 }
//                 None => {
//                     println!(
//                         "Peer has sent a {} for a request we don't know about. Request ID: {}",
//                         api_message.message_name_as_string(),
//                         api_message.message_id
//                     );
//                 }
//             }
//         }
//         "SHAKINIT" => {
//             tokio::spawn(async move {
//                 if let Ok(serialized_handshake_challenge) =
//                     new_handshake_challenge(&api_message, base_peer.wallet_lock.clone()).await
//                 {
//                     let mut peers_db = base_peer.peers_db_lock.write().await;
//                     let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                     peer.send_response(api_message.message_id, serialized_handshake_challenge)
//                         .await;
//                 }
//             });
//         }
//         "SHAKCOMP" => {
//             tokio::spawn(async move {
//                 match socket_handshake_verify(&api_message.message_data()) {
//                     Some(deserialize_challenge) => {

//                         socket_handshake_complete(base_peer.peers_db_lock.clone(), peer_id, deserialize_challenge.opponent_pubkey()).await;
//                         let mut peers_db = base_peer.peers_db_lock.write().await;
//                         let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                         peer.send_response(api_message.message_id, String::from("OK").as_bytes().try_into().unwrap()).await;
//                     }
//                     None => {
//                         println!("Error verifying peer handshake signature");
//                     }
//                 }
//             });
//         }
//         "REQBLOCK" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;

//                 let mut peers_db = base_peer.peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_req_block(api_message, base_peer.blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLOCK"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }
//             });
//         }
//         "REQBLKHD" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;
//                 let mut peers_db = base_peer.peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_send_block_header(api_message, base_peer.blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLKHD"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }

//             });
//         }
//         "REQCHAIN" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;

//                 let mut peers_db = base_peer.peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_send_blockchain(api_message, base_peer.blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLKHD"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }
//             });
//         }
//         "SNDTRANS" => {
//             tokio::spawn(async move {
//                 if let Some(tx) = socket_receive_transaction(api_message.clone()) {
//                     let mut mempool = base_peer.mempool_lock.write().await;
//                     let mut peers_db = base_peer.peers_db_lock.write().await;
//                     let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                     match mempool.add_transaction(tx).await {
//                         AddTransactionResult::Accepted | AddTransactionResult::Exists => {
//                             // TODO The tx is accepted, propagate it to all available peers
//                             peer.send_response(
//                                 api_message.message_id,
//                                 String::from("OK").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                         AddTransactionResult::Invalid => {
//                             peer.send_error_response(
//                                 api_message.message_id,
//                                 String::from("Invalid").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                         AddTransactionResult::Rejected => {
//                             peer.send_error_response(
//                                 api_message.message_id,
//                                 String::from("Invalid").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                     }
//                 }
//             });
//         }
//         "SNDCHAIN" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }
//         "SNDBLKHD" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }
//         "SNDKYLST" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }

//         _ => {
//             // If anything other than RESULT__ or ERROR___ is sent here, there is a problem.
//             // The "client" side of the connection sends APIMessage and the "network/server" side responds.
//             // We should not handle anything other than RESULT__ and ERROR___ here.
//             println!(
//                 "Unhandled command received by client... {}",
//                 &api_message.message_name_as_string().as_str()
//             );
//         }
//     }
// }

// async fn handle_peer_command_response(
//     command_name: [u8; 8],
//     peer_id: SaitoHash,
//     response_api_message: &APIMessage,
// ) {
//     match String::from_utf8_lossy(&command_name).to_string().as_ref() {
//         "SHAKINIT" => {
//             // SHAKINIT response
//             // We should sign the response and send a SHAKCOMP.
//             // We want to reuse socket_handshake_verify, so we will sign first before
//             // verifying the peer's signature
//             let privatekey: SaitoPrivateKey;
//             {
//                 let wallet = base_peer.wallet_lock.read().await;
//                 privatekey = wallet.get_private_key();
//             }
//             let signed_challenge =
//                 sign_blob(&mut response_api_message.message_data.to_vec(), privatekey)
//                     .to_owned();
//             match socket_handshake_verify(&signed_challenge) {
//                 Some(deserialize_challenge) => {
//                     // TODO make this a peer method and remove the first argument
//                     socket_handshake_complete(base_peer.peers_db_lock.clone(), peer_id, deserialize_challenge.challenger_pubkey()).await;
//                     let mut peers_db = base_peer.peers_db_lock.write().await;
//                     let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                     peer.send_command(&String::from("SHAKCOMP"), signed_challenge).await;
//                 }
//                 None => {
//                     println!("Error verifying peer handshake signature");
//                 }
//             }
//         }
//         "SHAKCOMP" => {
//             println!("GOT SHAKCOMP RESPONSE");
//             println!(
//                 "{}",
//                 String::from_utf8_lossy(&response_api_message.message_data()).to_string()
//             );
//         }
//         "REQBLOCK" => {
//             println!("GOT REQBLOCK RESPONSE");
//         }
//         "REQBLKHD" => {
//             println!("GOT REQBLKHD RESPONSE");
//         }
//         "REQCHAIN" => {
//             println!("GOT REQCHAIN RESPONSE");
//         }
//         "SNDTRANS" => {
//             println!("GOT SNDTRANS RESPONSE");
//         }
//         "SNDCHAIN" => {
//             println!("GOT SNDCHAIN RESPONSE");
//         }
//         "SNDBLKHD" => {
//             println!("GOT SNDBLKHD RESPONSE");
//         }
//         "SNDKYLST" => {
//             println!("GOT SNDKYLST RESPONSE");
//         }
//         _ => {}
//     }
// }
// fn handle_peer_command_error_response(
//     command_name: [u8; 8],
//     _response_api_message: &APIMessage,
// ) {
//     println!(
//         "Error response for command {}",
//         String::from_utf8_lossy(&command_name).to_string()
//     );
// }
// async fn handle_peer_command(
//     peer_id: SaitoHash,
//     msg: Message,
//     peers_db_lock: Arc<RwLock<PeersDB>>,
//     wallet_lock: Arc<RwLock<Wallet>>,
//     mempool_lock: Arc<RwLock<Mempool>>,
//     blockchain_lock: Arc<RwLock<Blockchain>>,
// ) {
//     let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
//     let command = String::from_utf8_lossy(api_message.message_name());

//     match &command.to_string()[..] {
//         "SHAKINIT" => {
//             tokio::spawn(async move {
//                 if let Ok(serialized_handshake_challenge) =
//                     new_handshake_challenge(&api_message, wallet_lock).await
//                 {
//                     let mut peers_db = peers_db_lock.write().await;
//                     let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                     peer.send_response(api_message.message_id, serialized_handshake_challenge)
//                         .await;
//                 }
//             });
//         }
//         "SHAKCOMP" => {
//             tokio::spawn(async move {
//                 match socket_handshake_verify(&api_message.message_data()) {
//                     Some(deserialize_challenge) => {

//                         socket_handshake_complete(peers_db_lock.clone(), peer_id, deserialize_challenge.opponent_pubkey()).await;
//                         let mut peers_db = peers_db_lock.write().await;
//                         let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                         peer.send_response(api_message.message_id, String::from("OK").as_bytes().try_into().unwrap()).await;
//                     }
//                     None => {
//                         println!("Error verifying peer handshake signature");
//                     }
//                 }
//             });
//         }
//         "REQBLOCK" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;

//                 let mut peers_db = peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_req_block(api_message, blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLOCK"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }
//             });
//         }
//         "REQBLKHD" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;
//                 let mut peers_db = peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_send_block_header(api_message, blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLKHD"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }

//             });
//         }
//         "REQCHAIN" => {
//             tokio::spawn(async move {
//                 let message_id = api_message.message_id;

//                 let mut peers_db = peers_db_lock.write().await;
//                 let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();

//                 if let Some(bytes) = socket_send_blockchain(api_message, blockchain_lock).await {
//                     let message_data = String::from("OK").as_bytes().try_into().unwrap();
//                     peer.send_response(message_id, message_data).await;
//                     peer.send_command(&String::from("SNDBLKHD"), bytes).await;
//                 } else {
//                     let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
//                     peer.send_error_response(message_id, message_data).await;
//                 }
//             });
//         }
//         "SNDTRANS" => {
//             tokio::spawn(async move {
//                 if let Some(tx) = socket_receive_transaction(api_message.clone()) {
//                     let mut mempool = mempool_lock.write().await;
//                     let mut peers_db = peers_db_lock.write().await;
//                     let peer = peers_db.get_mut(&peer_id).unwrap().as_mut();
//                     match mempool.add_transaction(tx).await {
//                         AddTransactionResult::Accepted | AddTransactionResult::Exists => {
//                             // TODO The tx is accepted, propagate it to all available peers
//                             peer.send_response(
//                                 api_message.message_id,
//                                 String::from("OK").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                         AddTransactionResult::Invalid => {
//                             peer.send_error_response(
//                                 api_message.message_id,
//                                 String::from("Invalid").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                         AddTransactionResult::Rejected => {
//                             peer.send_error_response(
//                                 api_message.message_id,
//                                 String::from("Invalid").as_bytes().try_into().unwrap(),
//                             )
//                             .await;
//                         }
//                     }
//                 }
//             });
//         }
//         "SNDCHAIN" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }
//         "SNDBLKHD" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }
//         "SNDKYLST" => {
//             tokio::spawn(async move {
//                 let _message_id = api_message.message_id;
//             });
//         }
//         _ => {}
//     }
// }
