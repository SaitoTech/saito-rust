use super::api_message::APIMessage;
use super::network::CHALLENGE_EXPIRATION_TIME;
use crate::block::Block;
use crate::blockchain::{Blockchain, GENESIS_PERIOD};
use crate::crypto::{hash, sign_blob, verify, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::{AddTransactionResult, Mempool};
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
use futures::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

use futures::{FutureExt, SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

pub type PeersDB = HashMap<SaitoHash, BasePeer>;
pub type OutboundPeersDB = HashMap<SaitoHash, OutboundPeer>;
pub type InboundPeersDB = HashMap<SaitoHash, InboundPeer>;

lazy_static::lazy_static! {
    // We use Arc for thread-safe reference counting and Mutex for thread-safe mutabilitity
    pub static ref PEERS_DB_GLOBAL: Arc<RwLock<PeersDB>> = Arc::new(RwLock::new(PeersDB::new()));
    pub static ref INBOUND_PEER_CONNECTIONS_GLOBAL: Arc<RwLock<InboundPeersDB>> = Arc::new(RwLock::new(InboundPeersDB::new()));
    pub static ref OUTBOUND_PEER_CONNECTIONS_GLOBAL: Arc<RwLock<OutboundPeersDB>> = Arc::new(RwLock::new(OutboundPeersDB::new()));
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}

pub struct PeerFlags {
    is_connected_or_connecting: bool,
    has_completed_handshake: bool,
    is_from_peer_list: bool,
}

pub struct BasePeer {
    peer_flags: PeerFlags,
    connection_id: SaitoHash,
    publickey: Option<SaitoPublicKey>,
    host: Option<[u8; 4]>,
    port: Option<u16>,
    requests: HashMap<u32, [u8; 8]>,
    request_count: u32,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
}

pub trait PeerSocket {
    fn send(&self, api_message: APIMessage);
    fn get_next_message();
}
pub struct InboundPeer {
    pub sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>,
}
pub struct OutboundPeer {
    pub write_sink:
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::protocol::Message>,
}

impl BasePeer {
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
    ) -> BasePeer {
        BasePeer {
            peer_flags: PeerFlags {
                is_connected_or_connecting,
                has_completed_handshake,
                is_from_peer_list,
            },
            connection_id: connection_id,
            host: host,
            port: port,
            publickey: None,
            requests: HashMap::new(),
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

    async fn send(&mut self, api_message: APIMessage) {
        println!("SEND {}", api_message.message_name_as_string());
        let outbound_peer_connection_db_global = OUTBOUND_PEER_CONNECTIONS_GLOBAL.clone();
        let mut outbound_peer_connection_db = outbound_peer_connection_db_global.write().await;
        let inbound_peer_connection_db_global = INBOUND_PEER_CONNECTIONS_GLOBAL.clone();
        let mut inbound_peer_connection_db = inbound_peer_connection_db_global.write().await;

        let outbound_peer_connection = outbound_peer_connection_db.get_mut(&self.connection_id);
        let inbound_peer_connection = inbound_peer_connection_db.get_mut(&self.connection_id);

        if inbound_peer_connection.is_some() && outbound_peer_connection.is_some() {
            panic!("Peer has both inbound and outbound connections, this should never happen");
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
    pub async fn send_command(&mut self, command: &str, message: Vec<u8>) {
        self.requests.insert(
            self.request_count,
            String::from(command).as_bytes().try_into().unwrap(),
        );
        self.request_count += 1;
        self.send(APIMessage::new(command, self.request_count - 1, message))
            .await;
    }
    pub async fn send_response_from_str(&mut self, message_id: u32, message_str: &str) {
        self.send(APIMessage::new_from_string(
            "RESULT__",
            message_id,
            message_str,
        ))
        .await;
    }
    pub async fn send_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(APIMessage::new("RESULT__", message_id, message))
            .await;
    }
    pub async fn send_error_response_from_str(&mut self, message_id: u32, message_str: &str) {
        self.send(APIMessage::new_from_string(
            "ERROR___",
            message_id,
            message_str,
        ))
        .await;
    }
    pub async fn send_error_response(&mut self, message_id: u32, message: Vec<u8>) {
        self.send(APIMessage::new("ERROR___", message_id, message))
            .await;
    }
    pub async fn handle_peer_command(&mut self, api_message_orig: &APIMessage) {
        let api_message = api_message_orig.clone();
        let mempool_lock = self.mempool_lock.clone();
        let blockchain_lock = self.blockchain_lock.clone();
        match api_message_orig.message_name_as_string().as_str() {
            "RESULT__" => {
                println!("GOT RESULT__");
                let command_sent = self.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_response_command(command_name, &api_message)
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
                println!("GOT ERROR___");
                let command_sent = self.requests.remove(&api_message.message_id);
                match command_sent {
                    Some(command_name) => {
                        self.handle_peer_response_error(command_name, &api_message);
                    }
                    None => {
                        println!(
                            "Peer has sent an error {} . Request ID: {}",
                            api_message.message_name_as_string(),
                            api_message.message_id
                        );
                    }
                }
            }
            "SHAKINIT" => {
                println!("GOT SHAKINIT");
                if let Ok(serialized_handshake_challenge) =
                    build_serialized_challenge(&api_message, self.wallet_lock.clone()).await
                {
                    self.send_response(api_message.message_id, serialized_handshake_challenge)
                        .await;
                }
            }
            "SHAKCOMP" => {
                println!("GOT SHAKCOMP");
                match socket_handshake_verify(&api_message.message_data()) {
                    Some(deserialize_challenge) => {
                        self.set_has_completed_handshake(true);
                        self.set_publickey(deserialize_challenge.opponent_pubkey());
                        self.send_response(
                            api_message.message_id,
                            String::from("OK").as_bytes().try_into().unwrap(),
                        )
                        .await;
                        println!("HANDSHAKE COMPLETE!");
                    }
                    None => {
                        println!("Error verifying peer handshake signature");
                    }
                }
            }
            "REQBLOCK" => {
                println!("GOT REQBLOCK");
                let api_message = build_request_block_response(&api_message, blockchain_lock).await;
                self.send(api_message).await;
            }
            "REQBLKHD" => {
                println!("GOT REQBLKHD");
                let message_id = api_message.message_id;
                if let Some(bytes) = socket_send_block_header(&api_message, blockchain_lock).await {
                    let message_data = String::from("OK").as_bytes().try_into().unwrap();
                    self.send_response(message_id, message_data).await;
                    self.send_command("SNDBLKHD", bytes).await;
                } else {
                    self.send_error_response_from_str(message_id, "ERROR").await;
                }
            }
            "REQCHAIN" => {
                println!("GOT REQCHAIN");
                self.send_response_from_str(api_message.message_id, "OK")
                    .await;
                if let Some(send_blockchain_message) = build_send_blockchain_message(
                    &RequestBlockchainMessage::deserialize(&api_message.message_data()),
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
                    self.send_error_response_from_str(api_message.message_id, "UNKNOWN BLOCK HASH")
                        .await;
                }
            }
            "SNDTRANS" => {
                println!("GOT SNDTRANS");
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
                println!("GOT SNDCHAIN");
                let _message_id = api_message.message_id;

                let send_blockchain_message =
                    SendBlockchainMessage::deserialize(api_message.message_data());
                for send_blockchain_block_data in
                    send_blockchain_message.get_blocks_data().into_iter()
                {
                    let request_block_message = RequestBlockMessage::new(
                        None,
                        Some(send_blockchain_block_data.block_hash),
                        None,
                    );
                    self.send_command(&String::from("REQBLOCK"), request_block_message.serialize())
                        .await;
                }
            }
            "SNDBLKHD" => {
                println!("GOT SNDBLKHD");
                let _message_id = api_message.message_id;
            }
            "SNDKYLST" => {
                println!("GOT SNDKYLST");
                let _message_id = api_message.message_id;
            }
            _ => {
                println!(
                    "Unhandled command received by client... {}",
                    &api_message.message_name_as_string()
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
                println!("GOT SHAKINIT RESPONSE");
                // SHAKINIT response
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
        response_api_message: &APIMessage,
    ) {
        println!(
            "{} ERROR: {}",
            String::from_utf8_lossy(&command_name).to_string(),
            response_api_message.message_data_as_str(),
        );
        match String::from_utf8_lossy(&command_name).to_string().as_ref() {
            "SHAKCOMP" => {
                // TODO delete the peer if there is an error here
            }
            _ => {}
        }
    }
}

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
    let peer = BasePeer::new(
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

    println!("{:?} connected", connection_id);

    tokio::task::spawn(async move {
        while let Some(result) = peer_ws_rcv.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("error receiving ws message for peer: {}", e);
                    break;
                }
            };
            if msg.as_bytes().len() > 0 {
                let api_message = APIMessage::deserialize(&msg.as_bytes().to_vec());
                let mut peer_db = peer_db_lock.write().await;
                let peer = peer_db.get_mut(&connection_id).unwrap();
                peer.handle_peer_command(&api_message).await;
            } else {
                println!("Message of length 0... why?");
                println!("This seems to occur if we aren't holding a reference to the sender/stream on the");
                println!("other end of the connection. I suspect that when the stream goes out of scope,");
                println!(
                    "it's deconstructor is being called and sends a 0 length message to indicate"
                );
                println!(
                    "that the stream has ended... I'm leaving this println here for now because"
                );
                println!(
                    "it would be very helpful to see this if starts to occur again. We may want to"
                );
                println!("treat this as a disconnect.");
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

pub fn socket_receive_transaction(message: APIMessage) -> Option<Transaction> {
    let tx = Transaction::deserialize_from_net(message.message_data);
    Some(tx)
}

pub async fn build_request_block_response(
    api_message: &APIMessage,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> APIMessage {
    let request_block_message = RequestBlockMessage::deserialize(api_message.message_data());
    let blockchain = blockchain_lock.read().await;
    if request_block_message.get_block_id().is_some() {
        APIMessage::new_from_string("ERROR___", api_message.message_id, "Unsupported: fetching blocks by id is not yet supported, please fetch the block by hash.")
    } else if request_block_message.get_block_hash().is_some() {
        let block_hash: SaitoHash = api_message.message_data[0..32].try_into().unwrap();
        println!("REQBLOCK {:?}", &block_hash);
        match blockchain.get_block_sync(&block_hash) {
            Some(target_block) => APIMessage::new(
                "RESULT__",
                api_message.message_id,
                target_block.serialize_for_net(),
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
        println!("GOT BLOCK ZERO HASH");
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

    println!(
        "GET BLOCK FOR SNDCHAIN {:?}",
        &hex::encode(&peers_latest_hash)
    );

    let blockchain = blockchain_lock.read().await;

    let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
    if let Some(latest_block) = blockchain.get_latest_block() {
        let mut previous_block_hash: SaitoHash = latest_block.get_hash();
        let mut this_block: &Block; // = blockchain.get_block_sync(&previous_block_hash).unwrap();
        let mut block_count = 0;
        while &previous_block_hash != peers_latest_hash && block_count < GENESIS_PERIOD {
            block_count += 1;
            println!(
                "GET BLOCK FOR SNDCHAIN {:?}",
                &hex::encode(&previous_block_hash)
            );
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
