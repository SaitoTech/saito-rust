use crate::{
    consensus::{SaitoMessage},
    blockchain::Blockchain,
};
use::std::{
    sync::{Arc},
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use tokio::time;

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
	    broadcast_channel_sender : bcs,
        }
    }


    pub async fn start_bundling(&self, mempool_lock: Arc<RwLock<Mempool>>, blockchain_lock: Arc<RwLock<Blockchain>>) {

        let mut already_bundling;
        let mut count = 5;


        //
        // if we are not already bundling, start...
        //
        { 
          let mut w = BUNDLER_ACTIVE.write().await;
          already_bundling = *w;
          *w = 1;
        }

	//
	// if we were though, exit...
	//
        if already_bundling > 0 {
            return;
        }


	//
	// sanity check to update already_bundling
	//
        {
            let r = BUNDLER_ACTIVE.read().await;
            already_bundling = *r;
        }


        //
        // spawn async thread to manage timer
        //
        tokio::spawn(async move {

            let mut interval = time::interval(Duration::from_millis(500));
            while already_bundling > 0 {

                interval.tick().await;

                if already_bundling > 0 {

                    count -= 1;
                    println!("Tick... {:?}", count);
                    interval.tick().await;

                    if count == 0 {

			//
			// if we stop bundling that requires write-access
			// to the mempool and at least read access to the 
			// blockchain. We are in an async thread that has
			// not taken ownership of either, which is why we
			// have received the clones of the locks as args
			//
                        let mut mempool = mempool_lock.write().await;
                        mempool.stop_bundling().await;
                        mempool.bundle_block(blockchain_lock.clone()).await;
                    }
                    {
			//
			// update already_bundling so that we can stop our
			// loop if anyone else (such as our stop_bundling
			// call above) has changed the value of our timer
			// tracker.
			//
                        let r = BUNDLER_ACTIVE.read().await;
                        already_bundling = *r;
                    }
                }
            }
        });
    }


    pub async fn stop_bundling(&self) {
        let mut w = BUNDLER_ACTIVE.write().await;
        *w = 0;
    }

    pub async fn bundle_block(&mut self, blockchain_lock: Arc<RwLock<Blockchain>>) {

	println!("Bundling a Block!");

        self.broadcast_channel_sender
                        .send(SaitoMessage::StartBundling { })
			.expect("error: Mempool error broadcasting StartBundling message");

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
    fn mempool_start_bundling_test() {
        assert_eq!(true, true);
    }
    fn mempool_stop_bundling_test() {
        assert_eq!(true, true);
    }

}
