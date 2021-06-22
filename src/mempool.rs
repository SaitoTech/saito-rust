use crate::{
    saito_message::SaitoMessage,
};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};


lazy_static! {
    pub static ref BUNDLER_ACTIVE: RwLock<u64> = RwLock::new(0);
}




/// The `Mempool` holds unprocessed blocks and transactions and is in control of 
/// discerning when thenodeis allowed to create a block. It bundles the block and 
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s 
/// received over the network are queued in the `Mempool` before being added to 
/// the `Blockchain`
pub struct Mempool {

    broadcast_channel_sender:  broadcast::Sender<SaitoMessage>,

}

impl Mempool {

    pub fn new(bcs : broadcast::Sender<SaitoMessage>) -> Self {
        Mempool {
	    broadcast_channel_sender = bcs,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::keypair::Keypair;
    use std::sync::Arc;

    #[test]
    fn mempool_test() {
        assert_eq!(true, true);
    }

}
