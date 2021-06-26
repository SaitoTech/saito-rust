use crate::consensus::SaitoMessage;
use tokio::sync::broadcast;

pub async fn run(
    _broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    while let Ok(_message) = broadcast_channel_receiver.recv().await {
        //println!("NEW BLOCK!");
    }

    Ok(())
}
