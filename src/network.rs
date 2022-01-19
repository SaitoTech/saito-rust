use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{hash, sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::peer::{Peer, PeerType, OutboundSocket, InboundSocket};
use crate::networking::filters::{
    get_block_route_filter, post_transaction_route_filter, ws_upgrade_route_filter,
};
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use secp256k1::PublicKey;
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};
use std::collections::HashMap;
use std::task::{Context, Poll, Waker};
use warp::ws::{Message, WebSocket};
use std::error::Error;
use std::pin::Pin;
use futures::{Future, FutureExt, SinkExt, StreamExt};
use uuid::Uuid;
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
/// Local Broadcast Message Types
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

pub struct NetworkCallback {
    connection_id: SaitoHash,
    request_id: u32,
    api_message_command: String,
}


/// A future which wraps an APIMessage REQUEST->RESPONSE into a Future(e.g. REQBLOCK->RESULT__).
/// This enables a much cleaner interface for inter-node message relays by allowing nodes to await
/// on messages that are sent and then handling the results directly as either RESULT__ or 
/// ERROR___ responses.
impl NetworkCallback {
    pub async fn new(command: &str, message: Vec<u8>, peer: &mut Peer) -> Self {
        peer.request_count += 1;
        let api_message = APIMessage::new(command, peer.request_count - 1, message);
        Network::send_request(api_message, &peer.connection_id).await;
        NetworkCallback {
            connection_id: peer.connection_id,
            request_id: peer.request_count - 1,
            api_message_command: String::from(command),
        }
    }
}

impl Future for NetworkCallback {
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
    // Initialize the network class generally, including adding any peers we have
    // configured (peers set in the configuration/*.yml) into our PEERS_DB_GLOBAL
    // data structure.
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







    pub async fn receive_request(api_message: APIMessage, connection_id: SaitoHash) {
        match api_message.get_message_name_as_string().as_str() {

            "RESULT__" | "ERROR___" => {
                let request_wakers_lock = PEERS_REQUEST_WAKERS_GLOBAL.clone();
                let mut request_wakers = request_wakers_lock.write().unwrap();
                let option_waker =
                    request_wakers.remove(&(connection_id, api_message.message_id));

                let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
                let mut request_responses = request_responses_lock.write().unwrap();
                request_responses.insert(
                    (connection_id, api_message.message_id),
                    api_message,
                );

                if let Some(waker) = option_waker {
                    waker.wake();
                }
            }
            _ => {

		//
		// TODO - we do not need to fetch every peer mutably just to answer
		// and deal with inbound requests.
		//
                //let peers_db_global = PEERS_DB_GLOBAL.clone();
                //let mut peer_db = peers_db_global.write().await;
                //let peer = peer_db.get_mut(&connection_id).unwrap();

        	let command = api_message.get_message_name_as_string();
        	match command.as_str() {
        	    "REQBLOCK" => {
            	    }
		    _ => {
                	error!(
                	    "Unhandled command received by client... {}",
                	    &api_message.get_message_name_as_string()
                	);
 	               //peer.send_error_response_from_str(api_message.message_id, "NO SUCH").await;
         	   }
        	}
            }
        }
    }





    //
    // Sends an APIMessage to another peer through a socket connection. Since Outbound and Inbound peers 
    // Streams(Sinks) are not unified into a single Trait yet, we have to check both databases to find out
    // which sort of sink this peer is using and send the message through the appropriate stream.
    //
    pub async fn send_request(api_message: APIMessage, connection_id: &SaitoHash) {

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
    // Sends an API message to another peer through a socket connection, while handling the process through
    // a PeerRequest that allows us to .await the response and continue execution when and if the remote 
    // peer responds to the message.
    //
    pub async fn send_request_with_callback(api_message: APIMessage, connection_id: &SaitoHash) {
      //
      // to be implemented
      //
    }


    /// Connect to a peer via websocket and spawn a Task to handle message received on the socket
    /// and pipe them to handle_peer_message().
    async fn add_peer(connection_id: SaitoHash, wallet_lock: Arc<RwLock<Wallet>>) {

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





    pub async fn add_remote_peer(
        ws: WebSocket,
        network_lock: Arc<RwLock<Network>>,
    ) {

        let (peer_ws_sender, mut peer_ws_rcv) = ws.split();
        let (peer_sender, peer_rcv) = mpsc::unbounded_channel();
        let peer_rcv = UnboundedReceiverStream::new(peer_rcv);
        tokio::task::spawn(peer_rcv.forward(peer_ws_sender).map(|result| {
            if let Err(e) = result {
                error!("error sending websocket msg: {}", e);
            }
        }));

        let connection_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
        let mut peer = Peer::new(
            connection_id,
            None,
            None,
        );
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
    // fetch block from network
    //
    async fn fetch_block(block_hash: SaitoHash) {
/***

***/
    }


    //
    // send block to all peers
    //
    async fn propagate_block(block_hash: SaitoHash) {
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

    pub async fn propagate_transaction(wallet_lock: Arc<RwLock<Wallet>>, mut tx: Transaction) {
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




/***********
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
    /// sends a command to the peer's socket but doesn't not await the response
    pub async fn send_command_fire_and_forget(&mut self, command: &str, message: Vec<u8>) {
        // Create a PeerRequest(Future), but do not await it.
        let _peer_request = PeerRequest::new(command, message, self).await;
        // TODO: low priority. Ensure that commands sent this way are actually cleaned from PEERS_REQUEST_RESPONSES_GLOBAL and PEERS_REQUEST_WAKERS_GLOBAL.
        //       I'm quite sure this isn't a problem, but did not confirm.
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

********/


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
                                Network::add_peer(connection_id, wallet_lock_clone).await;
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
            .or(ws_upgrade_route_filter(
                network_lock_clone.clone(),
            ));

        info!("Listening for HTTP on port {}", port);
        let (_, server) =
            warp::serve(routes).bind_with_graceful_shutdown((host, port), signal_for_shutdown());
        server.await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    // use crate::hop::Hop;
    // use crate::slip::Slip;
    // use crate::transaction::TransactionType;
    use crate::peer::{
        INBOUND_PEER_CONNECTIONS_GLOBAL, PEERS_REQUEST_RESPONSES_GLOBAL,
        PEERS_REQUEST_WAKERS_GLOBAL,
    };
    use crate::transaction::Transaction;
    use crate::{
        block::{Block, BlockType},
        crypto::{generate_keys, hash, sign_blob, verify, SaitoSignature},
        mempool::Mempool,
        networking::{
            api_message::APIMessage,
            filters::ws_upgrade_route_filter,
            message_types::{
                handshake_challenge::HandshakeChallenge,
                request_block_message::RequestBlockMessage,
                send_block_head_message::SendBlockHeadMessage,
                send_blockchain_message::{
                    SendBlockchainBlockData, SendBlockchainMessage, SyncType,
                },
            },
            peer::{
                INBOUND_PEER_CONNECTIONS_GLOBAL, OUTBOUND_PEER_CONNECTIONS_GLOBAL, PEERS_DB_GLOBAL,
                PEERS_REQUEST_RESPONSES_GLOBAL, PEERS_REQUEST_WAKERS_GLOBAL,
            },
        },
        test_utilities::test_manager::TestManager,
        time::create_timestamp,
    };
    use secp256k1::PublicKey;
    use warp::{test::WsClient, ws::Message};

    /// This doesn't currently seem to create a problem, but I think
    async fn clean_peers_dbs() {
        let peers_db_global = PEERS_DB_GLOBAL.clone();
        let mut peer_db = peers_db_global.write().await;

        let request_responses_lock = PEERS_REQUEST_RESPONSES_GLOBAL.clone();
        let mut request_responses = request_responses_lock.write().unwrap();

        let request_wakers_lock = PEERS_REQUEST_WAKERS_GLOBAL.clone();
        let mut request_wakers = request_wakers_lock.write().unwrap();

        let outbound_peer_connection_db_global = OUTBOUND_PEER_CONNECTIONS_GLOBAL.clone();
        let mut outbound_peer_connection_db = outbound_peer_connection_db_global.write().await;
        let inbound_peer_connection_db_global = INBOUND_PEER_CONNECTIONS_GLOBAL.clone();
        let mut inbound_peer_connection_db = inbound_peer_connection_db_global.write().await;

        peer_db.drain();
        request_responses.drain();
        request_wakers.drain();
        outbound_peer_connection_db.drain();
        inbound_peer_connection_db.drain();
    }

    /// This function will be used in mosts test of network, it will open a socket, negotiate a handshake,
    /// and return the socket so we are ready to start sending APIMessages through the socket, which
    /// we can use as a mock peer.
    async fn create_socket_and_do_handshake(
        wallet_arc: Arc<RwLock<Wallet>>,
        mempool_arc: Arc<RwLock<Mempool>>,
        blockchain_arc: Arc<RwLock<Blockchain>>,
        broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    ) -> WsClient {
        // mock things:
        let (publickey, privatekey) = generate_keys();

        // use Warp test to open a socket:
        let socket_filter = ws_upgrade_route_filter(
            wallet_arc,
            mempool_arc,
            blockchain_arc,
            broadcast_channel_sender,
        );
        let mut ws_client = warp::test::ws()
            .path("/wsopen")
            .handshake(socket_filter)
            .await
            .expect("handshake");

        // create a SHAKINIT message
        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );
        let api_message = APIMessage::new("SHAKINIT", 42, message_data);

        // send SHAKINIT through the socket
        ws_client
            .send(Message::binary(api_message.serialize()))
            .await;

        // read a message from the socket, it should be a RESULT__ with the same id as our SHAKINIT command
        let resp = ws_client.recv().await.unwrap();
        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());
        assert_eq!(command, "RESULT__");
        assert_eq!(index, 42);

        // deserialize the HandshakeChallenge message:
        let deserialize_challenge =
            HandshakeChallenge::deserialize(&resp.as_bytes()[12..].to_vec());
        let raw_challenge: [u8; CHALLENGE_SIZE] =
            resp.as_bytes()[12..][..CHALLENGE_SIZE].try_into().unwrap();
        let sig: SaitoSignature = resp.as_bytes()[12..][CHALLENGE_SIZE..CHALLENGE_SIZE + 64]
            .try_into()
            .unwrap();

        // confirm the HandshakeChallenge has all the right data and the signature is correct:
        assert_eq!(
            deserialize_challenge.challenger_ip_address(),
            [127, 0, 0, 1]
        );
        assert_eq!(deserialize_challenge.opponent_ip_address(), [127, 0, 0, 1]);
        assert_eq!(deserialize_challenge.opponent_pubkey(), publickey);
        assert!(verify(
            &hash(&raw_challenge.to_vec()),
            sig,
            deserialize_challenge.challenger_pubkey(),
        ));

        // sign the raw challenge and create a SHAKCOMP message from it
        let signed_challenge =
            sign_blob(&mut resp.as_bytes()[12..].to_vec(), privatekey).to_owned();
        let api_message = APIMessage::new("SHAKCOMP", 43, signed_challenge);

        // send SHAKCOMP through the socket
        ws_client
            .send(Message::binary(api_message.serialize()))
            .await;

        // read a message from the socket and confirm that the RESULT__ is OK
        let resp = ws_client.recv().await.unwrap();
        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());
        let msg = String::from_utf8_lossy(&resp.as_bytes()[12..]);
        assert_eq!(command, "RESULT__");
        assert_eq!(index, 43);
        assert_eq!(msg, "OK");

        ws_client
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_sndchain() {
        // mock things:
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, _broadcast_channel_receiver) = broadcast::channel(32);
        // create a mock peer/socket:
        clean_peers_dbs().await;
        let mut ws_client = create_socket_and_do_handshake(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
        )
            .await;

        // Build a SNDCHAIN request
        let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
        blocks_data.push(SendBlockchainBlockData {
            block_id: 1,
            block_hash: [1; 32],
            timestamp: 0,
            pre_hash: [0; 32],
            number_of_transactions: 0,
        });
        blocks_data.push(SendBlockchainBlockData {
            block_id: 2,
            block_hash: [2; 32],
            timestamp: 1,
            pre_hash: [1; 32],
            number_of_transactions: 0,
        });
        let send_chain_message = SendBlockchainMessage::new(SyncType::Full, [0; 32], blocks_data);
        let api_message = APIMessage::new("SNDCHAIN", 12345, send_chain_message.serialize());
        // send SNDCHAIN request
        ws_client
            .send(Message::binary(api_message.serialize()))
            .await;
        let resp = ws_client.recv().await.unwrap();

        let api_message_resp = APIMessage::deserialize(&resp.as_bytes().to_vec());

        assert_eq!(
            api_message_resp.get_message_name_as_string(),
            String::from("RESULT__")
        );
        assert_eq!(api_message_resp.get_message_id(), 12345);
        assert_eq!(
            api_message_resp.get_message_data_as_string(),
            String::from("OK")
        );
        // After "OK", we expect the peer to start doing REQBLOCK requests
        // Read the next message from the socket... should be REQBLOCK
        let resp = ws_client.recv().await.unwrap();
        let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
        assert_eq!(api_message_request.get_message_name_as_string(), "REQBLOCK");
        let request_block_request =
            RequestBlockMessage::deserialize(api_message_request.get_message_data());
        assert_eq!(request_block_request.get_block_hash().unwrap(), [1; 32]);

        // Send a mock response to the first REQBLOCK request
        let block = Block::new();
        let request_block_response = APIMessage::new(
            "RESULT__",
            api_message_request.get_message_id(),
            block.serialize_for_net(BlockType::Full),
        );
        // We should get another REQBLOCK request for the 2nd block in SNDCHAIN
        ws_client
            .send(Message::binary(request_block_response.serialize()))
            .await;

        let resp = ws_client.recv().await.unwrap();

        let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
        assert_eq!(api_message_request.get_message_name_as_string(), "REQBLOCK");
        let request_block_request =
            RequestBlockMessage::deserialize(api_message_request.get_message_data());
        assert_eq!(request_block_request.get_block_hash().unwrap(), [2; 32]);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_sndblkhd() {
        // mock things:
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, _broadcast_channel_receiver) = broadcast::channel(32);
        // create a mock peer/socket:
        clean_peers_dbs().await;
        let mut ws_client = create_socket_and_do_handshake(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
        )
            .await;

        // create a SNDBLKHD message
        let mock_hash = [3; 32];
        let send_chain_message = SendBlockHeadMessage::new(mock_hash);
        let api_message = APIMessage::new("SNDBLKHD", 12345, send_chain_message.serialize());

        // send SNDBLKHD message through the socket
        ws_client
            .send(Message::binary(api_message.serialize()))
            .await;

        // read a message off the socket, it should be a RESULT__ for the SNDBLKHD message
        let resp = ws_client.recv().await.unwrap();
        let api_message_response = APIMessage::deserialize(&resp.as_bytes().to_vec());
        assert_eq!(
            api_message_response.get_message_name_as_string(),
            String::from("RESULT__")
        );
        assert_eq!(api_message_response.get_message_id(), 12345);
        assert_eq!(
            api_message_response.get_message_data_as_string(),
            String::from("OK")
        );

        // read another message from the socket, this should be a REQBLOCK command with the hash
        // we sent with SNDBLKHD
        let resp = ws_client.recv().await.unwrap();
        let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
        assert_eq!(
            api_message_request.get_message_name_as_string(),
            String::from("REQBLOCK")
        );
        let request_block_request =
            RequestBlockMessage::deserialize(api_message_request.get_message_data());
        assert_eq!(request_block_request.get_block_hash().unwrap(), [3; 32]);
    }

    #[tokio::test]
    async fn missing_blocks_test() {
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, mut broadcast_channel_receiver) = broadcast::channel(32);

        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());

        let current_timestamp = create_timestamp();

        // BLOCK 1
        test_manager
            .add_block(current_timestamp, 3, 0, false, vec![])
            .await;

        // BLOCK 2
        test_manager
            .add_block(current_timestamp + 120000, 0, 1, false, vec![])
            .await;

        // BLOCK 3
        test_manager
            .add_block(current_timestamp + 240000, 0, 1, false, vec![])
            .await;

        // BLOCK 4
        test_manager
            .add_block(current_timestamp + 360000, 0, 1, false, vec![])
            .await;

        // BLOCK 5
        test_manager
            .add_block(current_timestamp + 480000, 0, 1, false, vec![])
            .await;
        {
            let blockchain = blockchain_lock.read().await;

            assert_eq!(5, blockchain.get_latest_block_id());
        }

        let publickey;
        let privatekey;
        {
            let wallet = wallet_lock.read().await;
            publickey = wallet.get_publickey();
            privatekey = wallet.get_privatekey();
        }

        let mut block_with_unknown_parent = Block::new();
        block_with_unknown_parent.set_id(4);
        block_with_unknown_parent.set_previous_block_hash([2; 32]);
        block_with_unknown_parent.set_burnfee(10);
        block_with_unknown_parent.set_timestamp(create_timestamp());

        let mut tx =
            Transaction::generate_vip_transaction(wallet_lock.clone(), publickey, 10_000_000, 1)
                .await;
        tx.generate_metadata(publickey);

        tx.sign(privatekey);

        block_with_unknown_parent.set_transactions(&mut vec![tx]);

        let block_merkle_root = block_with_unknown_parent.generate_merkle_root();
        block_with_unknown_parent.set_merkle_root(block_merkle_root);
        block_with_unknown_parent.sign(publickey, privatekey);

        block_with_unknown_parent.set_source_connection_id([5; 32]);

        // connect a peer
        // let mut ws_client = create_socket_and_do_handshake(
        //     wallet_lock.clone(),
        //     mempool_lock.clone(),
        //     blockchain_lock.clone(),
        //     broadcast_channel_sender.clone(),
        // )
        // .await;

        let block_with_unknown_parent_hash = block_with_unknown_parent.get_hash().clone();
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.set_broadcast_channel_sender(broadcast_channel_sender.clone());
            blockchain.add_block(block_with_unknown_parent).await;
            let block = blockchain.get_block(&block_with_unknown_parent_hash).await;
            println!(
                "is_some?> {:?}",
                &hex::encode(&block_with_unknown_parent_hash)
            );
            assert!(block.is_some());
        }
        if let Ok(msg) = broadcast_channel_receiver.recv().await {
            match msg {
                SaitoMessage::MissingBlock {
                    peer_id: connection_id,
                    hash: _block_hash,
                } => {
                    assert_eq!(connection_id, [5; 32]);
                }
                _ => {
                    assert!(false, "message should be MissingBlock");
                }
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_blockchain_causes_sndblkhd() {
        // initialize peers db:
        clean_peers_dbs().await;
        // mock things:
        let mut settings = config::Config::default();
        settings.set("network.host", vec![127, 0, 0, 1]).unwrap();
        settings.set("network.port", 3002).unwrap();
        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);

        // TODO
        // This should be in the blockchain constructor.
        // Normally this is done in Network::run, but we need to set the broadcast_channel_sender here.
        {
            let mut blockchain = blockchain_lock.write().await;
            blockchain.set_broadcast_channel_sender(broadcast_channel_sender.clone());
        }

        // connect a peer
        let mut ws_client = create_socket_and_do_handshake(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
        )
            .await;

        // Start the network listening for messages on the global broadcast channel.
        // TODO All these things should also perhaps be passed to the network constructor
        let wallet_lock_for_task = wallet_lock.clone();
        let mempool_lock_for_task = mempool_lock.clone();
        let blockchain_lock_for_task = blockchain_lock.clone();
        tokio::spawn(async move {
            crate::networking::network::run(
                settings,
                wallet_lock_for_task,
                mempool_lock_for_task,
                blockchain_lock_for_task,
                broadcast_channel_sender,
                broadcast_channel_receiver,
            )
                .await
                .unwrap();
        });

        // make a block add add it to blockchain
        let mut test_manager = TestManager::new(blockchain_lock.clone(), wallet_lock.clone());
        let current_timestamp = create_timestamp();
        test_manager
            .add_block(current_timestamp, 3, 0, false, vec![])
            .await;

        let blockchain = blockchain_lock.read().await;
        assert_eq!(1, blockchain.get_latest_block_id());

        // during add_block the blockchain should send a BlockchainSavedBlock message to the network which should cause
        // a SNDBLKHD message to be sent to every peer that has completed handshake.
        let resp = ws_client.recv().await.unwrap();
        let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
        assert_eq!(
            api_message_request.get_message_name_as_string(),
            String::from("SNDBLKHD")
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_network_message_sending() {
        // mock things:
        let mut settings = config::Config::default();
        settings.set("network.host", vec![127, 0, 0, 1]).unwrap();
        settings.set("network.port", 3002).unwrap();

        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
        let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);
        // connect a peer to the client
        clean_peers_dbs().await;
        let mut ws_client = create_socket_and_do_handshake(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
            broadcast_channel_sender.clone(),
        )
            .await;
        // network object under test:
        let bcs_clone = broadcast_channel_sender.clone();
        tokio::spawn(async move {
            crate::networking::network::run(
                settings,
                wallet_lock.clone(),
                mempool_lock.clone(),
                blockchain_lock.clone(),
                bcs_clone,
                broadcast_channel_receiver,
            )
                .await
                .unwrap();
        });
        // send 2 message to network:
        tokio::spawn(async move {
            broadcast_channel_sender
                .send(SaitoMessage::BlockchainSavedBlock { hash: [0; 32] })
                .expect("error: BlockchainAddBlockFailure message failed to send");
            broadcast_channel_sender
                .send(SaitoMessage::BlockchainSavedBlock { hash: [0; 32] })
                .expect("error: BlockchainAddBlockFailure message failed to send");
        });
        // These messages should prompt SNDBLKHD commands to each peer
        for _i in 0..2 {
            let resp = ws_client.recv().await.unwrap();
            let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
            assert_eq!(
                api_message_request.get_message_name_as_string(),
                String::from("SNDBLKHD")
            );
        }
    }

    //////// TEST SNDTRANS ////////
    // TODO: currently the main logic "test sndtrans to peers" passed. But there is no way to get
    // tx to be validated & send it to peer in the test. We may figured out how to get tx validation
    // later in a test. And we may move all integration tests out of the main codebase &
    // Since the integration tests should spin up the app as a whole, it should probably live in `tests/`
    // We will need to mock services and create fake DBs for testing (for e.g.),
    // #[tokio::test]
    // #[serial_test::serial]
    // async fn test_sndtrans() {
    //     // mock things:
    //     let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
    //     let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    //     let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    //     // create a mock peer/socket:
    //     clean_peers_dbs().await;
    //
    //     let wallet = wallet_lock.read().await;
    //
    //     let mut ws_client = create_socket_and_do_handshake(
    //         wallet_lock.clone(),
    //         mempool_lock.clone(),
    //         blockchain_lock.clone(),
    //     )
    //     .await;
    //     // create a SNDTRANS message
    //     let mock_input = Slip::new();
    //     let mock_output = Slip::new();
    //     let mut mock_hop = Hop::new();
    //     mock_hop.set_from([0; 33]);
    //     mock_hop.set_to([0; 33]);
    //     mock_hop.set_sig([0; 64]);
    //     let mut mock_tx = Transaction::new();
    //     let mut mock_path: Vec<Hop> = vec![];
    //     mock_path.push(mock_hop);
    //     let ctimestamp = create_timestamp();
    //
    //     mock_tx.set_timestamp(ctimestamp);
    //     mock_tx.add_input(mock_input);
    //     mock_tx.add_output(mock_output);
    //     mock_tx.set_message(vec![104, 101, 108, 108, 111]);
    //     mock_tx.set_transaction_type(TransactionType::Normal);
    //     mock_tx.set_signature([1; 64]);
    //     mock_tx.set_path(mock_path);
    //
    //     let serialized_tx = mock_tx.serialize_for_net();
    //     let api_message = APIMessage::new("SNDTRANS", 67890, serialized_tx);
    //
    //     // send SNDTRANS message through the socket
    //     ws_client
    //         .send(Message::binary(api_message.serialize()))
    //         .await;
    //
    //     // read a message off the socket, it should be a RESULT__ for the SNDTRANS message
    //     let resp = ws_client.recv().await.unwrap();
    //     let api_message_response = APIMessage::deserialize(&resp.as_bytes().to_vec());
    //     assert_eq!(
    //         api_message_response.get_message_name_as_string(),
    //         String::from("RESULT__")
    //     );
    //     assert_eq!(api_message_response.get_message_id(), 67890);
    //     assert_eq!(
    //         api_message_response.get_message_data_as_string(),
    //         String::from("OK")
    //     );
    // }

    // #[tokio::test]
    // #[serial_test::serial]
    // async fn test_network_sndtrans() {
    //     // mock things:
    //     let mut settings = config::Config::default();
    //     settings.set("network.host", vec![127, 0, 0, 1]).unwrap();
    //     settings.set("network.port", 3002).unwrap();
    //
    //     let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
    //     let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    //     let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    //     let (broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);
    //     // connect a peer to the client
    //     clean_peers_dbs().await;
    //     let mut ws_client = create_socket_and_do_handshake(
    //         wallet_lock.clone(),
    //         mempool_lock.clone(),
    //         blockchain_lock.clone(),
    //     )
    //     .await;
    //
    //     // network object under test:
    //     tokio::spawn(async move {
    //         crate::networking::network::run(
    //             settings,
    //             wallet_lock.clone(),
    //             mempool_lock.clone(),
    //             blockchain_lock.clone(),
    //             broadcast_channel_receiver,
    //         )
    //         .await
    //         .unwrap();
    //     });
    //
    //     let mock_input = Slip::new();
    //     let mock_output = Slip::new();
    //     let mut mock_hop = Hop::new();
    //     mock_hop.set_from([0; 33]);
    //     mock_hop.set_to([0; 33]);
    //     mock_hop.set_sig([0; 64]);
    //     let mut mock_tx = Transaction::new();
    //     let mut mock_path: Vec<Hop> = vec![];
    //     mock_path.push(mock_hop);
    //     let ctimestamp = create_timestamp();
    //
    //     mock_tx.set_timestamp(ctimestamp);
    //     mock_tx.add_input(mock_input);
    //     mock_tx.add_output(mock_output);
    //     mock_tx.set_message(vec![104, 101, 108, 108, 111]);
    //     mock_tx.set_transaction_type(TransactionType::Normal);
    //     mock_tx.set_signature([1; 64]);
    //     mock_tx.set_path(mock_path);
    //
    //     let mut settings2 = config::Config::default();
    //     settings2.set("network.host", vec![127, 0, 0, 1]).unwrap();
    //     settings2.set("network.port", 3003).unwrap();
    //     settings2
    //         .set("network.peers.host", vec![127, 0, 0, 1])
    //         .unwrap();
    //     settings2.set("network.peers.port", 3002).unwrap();
    //
    //     let wallet_lock2 = Arc::new(RwLock::new(Wallet::new()));
    //     let mempool_lock2 = Arc::new(RwLock::new(Mempool::new(wallet_lock2.clone())));
    //     let blockchain_lock2 = Arc::new(RwLock::new(Blockchain::new(wallet_lock2.clone())));
    //     let (_broadcast_channel_sender, broadcast_channel_receiver) = broadcast::channel(32);
    //     let mut ws_client2 = create_socket_and_do_handshake(
    //         wallet_lock2.clone(),
    //         mempool_lock2.clone(),
    //         blockchain_lock2.clone(),
    //     )
    //     .await;
    //
    //     // network object under test:
    //     tokio::spawn(async move {
    //         crate::networking::network::run(
    //             settings2,
    //             wallet_lock2.clone(),
    //             mempool_lock2.clone(),
    //             blockchain_lock2.clone(),
    //             broadcast_channel_receiver,
    //         )
    //         .await
    //         .unwrap();
    //     });
    //
    //     // send message to network:
    //     tokio::spawn(async move {
    //         broadcast_channel_sender
    //             .send(SaitoMessage::WalletNewTransaction {
    //                 transaction: mock_tx,
    //             })
    //             .expect("error: WalletNewTransaction message failed to send");
    //     });
    //
    //     let resp = ws_client.recv().await.unwrap();
    //     let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
    //     assert_eq!(
    //         api_message_request.get_message_name_as_string(),
    //         String::from("SNDTRANS")
    //     );
    //
    //     let resp = ws_client2.recv().await.unwrap();
    //     let api_message_request = APIMessage::deserialize(&resp.as_bytes().to_vec());
    //     assert_eq!(
    //         api_message_request.get_message_name_as_string(),
    //         String::from("SNDTRANS")
    //     );
    // }

    // #[tokio::test]
    // #[serial_test::serial]
    // async fn test_peer_sndtrans() {
    //     // mock things:
    //     let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
    //     let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
    //     let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));
    //     // create a mock peer/socket:
    //     clean_peers_dbs().await;
    //
    //     let mut ws_client = create_socket_and_do_handshake(
    //         wallet_lock.clone(),
    //         mempool_lock.clone(),
    //         blockchain_lock.clone(),
    //     )
    //     .await;
    //
    //     let wallet = wallet_lock.read().await;
    //
    //     // create a SNDTRANS message by creating a mock tx
    //     let mock_input = Slip::new();
    //     let mock_output = Slip::new();
    //     let mut mock_hop = Hop::new();
    //     mock_hop.set_from(wallet.get_publickey());
    //     mock_hop.set_to([0; 33]);
    //     mock_hop.set_sig([0; 64]);
    //     let mut mock_tx = Transaction::new();
    //     let mut mock_path: Vec<Hop> = vec![];
    //     mock_path.push(mock_hop);
    //     let ctimestamp = create_timestamp();
    //
    //     mock_tx.set_timestamp(ctimestamp);
    //     mock_tx.add_input(mock_input);
    //     mock_tx.add_output(mock_output);
    //     mock_tx.set_message(vec![104, 101, 108, 108, 111]);
    //     mock_tx.set_transaction_type(TransactionType::Normal);
    //     mock_tx.set_signature([1; 64]);
    //     // mock_tx.generate_metadata(wallet.get_publickey());
    //     // mock_tx.sign(wallet.get_privatekey());
    //     mock_tx.set_path(mock_path);
    //
    //     let serialized_tx = mock_tx.serialize_for_net();
    //     let api_message = APIMessage::new("SNDTRANS", 67890, serialized_tx);
    //
    //     // create 2nd mock peer/socket
    //     let wallet_lock2 = Arc::new(RwLock::new(Wallet::new()));
    //     let mempool_lock2 = Arc::new(RwLock::new(Mempool::new(wallet_lock2.clone())));
    //     let blockchain_lock2 = Arc::new(RwLock::new(Blockchain::new(wallet_lock2.clone())));
    //
    //     let mut ws_client2 = create_socket_and_do_handshake(
    //         wallet_lock2.clone(),
    //         mempool_lock2.clone(),
    //         blockchain_lock2.clone(),
    //     )
    //     .await;
    //
    //     // send SNDTRANS message through the socket - send it to peers
    //     // & check what we receive from ws_client & ws_client2 are the same
    //     ws_client
    //         .send(Message::binary(api_message.serialize()))
    //         .await;
    //
    //     // read a message off the socket, it should be a RESULT__ for the SNDTRANS message
    //     let resp = ws_client.recv().await.unwrap();
    //     let api_message_response = APIMessage::deserialize(&resp.as_bytes().to_vec());
    //     assert_eq!(
    //         api_message_response.get_message_name_as_string(),
    //         String::from("RESULT__")
    //     );
    //     assert_eq!(api_message_response.get_message_id(), 67890);
    //     assert_eq!(
    //         api_message_response.get_message_data_as_string(),
    //         String::from("OK")
    //     );
    //
    //     // read a message off the 2nd socket, it should be a SNDTRANS message
    //     let resp2 = ws_client2.recv().await.unwrap();
    //     let api_message_response2 = APIMessage::deserialize(&resp2.as_bytes().to_vec());
    //     assert_eq!(
    //         api_message_response2.get_message_name_as_string(),
    //         String::from("SNDTRANS")
    //     );
    //     assert_eq!(api_message_response2.get_message_id(), 0);
    //
    //     // deserialize the message data & check sig
    //     let tx2 = Transaction::deserialize_from_net(api_message_response2.get_into_message_data());
    //     assert_eq!(mock_tx.get_signature(), tx2.get_signature());
    // }
}
