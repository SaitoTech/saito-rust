
use crate::{
    block::Block, blockchain::Blockchain, consensus::SaitoMessage, crypto::{SaitoPublicKey,SaitoPrivateKey,SaitoHash}, slip::Slip,
    transaction::Transaction,
    wallet::Wallet,
};
use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum MempoolMessage {
    GenerateBlock,
    AddBlockToBlockchain,
}

/// The `Mempool` holds unprocessed blocks and transactions and is in control of
/// discerning when thenodeis allowed to create a block. It bundles the block and
/// sends it to the `Blockchain` to be added to the longest-chain. New `Block`s
/// received over the network are queued in the `Mempool` before being added to
/// the `Blockchain`
pub struct Mempool {
    blocks: Vec<Block>,
}

impl Mempool {

    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Mempool {
    	    blocks: vec![],
        }
    }


    pub fn add_block(&mut self, block: Block) -> bool {
        let hash_to_insert = block.get_hash();
        for blk in &self.blocks {
            if blk.get_hash() == hash_to_insert {
                return false;
            }
        }

println!("adding block to mempool queue");

        self.blocks.push(block);
        return true;
    }

    pub fn take_block(&mut self, hash: SaitoHash) -> Option<Block> {
        let mut block_found = false;
        let mut block_idx = 0;
        for i in 0..self.blocks.len() {
            if self.blocks[0].get_hash() == hash {
                block_idx = i;
                block_found = true;
                break;
            }
        }

        if block_found {
            let block = self.blocks.remove(block_idx);
            return Some(block);
        }
        return None;
    }


    pub async fn generate_block(
        &mut self,
        blockchain_lock: Arc<RwLock<Blockchain>>,
        _creator_publickey: SaitoPublicKey,
        _creator_privatekey: SaitoPrivateKey,
    ) -> Block {

	let blockchain = blockchain_lock.read().await;
        let previous_block_id = blockchain.get_latest_block_id();
        let previous_block_hash = blockchain.get_latest_block_hash();

	let mut block = Block::default();
        block.set_id(previous_block_id);
        block.set_previous_block_hash(previous_block_hash);
        block.set_hash();

        for _i in 0..1000 {

            let mut transaction = Transaction::default();

            transaction.set_message((0..1024).map(|_| rand::random::<u8>()).collect());

            let mut input1 = Slip::default();
            input1.set_publickey([1; 33]);
            input1.set_amount(1000000);
            input1.set_uuid([1; 64]);

            let mut output1 = Slip::default();
            output1.set_publickey([1; 33]);
            output1.set_amount(1000000);
            output1.set_uuid([1; 64]);

            transaction.add_input(input1);
            transaction.add_output(output1);
        }

        block
    }
}

// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
pub async fn run(
    mempool_lock: Arc<RwLock<Mempool>>,
    blockchain_lock: Arc<RwLock<Blockchain>>,
    wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {

    let (mempool_channel_sender, mut mempool_channel_receiver) = mpsc::channel(4);


    let generate_block_sender = mempool_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            generate_block_sender
                .send(MempoolMessage::GenerateBlock)
                .await
                .expect("error: GenerateBlock message failed to send");
            sleep(Duration::from_millis(5000));
        }
    });


    let add_block_to_blockchain_sender = mempool_channel_sender.clone();
    tokio::spawn(async move {
        loop {
            add_block_to_blockchain_sender
                .send(MempoolMessage::AddBlockToBlockchain)
                .await
                .expect("error: AddBlockToBlockchain message failed to send");
            sleep(Duration::from_millis(1000));
        }
    });


    loop {
        tokio::select! {
            Some(message) = mempool_channel_receiver.recv() => {
                match message {
                    // GenerateBlock makes periodic attempts to analyse the state of 
		    // the mempool and produce blocks if possible.
                    MempoolMessage::GenerateBlock => {

                        let mut mempool = mempool_lock.write().await;
                        let wallet = wallet_lock.read().await;

			let creator_publickey = wallet.get_publickey();
			let creator_privatekey = wallet.get_privatekey();

                        let block = mempool.generate_block(blockchain_lock.clone(), creator_publickey, creator_privatekey).await;
			mempool.add_block(block);
                    },

                    // AddBlockToBlockchain periodically checks the block queue to see
		    // if we should announce the existence of new blocks to the blockchain
		    MempoolMessage::AddBlockToBlockchain => {
                        let mempool = mempool_lock.read().await;
			if mempool.blocks.len() > 0 {
			    broadcast_channel_sender
                        	.send(SaitoMessage::MempoolNewBlock { hash: mempool.blocks[0].get_hash() })
                                .expect("error: MempoolNewBlock message failed to send");
			}
                    },
                }
            }


            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
                    // triggered when a block is received over the network and
                    // will be added to the `Blockchain`
                    SaitoMessage::MempoolNewBlock { hash } => {
                        let mut mempool = mempool_lock.write().await;
                        if let Some(block) = mempool.take_block(hash) {
                            let mut blockchain = blockchain_lock.write().await;
                            blockchain.add_block(block);
                        }
                    }
                    SaitoMessage::MempoolNewTransaction { transaction: _transaction } => {
                        let mut _mempool = mempool_lock.write().await;
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn mempool_new_test() {
        assert_eq!(true, true);
    }

    #[test]
    fn mempool_generate_block_test() {
        assert_eq!(true, true);
    }

}
