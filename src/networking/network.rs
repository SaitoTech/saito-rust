use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{hash, sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::networking::api_message::APIMessage;
use crate::networking::filters::{
    get_block_route_filter, post_transaction_route_filter, ws_upgrade_route_filter,
};
use crate::networking::message_types::request_blockchain_message::RequestBlockchainMessage;
use crate::networking::peer::{socket_handshake_verify, OutboundPeer, SaitoPeer};
use crate::util::format_url_string;

use crate::wallet::Wallet;
use futures::StreamExt;
use secp256k1::PublicKey;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;

use std::{sync::Arc, time::Duration};
use tokio_tungstenite::connect_async;
use tracing::{event, Level};
use uuid::Uuid;

use warp::{Filter, Rejection};

use super::peer::{PeerSetting, OUTBOUND_PEER_CONNECTIONS_GLOBAL, PEERS_DB_GLOBAL};

use config::Config;

pub const CHALLENGE_SIZE: usize = 82;
pub const CHALLENGE_EXPIRATION_TIME: u64 = 60000;

pub type Result<T> = std::result::Result<T, Rejection>;

/// Configuration for Network listener, port etc.
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
    config_settings: Config,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
}

pub async fn run(
    config_settings: Config,
    wallet_lock: Arc<RwLock<Wallet>>,
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
) -> crate::Result<()> {
    let network = Network::new(
        config_settings,
        wallet_lock.clone(),
        mempool_lock.clone(),
        blockchain_lock.clone(),
    );
    // TODO: refactor this into two separate classes maybe and split this run() into two...
    tokio::select! {
        res = network.run_client() => {
            if let Err(err) = res {
                eprintln!("network err {:?}", err)
            }
        },
        res = network.run_server() => {
            if let Err(err) = res {
                eprintln!("run_server err {:?}", err)
            }
        },
    }

    Ok(())
}

impl Network {
    /// Returns a Network
    /// # Arguments
    ///
    /// * `config_settings` - config::config::Config
    pub fn new(
        config_settings: Config,
        wallet_lock: Arc<RwLock<Wallet>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        blockchain_lock: Arc<RwLock<Blockchain>>,
    ) -> Network {
        Network {
            config_settings,
            wallet_lock,
            mempool_lock,
            blockchain_lock,
            broadcast_channel_sender: None,
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
                        // TODO IMMEDIATE
                        // let block = Block::deserialize_for_net(req_chain_result.message_data());
                        // peer.add_block_to_mempool(block).await;
                    } else {
                        // TODO delete the peer if there is an error here
                    }
                    event!(Level::INFO, "SHAKECOMPLETE!");
                }
                None => {
                    event!(Level::ERROR, "Error verifying peer handshake signature");
                }
            }
        }
    }
    /// Create an "outbound" connection to a peer
    // TODO move this to peer near to handle_inbound_peer_connection and give it a similar name.
    // these are the two functions which spawn tasks on either end of the socket.

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    pub async fn connect_to_peer(connection_id: SaitoHash, wallet_lock: Arc<RwLock<Wallet>>) {
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

                let _foo = tokio::spawn(async move {
                    while let Some(result) = read_stream.next().await {
                        match result {
                            Ok(message) => {
                                if !message.is_empty() {
                                    let api_message = APIMessage::deserialize(&message.into_data());
                                    SaitoPeer::handle_peer_message(api_message, connection_id)
                                        .await;
                                } else {
                                    event!(
                                        Level::ERROR,
                                        "Message of length 0... why?\n
                                        This seems to occur if we aren't holding a reference to the sender/stream on the\n
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
                                event!(Level::ERROR, "Error reading from peer socket {}", error);
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
                event!(Level::ERROR, "Error connecting to peer {}", error);
                let mut peer_db = peers_db_global.write().await;
                let peer = peer_db.get_mut(&connection_id).unwrap();
                peer.set_is_connected_or_connecting(false).await;
            }
        }
    }
    /// Load peers from the settings file into the peers db and initialized them appropriately.
    /// They will be disconnected and have not completed handshake and will be set as "from peer list".
    /// Peers which are "from peer list" will have their connection maintained(reconnected) in case
    /// there is a problem. The "reconnect loop"(spawn_reconnect_to_configured_peers_task) will
    /// automatically connect to these peers if we simply set them to disconnected here.
    pub async fn initialize_configured_peers(&self) {
        let peer_settings = match self
            .config_settings
            .get::<Vec<PeerSetting>>("network.peers")
        {
            Ok(peer_settings) => Some(peer_settings),
            Err(_) => None,
        };

        if let Some(peer_settings) = peer_settings {
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
    pub async fn spawn_reconnect_to_configured_peers_task(
        &self,
        wallet_lock: Arc<RwLock<Wallet>>,
    ) -> crate::Result<()> {
        let _foo = tokio::spawn(async move {
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
                        event!(
                            Level::INFO,
                            "found disconnected peer in peer settings, (re)connecting..."
                        );
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
    /// Runs warp::serve to listen for incoming connections
    pub async fn run_server(&self) -> crate::Result<()> {
        let host: [u8; 4] = self.config_settings.get::<[u8; 4]>("network.host").unwrap();
        let port: u16 = self.config_settings.get::<u16>("network.port").unwrap();

        let routes = get_block_route_filter()
            .or(post_transaction_route_filter(
                self.mempool_lock.clone(),
                self.blockchain_lock.clone(),
            ))
            .or(ws_upgrade_route_filter(
                self.wallet_lock.clone(),
                self.mempool_lock.clone(),
                self.blockchain_lock.clone(),
            ));
        println!("warp serve");
        warp::serve(routes).run((host, port)).await;
        Ok(())
    }
    /// connects to any peers configured in our peers list. Opens a socket, does handshake, sychronizes, etc.
    pub async fn run_client(&self) -> crate::Result<()> {
        self.initialize_configured_peers().await;
        let _foo = self
            .spawn_reconnect_to_configured_peers_task(self.wallet_lock.clone())
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::{
        crypto::{generate_keys, hash, sign_blob, verify, SaitoSignature},
        mempool::Mempool,
        networking::{
            api_message::APIMessage,
            filters::ws_upgrade_route_filter,
            message_types::{
                handshake_challenge::HandshakeChallenge,
                request_blockchain_message::RequestBlockchainMessage,
            },
        },
        transaction::Transaction,
    };
    use secp256k1::PublicKey;
    use warp::ws::Message;

    #[tokio::test]
    async fn test_message_serialize() {
        let api_message = APIMessage {
            message_name: String::from("HLLOWRLD").as_bytes().try_into().unwrap(),
            message_id: 1,
            message_data: String::from("SOMEDATA").as_bytes().try_into().unwrap(),
        };
        let serialized_api_message = api_message.serialize();

        let deserialized_api_message = APIMessage::deserialize(&serialized_api_message);
        assert_eq!(api_message, deserialized_api_message);
    }
    #[tokio::test]
    async fn test_handshake_new() {
        let mut settings = config::Config::default();
        settings.merge(config::File::with_name("config")).unwrap();

        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        {
            let mut wallet = wallet_lock.write().await;
            wallet.load_keys("test/testwallet", Some("asdf"));
        }
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));

        let (publickey, privatekey) = generate_keys();

        let socket_filter = ws_upgrade_route_filter(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
        );
        let mut ws_client = warp::test::ws()
            .path("/wsopen")
            .handshake(socket_filter)
            .await
            .expect("handshake");

        // let base58_pubkey = PublicKey::from_slice(&publickey).unwrap().serialize().to_base58();
        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );
        let api_message = APIMessage::new("SHAKINIT", 42, message_data);

        let serialized_api_message = api_message.serialize();

        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        let resp = ws_client.recv().await.unwrap();

        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());

        assert_eq!(command, "RESULT__");
        assert_eq!(index, 42);

        let deserialize_challenge =
            HandshakeChallenge::deserialize(&resp.as_bytes()[12..].to_vec());
        let raw_challenge: [u8; CHALLENGE_SIZE] =
            resp.as_bytes()[12..][..CHALLENGE_SIZE].try_into().unwrap();
        let sig: SaitoSignature = resp.as_bytes()[12..][CHALLENGE_SIZE..CHALLENGE_SIZE + 64]
            .try_into()
            .unwrap();

        assert_eq!(
            deserialize_challenge.challenger_ip_address(),
            [127, 0, 0, 1]
        );
        assert_eq!(deserialize_challenge.opponent_ip_address(), [127, 0, 0, 1]);
        assert_eq!(deserialize_challenge.opponent_pubkey(), publickey);
        assert!(verify(
            &hash(&raw_challenge.to_vec()),
            sig,
            deserialize_challenge.challenger_pubkey()
        ));

        let signed_challenge =
            sign_blob(&mut resp.as_bytes()[12..].to_vec(), privatekey).to_owned();

        let api_message = APIMessage::new("SHAKCOMP", 43, signed_challenge);
        let serialized_api_message = api_message.serialize();

        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        let resp = ws_client.recv().await.unwrap();

        let command = String::from_utf8_lossy(&resp.as_bytes()[0..8]);
        let index: u32 = u32::from_be_bytes(resp.as_bytes()[8..12].try_into().unwrap());
        let msg = String::from_utf8_lossy(&resp.as_bytes()[12..]);
        assert_eq!(command, "RESULT__");
        assert_eq!(index, 43);
        assert_eq!(msg, "OK");
    }

    #[tokio::test]
    async fn test_send_transaction() {
        let mut settings = config::Config::default();
        settings.merge(config::File::with_name("config")).unwrap();

        let wallet_lock = Arc::new(RwLock::new(Wallet::new()));
        {
            let mut wallet = wallet_lock.write().await;
            wallet.load_keys("test/testwallet", Some("asdf"));
        }
        let mempool_lock = Arc::new(RwLock::new(Mempool::new(wallet_lock.clone())));
        let blockchain_lock = Arc::new(RwLock::new(Blockchain::new(wallet_lock.clone())));

        let (publickey, _privatekey) = generate_keys();

        let socket_filter = ws_upgrade_route_filter(
            wallet_lock.clone(),
            mempool_lock.clone(),
            blockchain_lock.clone(),
        );
        let mut ws_client = warp::test::ws()
            .path("/wsopen")
            .handshake(socket_filter)
            .await
            .expect("transaction websocket");

        let mut message_data = vec![127, 0, 0, 1];
        message_data.extend(
            PublicKey::from_slice(&publickey)
                .unwrap()
                .serialize()
                .to_vec(),
        );
        let transaction = Transaction::new();
        let api_message = APIMessage::new("SNDTRANS", 0, transaction.serialize_for_net());
        let serialized_api_message = api_message.serialize();

        // TODO repair this test
        let _socket_resp = ws_client
            .send(Message::binary(serialized_api_message))
            .await;
        // let _resp = ws_client.recv().await.unwrap();

        // let mempool = mempool_lock.read().await;
        // assert_eq!(mempool.transactions.len(), 1);
    }
}
