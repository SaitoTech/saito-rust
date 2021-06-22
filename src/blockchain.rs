use crate::{
    consensus::{SaitoMessage},
};
use tokio::sync::{broadcast};



#[derive(Debug)]
pub struct Blockchain {

    broadcast_channel_sender:  broadcast::Sender<SaitoMessage>,

}

impl Blockchain {

    pub fn new(bsc : broadcast::Sender<SaitoMessage>) -> Self {
        Blockchain {
            broadcast_channel_sender: bsc,
        }
    }

}


