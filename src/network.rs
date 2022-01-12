use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{hash, sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::networking::filters::{
    get_block_route_filter, post_transaction_route_filter, ws_upgrade_route_filter,
};
use crate::networking::peer::{
    socket_handshake_verify, OutboundPeer, SaitoPeer, OUTBOUND_PEER_CONNECTIONS_GLOBAL,
    PEERS_DB_GLOBAL,
};
use crate::peer::Peer;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use log::{error, info, warn};
use secp256k1::PublicKey;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;

use futures::StreamExt;
use std::iter::Iterator;
use uuid::Uuid;
use warp::{Filter, Rejection};

//use super::message_types::send_block_head_message::SendBlockHeadMessage;
use crate::networking::signals::signal_for_shutdown;

use crate::configuration::{PeerSetting, Settings};
use crate::networking::api_message::APIMessage;
use crate::networking::message_types::request_blockchain_message::RequestBlockchainMessage;
use crate::networking::message_types::send_block_head_message::SendBlockHeadMessage;
use crate::util::format_url_string;

pub type Result<T> = std::result::Result<T, Rejection>;

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

pub struct Network {
    blockchain_lock: Arc<RwLock<Blockchain>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
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
    ) -> Network {
        Network {
            host: configuration.network.host,
            port: configuration.network.port,
            peer_conf: configuration.network.peers,
            blockchain_lock,
            mempool_lock,
            wallet_lock,
            broadcast_channel_sender: None,
        }
    }

    fn set_bcs(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Option::from(bcs);
    }

    /// Runs warp::serve to listen for incoming connections
    async fn run_server(&self) -> crate::Result<()> {
        let routes = get_block_route_filter(self.blockchain_lock.clone())
            .or(post_transaction_route_filter(
                self.mempool_lock.clone(),
                self.blockchain_lock.clone(),
            ))
            .or(ws_upgrade_route_filter(
                self.wallet_lock.clone(),
                self.mempool_lock.clone(),
                self.blockchain_lock.clone(),
                self.broadcast_channel_sender.clone(),
            ));
        info!("Listening for HTTP on port {}", self.port);
        let (_, server) = warp::serve(routes)
            .bind_with_graceful_shutdown((self.host, self.port), signal_for_shutdown());
        server.await;
        Ok(())
    }

    /// connects to any peers configured in our peers list.
    /// Opens a socket, does handshake, synchronizes, etc.
    async fn run_client(&self) -> crate::Result<()> {
        self.initialize_configured_peers().await;
        self.spawn_reconnect_to_configured_peers_task(self.wallet_lock.clone())
            .await
            .unwrap();
        Ok(())
    }

    /// Initialize configured peers (peers set in the configuration/*.yml) if any.
    /// This does not connect to the peers, it only sets their
    /// state and inserts them into PEERS_DB_GLOBAL so that the Task created by
    /// spawn_reconnect_to_configured_peers_task will open the connection.
    async fn initialize_configured_peers(&self) {
        if let Some(peer_settings) = &self.peer_conf {
            for peer_setting in peer_settings {
                let connection_id: SaitoHash = hash(&Uuid::new_v4().as_bytes().to_vec());
                let peer = SaitoPeer::new(
                    connection_id,
                    Some(peer_setting.host),
                    Some(peer_setting.port),
                    false,
                    false,
                    true,
                    self.wallet_lock.clone(),
                    self.mempool_lock.clone(),
                    self.blockchain_lock.clone(),
                    self.broadcast_channel_sender.clone(),
                );
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

    /// Launch a task which will monitor peers and make sure they stay connected. If a peer in our
    /// configured "peers list" becomes disconnected, this task will reconnect to the peer and
    /// redo the handshake and blockchain synchronization. For convenience, this task is also
    /// used to make initial connections with peers(not only to reconnect).
    async fn spawn_reconnect_to_configured_peers_task(
        &self,
        wallet_lock: Arc<RwLock<Wallet>>,
    ) -> crate::Result<()> {
        tokio::spawn(async move {
            loop {
                let peer_states: Vec<(SaitoHash, bool)>;
                {
                    let peers_db_global = PEERS_DB_GLOBAL.clone();
                    let peers_db = peers_db_global.read().await;
                    peer_states = peers_db
                        .keys()
                        .map(|connection_id| {
                            let peer = peers_db.get(connection_id).unwrap();
                            let should_try_reconnect = peer.get_is_from_peer_list()
                                && !peer.get_is_connected_or_connecting();
                            (*connection_id, should_try_reconnect)
                        })
                        .collect::<Vec<(SaitoHash, bool)>>();
                }
                for (connection_id, should_try_reconnect) in peer_states {
                    if should_try_reconnect {
                        info!("found disconnected peer in peer settings, (re)connecting...");
                        Network::connect_to_peer(connection_id, wallet_lock.clone()).await;
                    }
                }
                sleep(Duration::from_millis(1000)).await;
            }
        })
        .await
        .expect("error: spawn_reconnect_to_configured_peers_task failed");
        Ok(())
    }

    /// listening for message from the rest of the codebase
    pub async fn listen_for_messages(
        &self,
        mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
    ) {
        loop {
            while let Ok(message) = broadcast_channel_receiver.recv().await {
                match message {
                    SaitoMessage::MinerNewGoldenTicket {
                        ticket: _golden_ticket,
                    } => {
                        // TODO implement this...
                        println!("MinerNewGoldenTicket");
                    }
                    SaitoMessage::BlockchainSavedBlock { hash: block_hash } => {
                        warn!("SaitoMessage::BlockchainSavedBlock recv'ed by network");
                        Network::send_my_block_to_peers(block_hash).await;
                    }
                    SaitoMessage::WalletNewTransaction { transaction: tx } => {
                        info!("SaitoMessage::WalletNewTransaction new tx is detected by network");
                        Network::propagate_transaction_to_peers(self.wallet_lock.clone(), tx).await;
                    }
                    SaitoMessage::MissingBlock {
                        peer_id: connection_id,
                        hash: block_hash,
                    } => {
                        warn!("SaitoMessage::BlockchainSavedBlock recv'ed by network");
                        let peers_db_global = PEERS_DB_GLOBAL.clone();
                        let mut peer_db = peers_db_global.write().await;
                        if let Some(peer) = peer_db.get_mut(&connection_id) {
                            peer.do_reqblock(block_hash).await;
                        } else {
                            error!("missing peer cannot be fetched, unknown Peer")
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Connect to a peer via websocket and spawn a Task to handle message received on the socket
    /// and pipe them to handle_peer_message().
    async fn connect_to_peer(connection_id: SaitoHash, wallet_lock: Arc<RwLock<Wallet>>) {
        let peers_db_global = PEERS_DB_GLOBAL.clone();
        let peer_url;
        {
            let mut peer_db = peers_db_global.write().await;
            let peer = peer_db.get_mut(&connection_id).unwrap();
            peer_url = url::Url::parse(&format!(
                "ws://{}/wsopen",
                format_url_string(peer.get_host().unwrap(), peer.get_port().unwrap()),
            ))
            .unwrap();
            peer.set_is_connected_or_connecting(true).await;
        }

        let ws_stream_result = connect_async(peer_url).await;
        match ws_stream_result {
            Ok((ws_stream, _)) => {
                let (write_sink, mut read_stream) = ws_stream.split();
                {
                    let outbound_peer_db_global = OUTBOUND_PEER_CONNECTIONS_GLOBAL.clone();
                    outbound_peer_db_global
                        .write()
                        .await
                        .insert(connection_id, OutboundPeer { write_sink });
                }

                tokio::spawn(async move {
                    while let Some(result) = read_stream.next().await {
                        match result {
                            Ok(message) => {
                                if !message.is_empty() {
                                    let api_message = APIMessage::deserialize(&message.into_data());
                                    SaitoPeer::handle_peer_message(api_message, connection_id)
                                        .await;
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
                                peer.set_is_connected_or_connecting(false).await;
                            }
                        }
                    }
                });
                Network::handshake_and_synchronize_chain(&connection_id, wallet_lock).await;
            }
            Err(error) => {
                error!("Error connecting to peer {:?}", error);
                let mut peer_db = peers_db_global.write().await;
                let peer = peer_db.get_mut(&connection_id).unwrap();
                peer.set_is_connected_or_connecting(false).await;
            }
        }
    }

    /// After socket has been connected, the connector begins the handshake via SHAKINIT command.
    /// Once the handshake is complete, we synchronize the peers via REQCHAIN/SENDCHAIN and REQBLOCK.
    pub async fn handshake_and_synchronize_chain(
        connection_id: &SaitoHash,
        wallet_lock: Arc<RwLock<Wallet>>,
    ) {
        {
            let publickey: SaitoPublicKey;
            {
                let wallet = wallet_lock.read().await;
                publickey = wallet.get_publickey();
            }
            let mut message_data = vec![127, 0, 0, 1];
            message_data.extend(
                PublicKey::from_slice(&publickey)
                    .unwrap()
                    .serialize()
                    .to_vec(),
            );

            let peers_db_global = PEERS_DB_GLOBAL.clone();
            let mut peer_db = peers_db_global.write().await;
            let peer = peer_db.get_mut(connection_id).unwrap();

            let response_api_message = peer
                .send_command(&String::from("SHAKINIT"), message_data)
                .await
                .unwrap();
            // We should sign the response and send a SHAKCOMP.
            // We want to reuse socket_handshake_verify, so we will sign before verifying the peer's signature
            let privatekey: SaitoPrivateKey;
            {
                let wallet = wallet_lock.read().await;
                privatekey = wallet.get_privatekey();
            }
            let signed_challenge =
                sign_blob(&mut response_api_message.message_data.to_vec(), privatekey).to_owned();
            match socket_handshake_verify(&signed_challenge) {
                Some(deserialize_challenge) => {
                    peer.set_has_completed_handshake(true);
                    peer.set_publickey(deserialize_challenge.challenger_pubkey());
                    let result = peer
                        .send_command(&String::from("SHAKCOMP"), signed_challenge)
                        .await;

                    if result.is_ok() {
                        let request_blockchain_message =
                            RequestBlockchainMessage::new(0, [0; 32], [42; 32]);
                        let _req_chain_result = peer
                            .send_command(
                                &String::from("REQCHAIN"),
                                request_blockchain_message.serialize(),
                            )
                            .await
                            .unwrap();
                        //
                        // TODO _req_chain_result will be an OK message. We could verify it here, but it's not very useful.
                        // However, if we are finding issues, it may be useful to retry if we don't receive an OK soon.
                        //
                        // It's a bit difficult overly complex because the state needs to be tracked by the peer between here and
                        // the receipt of the SNDCHAIN. I.E. we may receive an OK here, but not receive a REQCHAIN
                        // message later.
                        //
                        // A simpler solution may be to redesign the API so that the response
                        // is sent directly at this point, rather than as a seperate APIMessage.
                        //
                    } else {
                        // TODO delete the peer if there is an error here
                    }
                    info!("Handshake complete!");
                }
                None => {
                    error!("Error verifying peer handshake signature");
                }
            }
        }
    }

    /// For sending blocks made by mempool to all peers
    async fn send_my_block_to_peers(block_hash: SaitoHash) {
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
    }

    /// For transaction made by mempool to all peers
    pub async fn propagate_transaction_to_peers(
        wallet_lock: Arc<RwLock<Wallet>>,
        mut tx: Transaction,
    ) {
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
    }
}

pub async fn run(
    network_lock: Arc<RwLock<Network>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let mut network = network_lock.write().await;
    network.set_bcs(broadcast_channel_sender.clone());
    tokio::select! {
        res = network.run_client() => {
            if let Err(err) = res {
                eprintln!("run_client err {:?}", err)
            }
        },
        res = network.run_server() => {
            if let Err(err) = res {
                eprintln!("run_server err {:?}", err)
            }
        },
        () = network.listen_for_messages(broadcast_channel_receiver) => {

        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
