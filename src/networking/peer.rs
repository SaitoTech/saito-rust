use crate::crypto::{SaitoHash, SaitoPublicKey};
use crate::storage::{Persistable, Storage};
use macros::Persistable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

pub type Peers = Arc<RwLock<HashMap<SaitoHash, Peer>>>;

#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Persistable)]
// #[persist_with_name(get_name)]
pub struct Peer {
    pub has_handshake: bool,
    #[serde_as(as = "Option<[_; 33]>")]
    pub pubkey: Option<SaitoPublicKey>,
    #[serde(skip)]
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}
// Demo of Persistable usage
// impl Peer {
//     pub fn get_name(&self) -> String {
//         hex::encode(&self.pubkey.unwrap())
//     }
// }
