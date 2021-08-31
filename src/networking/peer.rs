use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::crypto::{hash, sign_blob, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::{AddTransactionResult, Mempool};
use crate::networking::message_types::handshake_challenge::HandshakeChallenge;
use crate::networking::message_types::request_blockchain_message::RequestBlockchainMessage;
use crate::networking::message_types::send_blockchain_message::{
    SendBlockchainBlockData, SendBlockchainMessage, SyncType,
};
use crate::networking::network::CHALLENGE_SIZE;
use crate::time::create_timestamp;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use futures::stream::SplitSink;
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
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};
use tracing::{event, Level};

pub type PeersDB = HashMap<SaitoHash, BasePeer>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}

pub struct BasePeer {
    has_completed_handshake: bool,
    publickey: Option<SaitoPublicKey>,
    requests: HashMap<u32, [u8; 8]>,
    request_count: u32,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    inbound_peer: Option<InboundPeer>,
    outbound_peer: Option<OutboundPeer>,
}

pub struct InboundPeer {
    sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>,
}
pub struct OutboundPeer {
    write_sink:
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::protocol::Message>,
}

impl BasePeer {
    async fn send(&mut self, command: &String, message_id: u32, message: Vec<u8>) {
        let api_message = APIMessage::new(command, message_id, message);
        if self.inbound_peer.is_some() && self.outbound_peer.is_some() {
            panic!("Peer has both inbound and outbound connections, this should never happen");
        }
        if self.outbound_peer.is_some() {
            let _foo = self
                .outbound_peer
                .as_mut()
                .unwrap()
                .write_sink
                .send(api_message.serialize().into())
                .await;
        }
        if self.inbound_peer.is_some() {
            let _foo = self
                .inbound_peer
                .as_mut()
                .unwrap()
                .sender
                .send(Ok(Message::binary(api_message.serialize())));
        }
    }
    pub async fn send_command(&mut self, command: &String, message: Vec<u8>) {
        self.requests.insert(
            self.request_count,
            String::from(command).as_bytes().try_into().unwrap(),
        );
        self.request_count += 1;
        self.send(command, self.request_count - 1, message).await;
    }
    pub async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("RESULT__"), message_id, message)
            .await;
    }
    pub async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(&String::from("ERROR___"), message_id, message)
            .await;
    }
    pub async fn handle_peer_command(&mut self, api_message_orig: &APIMessage) {
        let api_message = api_message_orig.clone();
        let mempool_lock = self.mempool_lock.clone();
        let blockchain_lock = self.blockchain_lock.clone();
        match api_message_orig.message_name_as_string().as_str() {
            "RESULT__" => {
                let command_sent = self.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_response_command(command_name, &api_message)
                            .await;
                    }
                    None => {
                        event!(
                            Level::ERROR,
                            "Peer has sent a {} for a request we don't know about. Request ID: {}",
                            api_message.message_name_as_string(),
                            api_message.message_id
                        );
                    }
                }
            }
            "ERROR___" => {
                let command_sent = self.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_response_error(command_name, &api_message);
                    }
                    None => {
                        event!(
                            Level::ERROR,
                            "Peer has sent an error {} . Request ID: {}",
                            api_message.message_name_as_string(),
                            api_message.message_id
                        );
                    }
                }
            }
            "SHAKINIT" => {
                if let Ok(serialized_handshake_challenge) =
                    new_handshake_challenge(&api_message, self.wallet_lock.clone()).await
                {
                    self.send_response(api_message.message_id, serialized_handshake_challenge)
                        .await;
                }
            }
            "SHAKCOMP" => {
                match socket_handshake_verify(&api_message.message_data()) {
                    Some(deserialize_challenge) => {
                        self.set_has_completed_handshake(true);
                        self.set_publickey(deserialize_challenge.opponent_pubkey());
                        self.send_response(
                            api_message.message_id,
                            String::from("OK").as_bytes().try_into().unwrap(),
                        )
                        .await;
                    }
                    None => {
                        event!(Level::ERROR, "Error verifying peer handshake signature");
                    }
                }
            }
            "REQBLOCK" => {
                let message_id = api_message.message_id;
                if let Some(bytes) = socket_req_block(&api_message, blockchain_lock).await {
                    let message_data = String::from("OK").as_bytes().try_into().unwrap();
                    self.send_response(message_id, message_data).await;
                    self.send_command(&String::from("SNDBLOCK"), bytes).await;
                } else {
                    let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
                    self.send_error_response(message_id, message_data).await;
                }
            }
            "REQBLKHD" => {
                let message_id = api_message.message_id;
                if let Some(bytes) = socket_send_block_header(&api_message, blockchain_lock).await {
                    let message_data = String::from("OK").as_bytes().try_into().unwrap();
                    self.send_response(message_id, message_data).await;
                    self.send_command(&String::from("SNDBLKHD"), bytes).await;
                } else {
                    let message_data = String::from("ERROR").as_bytes().try_into().unwrap();
                    self.send_error_response(message_id, message_data).await;
                }
            }
            "REQCHAIN" => {
                self.send_response(
                    api_message.message_id,
                    String::from("OK").as_bytes().try_into().unwrap(),
                )
                .await;
                let deserialized_request_blockchain_message =
                    RequestBlockchainMessage::deserialize(&api_message.message_data());
                if let Some(send_blockchain_message) = build_send_blockchain_message(
                    &deserialized_request_blockchain_message,
                    blockchain_lock,
                )
                .await
                {
                    self.send_command(
                        &String::from("SNDCHAIN"),
                        send_blockchain_message.serialize(),
                    )
                    .await;
                } else {
                    let message_data = String::from("UNKNOWN BLOCK HASH")
                        .as_bytes()
                        .try_into()
                        .unwrap();
                    self.send_error_response(api_message.message_id, message_data)
                        .await;
                }
            }
            "SNDTRANS" => {
                if let Some(tx) = socket_receive_transaction(api_message.clone()) {
                    let mut mempool = mempool_lock.write().await;
                    match mempool.add_transaction(tx).await {
                        AddTransactionResult::Accepted | AddTransactionResult::Exists => {
                            // TODO The tx is accepted, propagate it to all available peers
                            self.send_response(
                                api_message.message_id,
                                String::from("OK").as_bytes().try_into().unwrap(),
                            )
                            .await;
                        }
                        AddTransactionResult::Invalid => {
                            self.send_error_response(
                                api_message.message_id,
                                String::from("Invalid").as_bytes().try_into().unwrap(),
                            )
                            .await;
                        }
                        AddTransactionResult::Rejected => {
                            self.send_error_response(
                                api_message.message_id,
                                String::from("Invalid").as_bytes().try_into().unwrap(),
                            )
                            .await;
                        }
                    }
                }
            }
            "SNDCHAIN" => {
                let _message_id = api_message.message_id;
                // break the msg into discrete blocks to fetch
                let hashes: Vec<&[u8]> = api_message.message_data.chunks(32).collect();
                for hash in hashes.into_iter() {
                    self.send_command(&String::from("REQBLOCK"), hash.to_vec())
                        .await;
                }
            }
            "SNDBLOCK" => {
                // receive the block fetched and add it to the blockchain
                let mut block = Block::deserialize_for_net(api_message.message_data);
                block.generate_hashes();
                {
                    let mut blockchain = self.blockchain_lock.write().await;
                    blockchain.add_block(block, true).await;
                }
            }
            "SNDBLKHD" => {
                let _message_id = api_message.message_id;
            }
            "SNDKYLST" => {
                let _message_id = api_message.message_id;
            }
            _ => {
                event!(
                    Level::ERROR,
                    "Unhandled command received by client... {}",
                    &api_message.message_name_as_string().as_str()
                );
            }
        }
    }
    async fn handle_peer_response_command(
        &mut self,
        command_name: [u8; 8],
        response_api_message: &APIMessage,
    ) {
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKINIT" => {
                // We should sign the response and send a SHAKCOMP.
                // We want to reuse socket_handshake_verify, so we will sign first before
                // verifying the peer's signature
                let privatekey: SaitoPrivateKey;
                {
                    let wallet = self.wallet_lock.read().await;
                    privatekey = wallet.get_privatekey();
                }
                let signed_challenge =
                    sign_blob(&mut response_api_message.message_data.to_vec(), privatekey)
                        .to_owned();
                match socket_handshake_verify(&signed_challenge) {
                    Some(deserialize_challenge) => {
                        self.set_has_completed_handshake(true);
                        self.set_publickey(deserialize_challenge.challenger_pubkey());
                        self.send_command(&String::from("SHAKCOMP"), signed_challenge)
                            .await;
                    }
                    None => {
                        event!(Level::ERROR, "Error verifying peer handshake signature");
                    }
                }
            }
            "SHAKCOMP" => {
                event!(
                    Level::TRACE,
                    "GOT SHAKCOMP RESPONSE {}",
                    String::from_utf8_lossy(&response_api_message.message_data()).to_string()
                );
                // now that we're connected, we want to request the chain from our new peer and see if we're synced
                // TODO get this data from blockchain.rs or wherever it is...
                let request_blockchain_message =
                    RequestBlockchainMessage::new(0, [0; 32], [42; 32]);

                self.send_command(
                    &String::from("REQCHAIN"),
                    request_blockchain_message.serialize(),
                )
                .await;
            }
            "REQBLOCK" => {
                println!("GOT REQBLOCK RESPONSE");
            }
            "REQBLKHD" => {
                println!("GOT REQBLKHD RESPONSE");
            }
            "REQCHAIN" => {
                println!(
                    "GOT REQCHAIN RESPONSE {}",
                    String::from_utf8_lossy(&response_api_message.message_data()).to_string()
                );
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
    fn handle_peer_response_error(
        &mut self,
        command_name: [u8; 8],
        _response_api_message: &APIMessage,
    ) {
        event!(
            Level::ERROR,
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
    pub fn set_has_completed_handshake(&mut self, has_completed_handshake: bool) {
        self.has_completed_handshake = has_completed_handshake;
    }
    pub fn get_has_completed_handshake(&self) -> bool {
        self.has_completed_handshake
    }
    pub fn set_publickey(&mut self, publickey: SaitoPublicKey) {
        self.publickey = Some(publickey)
    }
    pub fn get_publickey(&self) -> Option<SaitoPublicKey> {
        self.publickey
    }
}
impl InboundPeer {
    pub fn new(
        sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> BasePeer {
        BasePeer {
            has_completed_handshake: false,
            publickey: None,
            requests: HashMap::new(),
            request_count: 0,
            wallet_lock,
            mempool_lock,
            blockchain_lock,
            inbound_peer: Some(InboundPeer { sender: sender }),
            outbound_peer: None,
        }
    }
}

impl OutboundPeer {
    pub async fn new(
        write_sink: SplitSink<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
            tokio_tungstenite::tungstenite::Message,
        >,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> BasePeer {
        let peer = BasePeer {
            has_completed_handshake: false,
            publickey: None,
            requests: HashMap::new(),
            request_count: 0,
            wallet_lock,
            mempool_lock,
            blockchain_lock,
            inbound_peer: None,
            outbound_peer: Some(OutboundPeer {
                write_sink: write_sink,
            }),
        };

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
    let (peer_ws_sender, mut peer_ws_rcv) = ws.split();
    let (peer_sender, peer_rcv) = mpsc::unbounded_channel();
    let peer_rcv = UnboundedReceiverStream::new(peer_rcv);
    tokio::task::spawn(peer_rcv.forward(peer_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    let peer = InboundPeer::new(
        peer_sender,
        wallet_lock.clone(),
        mempool_lock.clone(),
        blockchain_lock.clone(),
    );

    let peer_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
    peers_db_lock.write().await.insert(peer_id.clone(), peer);

    event!(Level::TRACE, "{:?} connected", peer_id);

    tokio::task::spawn(async move {
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
            let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
            let mut peers_db = peers_db_lock.write().await;
            let peer = peers_db.get_mut(&peer_id).unwrap();
            peer.handle_peer_command(&api_message).await;
        }
        peers_db_lock.write().await.remove(&peer_id);
        event!(Level::TRACE, "{:?} disconnected", peer_id);
    });
}

pub async fn new_handshake_challenge(
    message: &APIMessage,
    wallet_lock: Arc<RwLock<Wallet>>,
) -> crate::Result<Vec<u8>> {
    let wallet = wallet_lock.read().await;
    let my_pubkey = wallet.get_publickey();
    let my_privkey = wallet.get_privatekey();

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
        event!(Level::ERROR, "Error validating timestamp for handshake complete");
        return None;
    }
    // we verify both signatures even though one is "ours". This function is called during both
    // "SHAKCOMP" and the "RESULT__" on the other end, so we just verify everything.

    if !verify(
        &hash(&message_data[..CHALLENGE_SIZE + 64].to_vec()),
        challenge.opponent_sig().unwrap(),
        challenge.opponent_pubkey(),
    ) {
        event!(Level::ERROR, "Error with validating opponent sig");
        return None;
    }
    if !verify(
        &hash(&message_data[..CHALLENGE_SIZE].to_vec()),
        challenge.challenger_sig().unwrap(),
        challenge.challenger_pubkey(),
    ) {
        // TODO figure out how to return more meaningful errors from Warp and replace all the warp::reject
        event!(Level::ERROR, "Error with validating challenger sig");
        return None;
    }

    Some(challenge)
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

pub async fn build_send_blockchain_message(
    request_blockchain_message: &RequestBlockchainMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Option<SendBlockchainMessage> {
    let block_zero_hash: SaitoHash = [0; 32];

    let peers_latest_hash: &SaitoHash;
    if request_blockchain_message.get_latest_block_id() == 0
        && request_blockchain_message.get_latest_block_hash() == &block_zero_hash
    {
        event!(Level::TRACE, "GOT BLOCK ZERO HASH");
        peers_latest_hash = &block_zero_hash;
    } else {
        // TODO contains_block_hash_at_block_id for some reason needs mutable access to block_ring
        // We should figure out why this is and get rid of this problem, I don't think there's any
        // reason we should need to get the write lock for this operation...
        let blockchain = blockchain_lock.read().await;
        if !blockchain.contains_block_hash_at_block_id(
            request_blockchain_message.get_latest_block_id(),
            *request_blockchain_message.get_latest_block_hash(),
        ) {
            // The latest hash from our peer is not in the longest chain
            return None;
        }
        peers_latest_hash = request_blockchain_message.get_latest_block_hash();
    }

    event!(
        Level::TRACE,
        "GET BLOCK FOR SNDCHAIN {:?}",
        &hex::encode(&peers_latest_hash)
    );

    let blockchain = blockchain_lock.read().await;

    let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
    if let Some(latest_block) = blockchain.get_latest_block() {
        let mut previous_block_hash: SaitoHash = latest_block.get_hash();
        let mut this_block: &Block; // = blockchain.get_block_sync(&previous_block_hash).unwrap();
        while &previous_block_hash != peers_latest_hash {
            event!(
                Level::TRACE,
                "GET BLOCK FOR SNDCHAIN {:?}",
                &hex::encode(&previous_block_hash)
            );
            this_block = blockchain.get_block_sync(&previous_block_hash).unwrap();
            blocks_data.push(SendBlockchainBlockData {
                block_id: this_block.get_id(),
                timestamp: this_block.get_timestamp(),
                pre_hash: [0; 32],
                number_of_transactions: 0,
            });
            previous_block_hash = this_block.get_previous_block_hash();
        }
        Some(SendBlockchainMessage::new(
            SyncType::Full,
            *peers_latest_hash,
            blocks_data,
        ))
    } else {
        panic!("Blockchain does not have any blocks");
    }
}
