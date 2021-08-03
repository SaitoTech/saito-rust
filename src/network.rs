use crate::{consensus::SaitoMessage, transaction::Transaction};
use tokio::sync::broadcast;

// use std::convert::Infallible;
use warp::body;
use warp::{Buf, Filter};

fn post_transaction(mut body: impl Buf) -> Transaction {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    // TODO: decide where we get `hash_for_signature`
    // Options are
    // 1. It's sent along with other tx info
    // 2. regenerated in deserialization
    // 3. regenereted when fetched -> hash_for_signature would become an Option<SaitoHash> in that case
    //
    // let mut tx = Transaction::deserialize_from_net(buffer);
    // let hash_for_signature = hash(&tx.serialize_for_signature());
    // tx.set_hash_for_signature(hash_for_signature);
    // tx

    Transaction::deserialize_from_net(buffer)
}

pub async fn run(
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut _broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {
    let transactions = warp::post()
        .and(warp::path("transactions"))
        .and(warp::path::end())
        .and(body::aggregate().map(move |body| {
            let transaction = post_transaction(body);
            broadcast_channel_sender
                .send(SaitoMessage::MempoolNewTransaction { transaction })
                .unwrap();
            Ok(warp::reply())
        }));

    let routes = transactions;

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
