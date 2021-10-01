use super::api_message::APIMessage;
use super::network::CHALLENGE_EXPIRATION_TIME;
use crate::block::{Block, BlockType};
use crate::blockchain::{Blockchain, GENESIS_PERIOD};
use crate::crypto::{hash, verify, SaitoHash, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::networking::message_types::handshake_challenge::HandshakeChallenge;
use crate::networking::message_types::request_block_message::RequestBlockMessage;
use crate::networking::message_types::request_blockchain_message::RequestBlockchainMessage;
use crate::networking::message_types::send_blockchain_message::{
    SendBlockchainBlockData, SendBlockchainMessage, SyncType,
};
use crate::networking::network::CHALLENGE_SIZE;
use crate::time::create_timestamp;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use async_recursion::async_recursion;
use futures::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

use futures::{Future, FutureExt, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};
use tracing::{event, Level};

pub type PeersDB = HashMap<SaitoHash, SaitoPeer>;
pub type RequestResponses = HashMap<(SaitoHash, u32), APIMessage>;
pub type RequestWakers = HashMap<(SaitoHash, u32), Waker>;
pub type OutboundPeersDB = HashMap<SaitoHash, OutboundPeer>;
pub type InboundPeersDB = HashMap<SaitoHash, InboundPeer>;

lazy_static::lazy_static! {
    pub static ref PEERS_DB_GLOBAL: Arc<tokio::sync::RwLock<PeersDB>> = Arc::new(tokio::sync::RwLock::new(PeersDB::new()));
    pub static ref PEERS_REQUEST_RESPONSES_GLOBAL: Arc<std::sync::RwLock<RequestResponses>> = Arc::new(std::sync::RwLock::new(RequestResponses::new()));
    pub static ref PEERS_REQUEST_WAKERS_GLOBAL: Arc<std::sync::RwLock<RequestWakers>> = Arc::new(std::sync::RwLock::new(RequestWakers::new()));
    pub static ref INBOUND_PEER_CONNECTIONS_GLOBAL: Arc<tokio::sync::RwLock<InboundPeersDB>> = Arc::new(tokio::sync::RwLock::new(InboundPeersDB::new()));
    pub static ref OUTBOUND_PEER_CONNECTIONS_GLOBAL: Arc<tokio::sync::RwLock<OutboundPeersDB>> = Arc::new(tokio::sync::RwLock::new(OutboundPeersDB::new()));
}

/// Settings for remote Peers
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}

/// Flags for Peer state.
pub struct PeerFlags {
    is_connected_or_connecting: bool,
    has_completed_handshake: bool,
    is_from_peer_list: bool,
}

/// A Peer. i.e. another node in the network.
pub struct SaitoPeer {
    peer_flags: PeerFlags,
    connection_id: SaitoHash,
    publickey: Option<SaitoPublicKey>,
    host: Option<[u8; 4]>,
    port: Option<u16>,
    request_count: u32,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
}

/// An inbound Peer. This holds a Stream(Sender/Sink) for a peer which has initialized a connection to us via /wsopen.
pub struct InboundPeer {
    pub sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>,
}

/// An outbound Peer. This holds a Stream(Sender/Sink) for a peer which we have connected to via /wsopen.
/// TODO: Unify InboundPeer and OutboundPeer into a single trait and perhaps integrate it into SaitoPeer.
///       This has proven to be a very difficult task because of ownership loops that make it very tricky
///       to interact with a peer by reading messages from a socket which is inside the peer. This is why
///       we have a separate structure to hold the sockets vs the rest of the peer data. It may be easy
///       to at least unify Inbound and Outbound into a single Trait though and merge InboundPeersDB and
///       OutboundPeersDB
pub struct OutboundPeer {
    pub write_sink:
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::protocol::Message>,
}

pub struct PeerRequest {
    connection_id: SaitoHash,
    request_id: u32,
    // This is here for debugging
    api_message_command: String,
}
/// A future which wraps an APIMessage REQUEST->RESPONSE into a Future(e.g. REQBLOCK->RESULT__).
/// This enables a much cleaner interface for inter-node message relays by allowing a response to
/// be returned directly from peer.send_command() and to turn an ERROR___ response into an Err()
/// Result.
impl PeerRequest {
    pub async fn new(command: &str, message: Vec<u8>, peer: &mut SaitoPeer) -> Self {
        peer.request_count += 1;
        let api_message = APIMessage::new(command, peer.request_count - 1, message);
        send_message_to_socket(api_message, &peer.connection_id).await;
        PeerRequest {
            connection_id: peer.connection_id,
            request_id: peer.request_count - 1,
            api_message_command: String::from(command),
        }
    }
}

impl Future for PeerRequest {
    type Output = Result<APIMessage, Box<dyn Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
        let mut request_responses = request_responses_lock.write().unwrap();
        match request_responses.remove(&(self.connection_id, self.request_id)) {
            Some(response) => {
                // Return from the Future with an important message!
                event!(
                    Level::INFO,
                    "HANDLING RESPONSE {}",
                    self.api_message_command
                );
                Poll::Ready(Ok(response))
            }
            None => {
                let request_wakers_lock = PEERS_REQUEST_WAKERS_GLOBAL.clone();
                let mut request_wakers = request_wakers_lock.write().unwrap();
                request_wakers.insert((self.connection_id, self.request_id), cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
/// Sends an APIMessage to a socket connection. Since Outbound and Inbound peers Streams(Sinks) are
/// not unified into a single Trait yet, we must check both dbs to find out which sort of sink this
/// peer is using and send the message through the appopriate stream.
async fn send_message_to_socket(api_message: APIMessage, connection_id: &SaitoHash) {
    let outbound_peer_connection_db_global = OUTBOUND_PEER_CONNECTIONS_GLOBAL.clone();
    let mut outbound_peer_connection_db = outbound_peer_connection_db_global.write().await;
    let inbound_peer_connection_db_global = INBOUND_PEER_CONNECTIONS_GLOBAL.clone();
    let mut inbound_peer_connection_db = inbound_peer_connection_db_global.write().await;

    let outbound_peer_connection = outbound_peer_connection_db.get_mut(connection_id);
    let inbound_peer_connection = inbound_peer_connection_db.get_mut(connection_id);

    if inbound_peer_connection.is_some() && outbound_peer_connection.is_some() {
        panic!("Peer has both inbound and outbound connections, this should never happen");
    }
    if inbound_peer_connection.is_none() && outbound_peer_connection.is_none() {
        panic!("Peer has neither outbound nor inbound connection");
    }
    if outbound_peer_connection.is_some() {
        let _foo = outbound_peer_connection
            .unwrap()
            .write_sink
            .send(api_message.serialize().into())
            .await;
    }
    if inbound_peer_connection.is_some() {
        let _foo = inbound_peer_connection
            .unwrap()
            .sender
            .send(Ok(Message::binary(api_message.serialize())));
    }
}

impl SaitoPeer {
    pub fn new(
        connection_id: SaitoHash,
        host: Option<[u8; 4]>,
        port: Option<u16>,
        is_connected_or_connecting: bool,
        has_completed_handshake: bool,
        is_from_peer_list: bool,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> SaitoPeer {
        SaitoPeer {
            peer_flags: PeerFlags {
                is_connected_or_connecting,
                has_completed_handshake,
                is_from_peer_list,
            },
            connection_id,
            host,
            port,
            publickey: None,
            request_count: 0,
            wallet_lock,
            mempool_lock,
            blockchain_lock,
        }
    }
    pub fn get_is_from_peer_list(&self) -> bool {
        self.peer_flags.is_from_peer_list
    }
    pub fn set_has_completed_handshake(&mut self, has_completed_handshake: bool) {
        self.peer_flags.has_completed_handshake = has_completed_handshake;
    }
    pub fn get_has_completed_handshake(&self) -> bool {
        self.peer_flags.has_completed_handshake
    }
    pub fn set_publickey(&mut self, publickey: SaitoPublicKey) {
        self.publickey = Some(publickey)
    }
    pub fn get_publickey(&self) -> Option<SaitoPublicKey> {
        self.publickey
    }
    /// this setter also will reset the appropriate flags on the peer if the peer becomes disconnected.
    /// Once disconnected, we want to also redo the handshake because this will help avoid IP-based
    /// connection stealing.
    pub async fn set_is_connected_or_connecting(&mut self, is_connected_or_connecting: bool) {
        // we need to clean out the connections from the connection DBs if we disconnected
        if !is_connected_or_connecting {
            let outbound_peer_connection_db_global = OUTBOUND_PEER_CONNECTIONS_GLOBAL.clone();
            let mut outbound_peer_connection_db = outbound_peer_connection_db_global.write().await;
            outbound_peer_connection_db.remove(&self.connection_id);
            let inbound_peer_connection_db_global = INBOUND_PEER_CONNECTIONS_GLOBAL.clone();
            let mut inbound_peer_connection_db = inbound_peer_connection_db_global.write().await;
            inbound_peer_connection_db.remove(&self.connection_id);
            // If we lose connection, we must also re-shake hands. Otherwise we risk IP-based handshake theft. This may be
            // a problem anyway with something like a CSFR, but we should at least make it as difficult as possible.
            self.peer_flags.has_completed_handshake = false;
        }
        // and set the flag
        self.peer_flags.is_connected_or_connecting = is_connected_or_connecting;
    }
    pub fn get_is_connected_or_connecting(&self) -> bool {
        self.peer_flags.is_connected_or_connecting
    }
    pub fn get_host(&self) -> Option<[u8; 4]> {
        self.host
    }
    pub fn get_port(&self) -> Option<u16> {
        self.port
    }
    pub fn get_connection_id(&self) -> SaitoHash {
        self.connection_id
    }
    /// This creates a PeerRequest Future(which sends the APIMessage to the peer), await's the response,
    /// and wraps the returned message into a Result(RESULT__ => Ok(), ERROR___ => Err()).
    #[async_recursion]
    pub async fn send_command(
        &mut self,
        command: &str,
        message: Vec<u8>,
    ) -> Result<APIMessage, APIMessage> {
        let peer_request = PeerRequest::new(command, message, self).await;
        // TODO should we turn this expect into an Err()???
        let response_message = peer_request
            .await
            .expect(&format!("Error returned from {}", command));
        match response_message.get_message_name_as_string().as_str() {
            "RESULT__" => Ok(response_message),
            "ERROR___" => Err(response_message),
            _ => {
                panic!("Received non-response response");
            }
        }
    }
    /// Helper function for sending basic OK results.
    pub async fn send_response_from_str(&mut self, message_id: u32, message_str: &str) {
        send_message_to_socket(
            APIMessage::new_from_string("RESULT__", message_id, message_str),
            &self.connection_id,
        )
        .await;
    }
    /// Helper function for sending RESULT__.
    pub async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        send_message_to_socket(
            APIMessage::new("RESULT__", message_id, message),
            &self.connection_id,
        )
        .await;
    }
    /// Helper function for sending basic errors with a string message.
    pub async fn send_error_response_from_str(&mut self, message_id: u32, message_str: &str) {
        send_message_to_socket(
            APIMessage::new_from_string("ERROR___", message_id, message_str),
            &self.connection_id,
        )
        .await;
    }
    /// Helper function for sending errors
    pub async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        send_message_to_socket(
            APIMessage::new("ERROR___", message_id, message),
            &self.connection_id,
        )
        .await;
    }
    /// handle any APIMessage from the socket. RESULT/ERROR will be matched to it's COMMAND via
    /// the ID and the Future will be polled via it's waker. Normal commands will be handled by handle_peer_command.
    pub async fn handle_peer_message(api_message_orig: APIMessage, connection_id: SaitoHash) {
        match api_message_orig.get_message_name_as_string().as_str() {
            "RESULT__" | "ERROR___" => {
                let request_wakers_lock = PEERS_REQUEST_WAKERS_GLOBAL.clone();
                let mut request_wakers = request_wakers_lock.write().unwrap();
                let option_waker =
                    request_wakers.remove(&(connection_id, api_message_orig.message_id));

                let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
                let mut request_responses = request_responses_lock.write().unwrap();
                request_responses.insert(
                    (connection_id, api_message_orig.message_id),
                    api_message_orig,
                );

                if let Some(waker) = option_waker {
                    waker.wake();
                }
            }
            _ => {
                let peers_db_global = PEERS_DB_GLOBAL.clone();
                let mut peer_db = peers_db_global.write().await;
                let peer = peer_db.get_mut(&connection_id).unwrap();
                SaitoPeer::handle_peer_command(peer, &api_message_orig).await;
            }
        }
    }
    // TODO Should this really be here? I don't think it's needed... let's just copy the
    // code in-place where it's called or at least put this somewhere else...
    pub async fn add_block_to_mempool(&mut self, block: Block) {
        {
            let mut mempool = self.mempool_lock.write().await;
            mempool.add_block(block);
        }
        Mempool::send_blocks_to_blockchain(self.mempool_lock.clone(), self.blockchain_lock.clone())
            .await;
    }
    /// Handlers for all the network API commands, e.g. REQBLOCK.
    pub async fn handle_peer_command(peer: &mut SaitoPeer, api_message_orig: &APIMessage) {
        // TODO eliminate this .clone():
        let api_message = api_message_orig.clone();
        let mempool_lock = peer.mempool_lock.clone();
        let blockchain_lock = peer.blockchain_lock.clone();
        let command = api_message_orig.get_message_name_as_string();
        event!(Level::INFO, "HANDLING COMMAND {}", command);
        match command.as_str() {
            "SHAKINIT" => {
                if let Ok(serialized_handshake_challenge) =
                    build_serialized_challenge(&api_message, peer.wallet_lock.clone()).await
                {
                    peer.send_response(api_message.message_id, serialized_handshake_challenge)
                        .await;
                }
            }
            "SHAKCOMP" => match socket_handshake_verify(&api_message.get_message_data()) {
                Some(deserialize_challenge) => {
                    peer.set_has_completed_handshake(true);
                    peer.set_publickey(deserialize_challenge.opponent_pubkey());
                    peer.send_response(
                        api_message.message_id,
                        String::from("OK").as_bytes().try_into().unwrap(),
                    )
                    .await;
                }
                None => {
                    event!(Level::ERROR, "Error verifying peer handshake signature");
                }
            },
            "REQBLOCK" => {
                let api_message = build_request_block_response(&api_message, blockchain_lock).await;
                send_message_to_socket(api_message, &peer.connection_id).await;
            }
            "REQBLKHD" => {
                let message_id = api_message.message_id;
                if let Some(bytes) = socket_send_block_header(&api_message, blockchain_lock).await {
                    let message_data = String::from("OK").as_bytes().try_into().unwrap();
                    peer.send_response(message_id, message_data).await;
                    let _result = peer.send_command("SNDBLKHD", bytes).await;
                } else {
                    peer.send_error_response_from_str(message_id, "ERROR").await;
                }
            }
            "REQCHAIN" => {
                peer.send_response_from_str(api_message.message_id, "OK")
                    .await;
                if let Some(send_blockchain_message) = build_send_blockchain_message(
                    &RequestBlockchainMessage::deserialize(&api_message.get_message_data()),
                    blockchain_lock,
                )
                .await
                {
                    let connection_id_clone = peer.connection_id.clone();
                    tokio::spawn(async move {
                        let peers_db_global = PEERS_DB_GLOBAL.clone();
                        let mut peer_db = peers_db_global.write().await;
                        let peer = peer_db.get_mut(&connection_id_clone).unwrap();

                        let _result = peer
                            .send_command(
                                &String::from("SNDCHAIN"),
                                send_blockchain_message.serialize(),
                            )
                            .await;
                    });
                } else {
                    peer.send_error_response_from_str(api_message.message_id, "UNKNOWN BLOCK HASH")
                        .await;
                }
            }
            "SNDTRANS" => {
                if let Some(tx) = socket_receive_transaction(api_message.clone()) {
                    let mut mempool = mempool_lock.write().await;
                    mempool.add_transaction(tx).await;
                    peer.send_response_from_str(api_message.message_id, "OK")
                        .await;
                    // in event of error
                    //peer.send_error_response(
                    //            api_message.message_id,
                    //            String::from("Invalid").as_bytes().try_into().unwrap(),
                    //        )
                    //        .await;
                }
            }
            "SNDCHAIN" => {
                peer.send_response_from_str(api_message.message_id, "OK")
                    .await;

                let send_blockchain_message =
                    SendBlockchainMessage::deserialize(api_message.get_message_data());
                for send_blockchain_block_data in
                    send_blockchain_message.get_blocks_data().into_iter()
                {
                    let request_block_message = RequestBlockMessage::new(
                        None,
                        Some(send_blockchain_block_data.block_hash),
                        None,
                    );
                    let connection_id_clone = peer.connection_id.clone();
                    let mempool_lock = mempool_lock.clone();
                    tokio::spawn(async move {
                        let peers_db_global = PEERS_DB_GLOBAL.clone();
                        let mut peer_db = peers_db_global.write().await;
                        let peer = peer_db.get_mut(&connection_id_clone).unwrap();
                        let result = peer
                            .send_command(
                                &String::from("REQBLOCK"),
                                request_block_message.serialize(),
                            )
                            .await;
                        match result {
                            Ok(serialized_block_message) => {
                                let block = Block::deserialize_for_net(
                                    serialized_block_message.get_message_data(),
                                );
                                {
                                    let mut mempool = mempool_lock.write().await;
                                    mempool.add_block(block);
                                }
                                Mempool::send_blocks_to_blockchain(
                                    peer.mempool_lock.clone(),
                                    peer.blockchain_lock.clone(),
                                )
                                .await;
                            }
                            Err(error_message) => {
                                event!(
                                    Level::ERROR,
                                    "REQBLOCK ERROR: {}",
                                    error_message.get_message_data_as_str()
                                );
                            }
                        }
                    });
                }
            }
            "SNDBLKHD" => {
                peer.send_error_response_from_str(api_message.message_id, "UNHANDLED COMMAND")
                    .await;
            }
            "SNDKYLST" => {
                peer.send_error_response_from_str(api_message.message_id, "UNHANDLED COMMAND")
                    .await;
            }
            _ => {
                event!(
                    Level::ERROR,
                    "Unhandled command received by client... {}",
                    &api_message.get_message_name_as_string()
                );
                peer.send_error_response_from_str(api_message.message_id, "NO SUCH")
                    .await;
            }
        }
    }
}
/// spawns a task to read messages from the socket of inbound peers.
pub async fn handle_inbound_peer_connection(
    ws: WebSocket,
    peer_db_lock: Arc<RwLock<PeersDB>>,
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

    let connection_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
    let peer = SaitoPeer::new(
        connection_id,
        None,
        None,
        true,
        false,
        false,
        wallet_lock.clone(),
        mempool_lock.clone(),
        blockchain_lock.clone(),
    );

    peer_db_lock
        .write()
        .await
        .insert(connection_id.clone(), peer);

    let inbound_peer_db_lock_global = INBOUND_PEER_CONNECTIONS_GLOBAL.clone();
    inbound_peer_db_lock_global.write().await.insert(
        connection_id.clone(),
        InboundPeer {
            sender: peer_sender,
        },
    );

    tokio::task::spawn(async move {
        while let Some(result) = peer_ws_rcv.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("error receiving ws message for peer: {}", e);
                    break;
                }
            };
            if !msg.as_bytes().is_empty() {
                let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
                SaitoPeer::handle_peer_message(api_message, connection_id).await;
            } else {
                event!(
                    Level::ERROR,
                    "Message of length 0... why?\n
                    This seems to occur if we aren't holding a reference to the sender/stream on the\n
                    other end of the connection. I suspect that when the stream goes out of scope,\n
                    it's deconstructor is being called and sends a 0 length message to indicate\n
                    that the stream has ended... I'm leaving this println here for now because\n
                    it would be very helpful to see this if starts to occur again. We may want to\n
                    treat this as a disconnect."
                );
            }
        }
        // We had some error. Remove all references to this peer.
        {
            let mut peer_db = peer_db_lock.write().await;
            let peer = peer_db.get_mut(&connection_id).unwrap();
            peer.set_is_connected_or_connecting(false).await;
            peer_db.remove(&connection_id);
        }
    });
}

pub async fn build_serialized_challenge(
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
        event!(
            Level::ERROR,
            "Error validating timestamp for handshake complete"
        );
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

pub async fn build_request_block_response(
    api_message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> APIMessage {
    let request_block_message = RequestBlockMessage::deserialize(api_message.get_message_data());
    let blockchain = blockchain_lock.read().await;
    if request_block_message.get_block_id().is_some() {
        APIMessage::new_from_string("ERROR___", api_message.message_id, "Unsupported: fetching blocks by id is not yet supported, please fetch the block by hash.")
    } else if request_block_message.get_block_hash().is_some() {
        //let block_hash: SaitoHash = api_message.message_data[0..32].try_into().unwrap();
        let block_hash: SaitoHash = request_block_message.get_block_hash().unwrap();

        match blockchain.get_block_sync(&block_hash) {
            Some(target_block) => APIMessage::new(
                "RESULT__",
                api_message.message_id,
                target_block.serialize_for_net(BlockType::Full),
            ),
            None => APIMessage::new_from_string(
                "ERROR___",
                api_message.message_id,
                "Unknown Block Hash",
            ),
        }
    } else {
        APIMessage::new_from_string(
            "ERROR___",
            api_message.message_id,
            "REQBLOCK requires ID or Hash of desired block",
        )
    }
}

pub async fn socket_send_block_header(
    api_message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> Option<Vec<u8>> {
    let block_hash: SaitoHash = api_message.message_data[0..32].try_into().unwrap();
    let blockchain = blockchain_lock.read().await;

    match blockchain.get_block_sync(&block_hash) {
        Some(target_block) => Some(target_block.serialize_for_net(BlockType::Header)),
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

    let blockchain = blockchain_lock.read().await;

    let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
    if let Some(latest_block) = blockchain.get_latest_block() {
        let mut previous_block_hash: SaitoHash = latest_block.get_hash();
        let mut this_block: &Block; // = blockchain.get_block_sync(&previous_block_hash).unwrap();
        let mut block_count = 0;
        while &previous_block_hash != peers_latest_hash && block_count < GENESIS_PERIOD {
            block_count += 1;
            this_block = blockchain.get_block_sync(&previous_block_hash).unwrap();
            blocks_data.push(SendBlockchainBlockData {
                block_id: this_block.get_id(),
                block_hash: this_block.get_hash(),
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
