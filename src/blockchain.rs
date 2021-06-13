use crate::{
  block::Block,
  types::SaitoMessage,
};

use tokio::sync::{broadcast};


#[derive(Debug)]
pub struct Blockchain {

    /// Vector of blocks
    blocks: Vec<Block>,
    broadcast_channel_sender:   Option<broadcast::Sender<SaitoMessage>>,

}


impl Blockchain {
    pub fn new() -> Self {
        Blockchain {

            blocks: vec![],
            broadcast_channel_sender: None,

        }
    }


    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<(SaitoMessage)>) {
        self.broadcast_channel_sender = Some(bcs);
    }

    /// Append `Block` to the index of `Blockchain`
    pub fn add_block(&mut self, mut block: Block) {

	//
	// Add to Blockchain
	//
        println!("Adding Block");
        self.blocks.push(block);
	println!("hash: {:?}", self.blocks[self.blocks.len()-1].get_hash());

	//
	// Resume Bundling
	//
        if !self.broadcast_channel_sender.is_none() {
           self.broadcast_channel_sender.as_ref().unwrap()
                        .send(SaitoMessage::StartBundling)
                        .expect("error: Mempool TryBundle Block message failed to send");
        }
    }

}


