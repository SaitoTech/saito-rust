use crate::crypto::{SaitoHash, SaitoPublicKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use serde::{Serialize, Deserialize};

pub type Peers = Arc<RwLock<HashMap<SaitoHash, Peer>>>;

#[derive(Debug, Clone)]
pub struct Peer {
    // pub host: [u8; 4],
    // pub port: u16,
    pub has_handshake: bool,
    pub pubkey: Option<SaitoPublicKey>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16
}
