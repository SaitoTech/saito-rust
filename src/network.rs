use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{hash, sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::networking::filters::{
    get_block_route_filter, post_transaction_route_filter, ws_upgrade_route_filter,
};
use crate::peer::{InboundSocket, OutboundSocket, Peer, PeerType};
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use async_recursion::async_recursion;
use futures::{Future, FutureExt, SinkExt, StreamExt};
use log::debug;
use secp256k1::PublicKey;
use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::connect_async;
use tracing::field::debug;
use tracing::{error, info, warn};
use uuid::Uuid;
use warp::ws::{Message, WebSocket};
use warp::{Filter, Rejection};

//use super::message_types::send_block_head_message::SendBlockHeadMessage;
use crate::networking::signals::signal_for_shutdown;

use crate::configuration::{PeerSetting, Settings};
use crate::networking::api_message::APIMessage;
use crate::networking::message_types::request_blockchain_message::RequestBlockchainMessage;
use crate::networking::message_types::send_block_head_message::SendBlockHeadMessage;
use crate::util::format_url_string;

pub type RunResult<T> = std::result::Result<T, Rejection>;

pub const CHALLENGE_SIZE: usize = 82;
pub const CHALLENGE_EXPIRATION_TIME: u64 = 60000;

//
// why do we have inbound and outbound peer DB at this point?
//
pub type PeersDB = HashMap<SaitoHash, Peer>;
pub type RequestResponses = HashMap<(SaitoHash, u32), APIMessage>;
pub type RequestWakers = HashMap<(SaitoHash, u32), Waker>;
pub type OutboundSocketsDB = HashMap<SaitoHash, OutboundSocket>;
pub type InboundSocketsDB = HashMap<SaitoHash, InboundSocket>;

lazy_static::lazy_static! {
    pub static ref PEERS_DB_GLOBAL: Arc<tokio::sync::RwLock<PeersDB>> = Arc::new(tokio::sync::RwLock::new(PeersDB::new()));
    pub static ref PEERS_REQUEST_RESPONSES_GLOBAL: Arc<std::sync::RwLock<RequestResponses>> = Arc::new(std::sync::RwLock::new(RequestResponses::new()));
    pub static ref PEERS_REQUEST_WAKERS_GLOBAL: Arc<std::sync::RwLock<RequestWakers>> = Arc::new(std::sync::RwLock::new(RequestWakers::new()));
    pub static ref INBOUND_SOCKETS_GLOBAL: Arc<tokio::sync::RwLock<InboundSocketsDB>> = Arc::new(tokio::sync::RwLock::new(InboundSocketsDB::new()));
    pub static ref OUTBOUND_SOCKETS_GLOBAL: Arc<tokio::sync::RwLock<OutboundSocketsDB>> = Arc::new(tokio::sync::RwLock::new(OutboundSocketsDB::new()));
}

//
// Local Broadcast Message Types
//
// In addition to responding to global broadcast messages, the
// network has a local broadcast channel it uses to coordinate
// attempts to check that connections are stable and clean up
// problematic peers.
//
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    LocalNetworkMonitoring,
}
//
// A future which wraps an APIMessage REQUEST->RESPONSE into a Future(e.g. REQBLOCK->RESULT__).
// This enables a much cleaner interface for inter-node message relays by allowing nodes to await
// on messages that are sent and then handling the results directly as either RESULT__ or
// ERROR___ responses.
//
pub struct NetworkRequestFuture {
    connection_id: SaitoHash,
    request_id: u32,
    api_message_command: String,
}
impl NetworkRequestFuture {
    pub async fn new(command: &str, message: Vec<u8>, connection_id: &SaitoHash) -> Self {
        Network::send_request(command, message, &connection_id).await;

        let mut request_count = 0;
        {
            let peer_connection_db_global = PEERS_DB_GLOBAL.clone();
            let mut peer_connection_db = peer_connection_db_global.write().await;
            let mut peer = peer_connection_db.get_mut(connection_id).unwrap();
            request_count = peer.request_count - 1;
        }

        NetworkRequestFuture {
            connection_id: *connection_id,
            request_id: request_count,
            api_message_command: String::from(command),
        }
    }
}
impl Future for NetworkRequestFuture {
    type Output = Result<APIMessage, Box<dyn Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
        let mut request_responses = request_responses_lock.write().unwrap();
        match request_responses.remove(&(self.connection_id, self.request_id)) {
            Some(response) => {
                // Return from the Future with an important message!
                info!("HANDLING RESPONSE {}", self.api_message_command);
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

pub struct Network {
    blockchain_lock: Arc<RwLock<Blockchain>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    host: [u8; 4],
    port: u16,
    peer_conf: Option<Vec<PeerSetting>>,
}

impl Network {
    /// Create a Network
    pub fn new(
        configuration: Settings,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        wallet_lock: Arc<RwLock<Wallet>>,
        broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    ) -> Network {
        Network {
            host: configuration.network.host,
            port: configuration.network.port,
            peer_conf: configuration.network.peers,
            blockchain_lock,
            mempool_lock,
            wallet_lock,
            broadcast_channel_sender,
        }
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = bcs;
    }

    //
    // initialize
    //
    async fn initialize(&self) {
        info!("{:?}", self.peer_conf);
        if let Some(peer_settings) = &self.peer_conf {
            for peer_setting in peer_settings {
                let connection_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
                let mut peer = Peer::new(
                    connection_id,
                    Some(peer_setting.host),
                    Some(peer_setting.port),
                );
                peer.set_peer_type(PeerType::Outbound);

                {
                    let peers_db_global = PEERS_DB_GLOBAL.clone();
                    peers_db_global
                        .write()
                        .await
                        .insert(connection_id.clone(), peer);
                }
            }
        }
    }

    //
    // connect to other peers
    //
    async fn add_outbound_peer(connection_id: SaitoHash, wallet_lock: Arc<RwLock<Wallet>>) {
        //
        // first we connect to the peer
        //
        let peers_db_global = PEERS_DB_GLOBAL.clone();
        let peer_url;
        {
            let mut peer_db = peers_db_global.write().await;
            let mut peer = peer_db.get_mut(&connection_id).unwrap();
            peer_url = url::Url::parse(&format!(
                "ws://{}/wsopen",
                format_url_string(peer.host.unwrap(), peer.port.unwrap()),
            ))
            .unwrap();
            peer.set_is_connecting(true);
        }

        //
        // create and save socket
        //
        let ws_stream_result = connect_async(peer_url).await;
        match ws_stream_result {
            Ok((ws_stream, _)) => {
                //
                // save socket
                //
                let (write_sink, mut read_stream) = ws_stream.split();
                {
                    let outbound_socket_db_global = OUTBOUND_SOCKETS_GLOBAL.clone();
                    outbound_socket_db_global
                        .write()
                        .await
                        .insert(connection_id, OutboundSocket { write_sink });
                }

                //
                // spawn thread to handle messages
                //
                tokio::spawn(async move {
                    while let Some(result) = read_stream.next().await {
                        match result {
                            Ok(message) => {
                                if !message.is_empty() {
                                    let api_message = APIMessage::deserialize(&message.into_data());
                                    Network::receive_request(api_message, connection_id).await;
                                } else {
                                    error!(
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
                            Err(error) => {
                                error!("Error reading from peer socket {:?}", error);
                                let peers_db_global = PEERS_DB_GLOBAL.clone();
                                let mut peer_db = peers_db_global.write().await;
                                let peer = peer_db.get_mut(&connection_id).unwrap();
                                peer.set_is_connected(false);
                                peer.set_is_connecting(false);
                            }
                        }
                    }
                });

                //
                // trigger anything we want to start on initialization
                //
                //Network::handshake_and_synchronize_chain(&connection_id, wallet_lock).await;
            }
            Err(error) => {
                error!("Error connecting to peer {:?}", error);
                let mut peer_db = peers_db_global.write().await;
                let peer = peer_db.get_mut(&connection_id).unwrap();
                peer.set_is_connected(false);
                peer.set_is_connecting(false);
            }
        }
    }

    //
    // add inbound peers
    //
    pub async fn add_inbound_peer(ws: WebSocket, network_lock: Arc<RwLock<Network>>) {
        debug!("add_inbound_peer");
        let (peer_ws_sender, mut peer_ws_rcv) = ws.split();
        let (peer_sender, peer_rcv) = mpsc::unbounded_channel();
        let peer_rcv = UnboundedReceiverStream::new(peer_rcv);
        tokio::task::spawn(peer_rcv.forward(peer_ws_sender).map(|result| {
            if let Err(e) = result {
                error!("error sending websocket msg: {}", e);
            }
        }));

        let connection_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
        let mut peer = Peer::new(connection_id, None, None);
        peer.set_peer_type(PeerType::Inbound);

        //
        // add peer to global static db
        //
        let peers_db_global = PEERS_DB_GLOBAL.clone();
        peers_db_global
            .write()
            .await
            .insert(connection_id.clone(), peer);

        //
        // add inbound socket
        //
        let inbound_sockets_global = INBOUND_SOCKETS_GLOBAL.clone();
        inbound_sockets_global.write().await.insert(
            connection_id.clone(),
            InboundSocket {
                sender: peer_sender,
            },
        );

        //
        // spawn a thread that waits for broadcasts that are sent on
        // this channel, which will be APIMessages. Once we receive
        // this messsage we forward it to Network::receiveRequest
        //
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
                    Network::receive_request(api_message, connection_id).await;
                } else {
                    error!(
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
                let mut peer_db = peers_db_global.write().await;
                let mut peer = peer_db.get_mut(&connection_id).unwrap();
                peer.set_is_connected(false);
                peer.set_is_connecting(false);
                peer_db.remove(&connection_id);
            }
        });
    }

    //
    // receive requests from peers
    //
    pub async fn receive_request(api_message: APIMessage, connection_id: SaitoHash) {
        debug!(
            "receive_request : {:?}",
            api_message.get_message_name_as_string()
        );
        match api_message.get_message_name_as_string().as_str() {
            "RESULT__" | "ERROR___" => {
                let request_wakers_lock = PEERS_REQUEST_WAKERS_GLOBAL.clone();
                let mut request_wakers = request_wakers_lock.write().unwrap();
                let option_waker = request_wakers.remove(&(connection_id, api_message.message_id));

                let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
                let mut request_responses = request_responses_lock.write().unwrap();
                request_responses.insert((connection_id, api_message.message_id), api_message);

                if let Some(waker) = option_waker {
                    waker.wake();
                }
            }
            _ => {
                //
                // TODO - we do not need to fetch every peer mutably just to answer
                // and deal with inbound requests.
                //
                // let peers_db_global = PEERS_DB_GLOBAL.clone();
                // let mut peer_db = peers_db_global.write().await;
                // let peer = peer_db.get_mut(&connection_id).unwrap();

                let command = api_message.get_message_name_as_string();
                match command.as_str() {
                    "REQBLOCK" => {}
                    _ => {
                        error!(
                            "Unhandled command received by client... {}",
                            &api_message.get_message_name_as_string()
                        );
                        Network::send_request(
                            &"RESULT__",
                            "RESULT__".as_bytes().to_vec(),
                            &connection_id,
                        )
                        .await;
                        debug!("result sent");
                        // peer.send_error_response_from_str(api_message.message_id, "NO SUCH")
                        //     .await;
                    }
                }
            }
        }
    }

    //
    // send request to peer
    //
    // Sends an APIMessage to another peer through a socket connection. Since Outbound and Inbound peers
    // Streams(Sinks) are not unified into a single Trait yet, we have to check both databases to find out
    // which sort of sink this peer is using and send the message through the appropriate stream.
    //
    pub async fn send_request(
        message_name: &str,
        message_data: Vec<u8>,
        connection_id: &SaitoHash,
    ) {
        debug!(
            "sending request : {:?} via connection : {:?}",
            message_name,
            hex::encode(connection_id)
        );
        let peer_connection_db_global = PEERS_DB_GLOBAL.clone();
        let mut peer_connection_db = peer_connection_db_global.write().await;
        let mut peer = peer_connection_db.get_mut(connection_id).unwrap();

        let api_message = APIMessage::new_for_peer(message_name, message_data, peer);

        let outbound_peer_connection_db_global = OUTBOUND_SOCKETS_GLOBAL.clone();
        let mut outbound_peer_connection_db = outbound_peer_connection_db_global.write().await;
        let inbound_peer_connection_db_global = INBOUND_SOCKETS_GLOBAL.clone();
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
            outbound_peer_connection
                .unwrap()
                .write_sink
                .send(api_message.serialize().into())
                .await
                .expect("unable to send to outbound_peer_connection... It may be that the socket\n
                was closed but the outbound peer was not cleaned up. If this occurs, it should be investigated.");
        }
        if inbound_peer_connection.is_some() {
            inbound_peer_connection
                .unwrap()
                .sender
                .send(Ok(Message::binary(api_message.serialize())))
                .expect("unable to send to outbound_peer_connection... It may be that the socket\n
                was closed but the inbount peer was not cleaned up. If this occurs, it should be investigated.");
        }
    }

    //
    // send request to peer
    //
    // Sends an API message to another peer through a socket connection, while handling the process through
    // a PeerRequest that allows us to .await the response and continue execution when and if the remote
    // peer responds to the message.
    //
    // #[async_recursion]
    pub async fn send_request_with_future(
        message_name: &str,
        message_data: Vec<u8>,
        connection_id: &SaitoHash,
    ) -> Result<APIMessage, APIMessage> {
        debug!(
            "send_request_with_future : {:?} via connection : {:?}",
            message_name,
            hex::encode(connection_id),
        );
        let network_request =
            NetworkRequestFuture::new(message_name, message_data, connection_id).await;
        let response_message = network_request
            .await
            .expect(&format!("Error returned from {}", message_name));
        match response_message.get_message_name_as_string().as_str() {
            "RESULT__" => Ok(response_message),
            "ERROR___" => Err(response_message),
            _ => {
                panic!("Received non-response response");
            }
        }
    }

    //
    // fetch block from network
    //
    async fn fetch_block(_block_hash: SaitoHash) {
        /***
         ***/
    }

    //
    // send block to all peers
    //
    #[warn(dead_code)]
    async fn propagate_block(_block_hash: SaitoHash) {
        /***
                let peers_db_global = PEERS_DB_GLOBAL.clone();
                let mut peers_db_mut = peers_db_global.write().await;
                // We need a stream iterator for async(to await send_command_fire_and_forget)
                let mut peers_iterator_stream = futures::stream::iter(peers_db_mut.values_mut());
                while let Some(peer) = peers_iterator_stream.next().await {
                    if peer.get_has_completed_handshake() {
                        let send_block_head_message = SendBlockHeadMessage::new(block_hash);
                        peer.send_command_fire_and_forget("SNDBLKHD", send_block_head_message.serialize())
                            .await;
                    } else {
                        info!("Hasn't completed handshake, will not send block??");
                    }
                }
        ***/
    }

    #[warn(dead_code)]
    pub async fn propagate_transaction(_wallet_lock: Arc<RwLock<Wallet>>, mut _tx: Transaction) {
        /****
                tokio::spawn(async move {
                    let peers_db_global = PEERS_DB_GLOBAL.clone();
                    let mut peers_db_mut = peers_db_global.write().await;
                    // We need a stream iterator for async(to await send_command_fire_and_forget)
                    let mut peers_iterator_stream = futures::stream::iter(peers_db_mut.values_mut());
                    while let Some(peer) = peers_iterator_stream.next().await {
                        if peer.get_has_completed_handshake() && !peer.is_in_path(&tx.get_path()) {
                            // change the last bytes in the vector for each SNDTRANS
                            let hop = tx
                                .build_last_hop(wallet_lock.clone(), peer.get_publickey().unwrap())
                                .await;

                            peer.send_command_fire_and_forget(
                                "SNDTRANS",
                                tx.serialize_for_net_with_hop(hop),
                            )
                                .await;
                        } else {
                            info!("Hasn't completed handshake, will not send transaction??");
                        }
                    }
                });
        ****/
    }
}

pub async fn run(
    network_lock: Arc<RwLock<Network>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::network::RunResult<()> {
    //
    // network gets global broadcast channel
    //
    {
        let mut network = network_lock.write().await;
        network.set_broadcast_channel_sender(broadcast_channel_sender.clone());
    }

    //
    // start network monitor (local)
    //
    let (network_channel_sender, mut network_channel_receiver) = mpsc::channel(4);
    let network_monitor_sender = network_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            network_monitor_sender
                .send(NetworkMessage::LocalNetworkMonitoring)
                .await
                .expect("Failed to send LocalNetworkMonitoring message");
            sleep(Duration::from_millis(10000)).await;
        }
    });

    //
    // initialize server
    //
    let network_lock_clone = network_lock.clone();
    tokio::select! {
        res = run_server(network_lock_clone) => {
            if let Err(err) = res {
                eprintln!("run_server err {:?}", err)
            }
        },
    }

    //
    // initialize network
    //
    {
        let network = network_lock.write().await;
        network.initialize().await;
    }

    //
    // listen to local and global messages
    //
    let network_lock_clone2 = network_lock.clone();
    loop {
        tokio::select! {

            //
            // Local Channel Messages
            //
            Some(message) = network_channel_receiver.recv() => {
                match message {

                    //
                    // Monitor the Network
                    //
                    NetworkMessage::LocalNetworkMonitoring => {

                        //
                        // Check Disconnected Peers
                        //
                        let peers_to_check: Vec<(SaitoHash, bool)>;
                        {
                            let peers_db_global = PEERS_DB_GLOBAL.clone();
                            let peers_db = peers_db_global.read().await;
                            peers_to_check = peers_db
                            .keys()
                            .map(|connection_id| {
                                let peer = peers_db.get(connection_id).unwrap();
                                let should_reconnect = peer.is_peer_type(PeerType::Outbound)
                                    && !peer.is_connected && !peer.is_connecting;
                                (*connection_id, should_reconnect)
                            })
                            .collect::<Vec<(SaitoHash, bool)>>();
                        }
                        for (connection_id, should_reconnect) in peers_to_check {
                            if should_reconnect {
                               info!("found disconnected peer in peer settings, (re)connecting...");
                                let network = network_lock_clone2.read().await;
                                let wallet_lock_clone = network.wallet_lock.clone();
                                Network::add_outbound_peer(connection_id, wallet_lock_clone).await;
                            }
                        }

                        // reconnect one-by-one
                        info!("Finished Connecting!");

                    },
                    _ => {}
                }
            }


            //
            // Saito Channel Messages
            //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    SaitoMessage::BlockchainNewLongestChainBlock { hash : block_hash, difficulty } => {
                    info!("Network aware of new longest chain block!");
                    },
                    SaitoMessage::BlockchainSavedBlock { hash: block_hash } => {
                        warn!("SaitoMessage::BlockchainSavedBlock recv'ed by network");
                        Network::propagate_block(block_hash).await;
                    },
                    SaitoMessage::WalletNewTransaction { transaction: tx } => {
                        info!("SaitoMessage::WalletNewTransaction new tx is detected by network");
                        let network = network_lock_clone2.read().await;
                        Network::propagate_transaction(network.wallet_lock.clone(), tx).await;
                    },
                    SaitoMessage::MissingBlock {
                        peer_id: connection_id,
                        hash: block_hash,
                    } => {
                        warn!("SaitoMessage::MissingBlock message received over broadcast channel");
                        //Network::fetch_block();
                    },
                    _ => {}
                }
            }
        }
    }
}

/// Runs warp::serve to listen for incoming connections
pub async fn run_server(network_lock_clone: Arc<RwLock<Network>>) -> crate::network::RunResult<()> {
    let mut routes;
    let mut host;
    let mut port;

    {
        let network = network_lock_clone.read().await;
        port = network.port;
        host = network.host;
        routes = get_block_route_filter(network.blockchain_lock.clone())
            .or(post_transaction_route_filter(
                network.mempool_lock.clone(),
                network.blockchain_lock.clone(),
            ))
            .or(ws_upgrade_route_filter(network_lock_clone.clone()));

        info!("Listening for HTTP on port {}", port);
        let (_, server) =
            warp::serve(routes).bind_with_graceful_shutdown((host, port), signal_for_shutdown());
        server.await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use log::debug;
    use std::borrow::Borrow;
    use std::convert::TryInto;
    use std::thread;

    use super::*;

    use crate::configuration::get_configuration;
    use crate::{
        block::{Block, BlockType},
        crypto::{generate_keys, hash, verify},
        networking::{api_message::APIMessage, filters::ws_upgrade_route_filter},
        test_utilities::test_manager::TestManager,
        time::create_timestamp,
    };
    use warp::{test::WsClient, ws::Message};

    /// This function will be used in mosts test of network, it will open a socket, negotiate a handshake,
    /// and return the socket so we are ready to start sending APIMessages through the socket, which
    /// we can use as a mock peer.
    #[tokio::test]
    async fn send_request_tests() {
        let (publickey, privatekey) = generate_keys();

        let mut settings = get_configuration().expect("failed to read configuration");
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);
        let network_lock = Arc::new(RwLock::new(Network::new(
            settings,
            blockchain_lock.clone(),
            mempool_lock.clone(),
            wallet_lock.clone(),
            broadcast_channel_sender.clone(),
        )));
        let network_lock_clone = network_lock.clone();
        tokio::spawn(async move {
            crate::network::run(
                network_lock_clone,
                broadcast_channel_sender.clone(),
                broadcast_channel_receiver,
            )
            .await;
        });
        // use Warp test to open a socket ?
        let socket_filter = ws_upgrade_route_filter(network_lock.clone());

        let mut connection_id: SaitoHash = [0; 32];
        {
            let peer = Peer::new(connection_id.clone(), Some([127, 0, 0, 1]), Some(3000));
            let mut peerdb = PEERS_DB_GLOBAL.write().await;
            peerdb.insert(connection_id.clone(), peer);
        }
        Network::add_outbound_peer(connection_id.clone(), wallet_lock.clone()).await;
        {
            debug!("peer count = {:?}", PEERS_DB_GLOBAL.read().await.len());
            debug!(
                "outbound connections = {:?}",
                OUTBOUND_SOCKETS_GLOBAL.read().await.len()
            );
        }
        //
        // send a send_request() message to this socket
        //
        // it should trigger something but not send a response

        // let message_name = "TESTING1";
        // crate::network::Network::send_request(
        //     message_name,
        //     [0, 1, 2, 3, 4, 5].to_vec(),
        //     &connection_id,
        // )
        // .await;
        // debug!("cccc");

        //
        // send a send_request_with_future() message and await it
        //
        // it should return the expected response
        let mut handle = tokio::spawn(async {
            let message_name = "TESTING2";
            let result = Network::send_request_with_future(
                message_name,
                [0, 1, 2, 3, 4, 5, 6].to_vec(),
                &[0; 32],
            )
            .await;
            let message = result.unwrap();
            debug!("reply is : {:?}", message.get_message_name_as_string());
            debug!("bbb");
        });
        handle.await;
        debug!("ccc");
        // assert!(result.is_ok());
    }
}
