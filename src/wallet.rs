use crate::consensus::SaitoMessage;
use crate::crypto::{Keypair, SaitoPrivateKey, SaitoPublicKey};
use ::std::{sync::Arc, thread::sleep, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone, Debug)]
pub enum WalletMessage {
    TestMessage,
    TestMessage2,
}

/// The `Wallet` manages the public and private keypair of the node and holds the
/// slips that are used to form transactions on the network.
pub struct Wallet {
    broadcast_channel_sender: Option<broadcast::Sender<SaitoMessage>>,
    wallet_channel_sender: Option<mpsc::Sender<WalletMessage>>,

    keypair: Keypair,
}

impl Wallet {
    pub fn new() -> Self {
        Wallet {
            broadcast_channel_sender: None,
            wallet_channel_sender: None,
            keypair: Keypair::new(),
        }
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.keypair.get_publickey()
    }

    pub fn get_privatekey(&self) -> SaitoPrivateKey {
        self.keypair.get_privatekey()
    }

    pub fn set_broadcast_channel_sender(&mut self, bcs: broadcast::Sender<SaitoMessage>) {
        self.broadcast_channel_sender = Some(bcs);
    }
    pub fn set_wallet_channel_sender(&mut self, wcs: mpsc::Sender<WalletMessage>) {
        self.wallet_channel_sender = Some(wcs);
    }
}

//
// This function is called on initialization to setup the sending
// and receiving channels for asynchronous loops or message checks
//
pub async fn run(
    wallet_lock: Arc<RwLock<Wallet>>,
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let (wallet_channel_sender, mut wallet_channel_receiver) = mpsc::channel(4);

    //
    // pass clones of our broadcast sender channels into Wallet so it
    // can broadcast into the world as well...
    //
    {
        let mut wallet = wallet_lock.write().await;
        wallet.set_broadcast_channel_sender(broadcast_channel_sender.clone());
        wallet.set_wallet_channel_sender(wallet_channel_sender.clone());
    }

    //
    // loops to trigger messages
    //
    tokio::spawn(async move {
        loop {
            wallet_channel_sender
                .send(WalletMessage::TestMessage)
                .await
                .expect("error: Wallet TestMessage failed to send");
            sleep(Duration::from_millis(5000));
        }
    });

    loop {
        tokio::select! {

              //
        // local messages
        //
            Some(message) = wallet_channel_receiver.recv() => {
                match message {
            //
            // TestMessage
            //
            // test if our local broadcast loop is working by printing
            // a confirmation message when we receive a WalletMessage::
            // TestMessage.
            //
                    WalletMessage::TestMessage => {
            println!("Wallet has received local TestMessage broadcast");
                    },
            _ => {}
                }
            }


              //
        // system-wide messages
        //
            Ok(message) = broadcast_channel_receiver.recv() => {
                match message {
            //
            // BlockchainAddBlock
            //
            // triggered when the blockchain has validated and added
            // a new block. This examines the block for any transactions
            // relevant to our wallet.
            //
                    //SaitoMessage::MempoolNewBlock { hash } => {
                    //}
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn wallet_new_test() {
        assert_eq!(true, true);
    }
}
