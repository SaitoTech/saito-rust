use super::api_message::APIMessage;
use crate::crypto::{SaitoHash, SaitoPublicKey};
use crate::storage::{Persistable, Storage};
use macros::Persistable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

pub type Peers = Arc<RwLock<HashMap<SaitoHash, Peer>>>;
pub type TokioSocketError =
    tokio::sync::mpsc::error::SendError<std::result::Result<warp::ws::Message, warp::Error>>;

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Persistable)]
pub struct Peer {
    pub has_handshake: bool,
    #[serde_as(as = "Option<[_; 33]>")]
    pub pubkey: Option<SaitoPublicKey>,
    #[serde(skip)]
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

impl Peer {
    pub fn send_message(&self, message: &APIMessage) -> std::result::Result<(), TokioSocketError> {
        self.sender
            .as_ref()
            .unwrap()
            .send(Ok(Message::binary(message.serialize())))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerSetting {
    pub host: [u8; 4],
    pub port: u16,
}
