use crate::blockchain::Blockchain;
use crate::consensus::SaitoMessage;
use crate::crypto::{hash, sign_blob, SaitoHash, SaitoPrivateKey, SaitoPublicKey};
use crate::mempool::Mempool;
use crate::peer::Peer;
use crate::transaction::Transaction;
use crate::wallet::Wallet;
use crate::networking::filters::{
    get_block_route_filter, post_transaction_route_filter, ws_upgrade_route_filter,
};
use log::{error, info, warn};
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::connect_async;

//use uuid::Uuid;
use warp::{Filter, Rejection};
//use super::message_types::send_block_head_message::SendBlockHeadMessage;
//use super::peer::{PeerSetting, OUTBOUND_PEER_CONNECTIONS_GLOBAL, PEERS_DB_GLOBAL};
use crate::networking::signals::signal_for_shutdown;

use config::Config;

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
}

impl Network {
    /// Create a Network
    pub fn new(
	saito_configuration_settings : Config,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        mempool_lock: Arc<RwLock<Mempool>>,
        wallet_lock: Arc<RwLock<Wallet>>,
    ) -> Network {

        let shost: [u8; 4] = saito_configuration_settings.get::<[u8; 4]>("network.host").unwrap();
        let sport: u16 = saito_configuration_settings.get::<u16>("network.port").unwrap();

        Network {
	    host: shost,
	    port: sport,
            blockchain_lock,
            mempool_lock,
            wallet_lock,
            broadcast_channel_sender: None,
        }
    }
}

pub async fn run(
    network_lock: Arc<RwLock<Network>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {

    //
    // spawn thread for server?
    //
    tokio::spawn(async move {

    });

println!("testing init done");

    //
    // connect to clients
    //

    Ok(())

}

#[cfg(test)]
mod tests {
}

