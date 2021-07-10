use crate::{consensus::SaitoMessage, transaction::Transaction};
use tokio::sync::broadcast;

// use std::convert::Infallible;
use warp::{Buf, Filter};
use warp::{body};

fn post_transaction(mut body: impl Buf) -> Transaction {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    Transaction::deserialize_from_net(buffer)
}

pub async fn run(
    broadcast_channel_sender: broadcast::Sender<SaitoMessage>,
    mut _broadcast_channel_receiver: broadcast::Receiver<SaitoMessage>,
) -> crate::Result<()> {

    let transactions = warp::post()
        .and(warp::path("transactions"))
        .and(warp::path::end())
        .and(
            body::aggregate().map(move |body| {
                let transaction= post_transaction(body);
                broadcast_channel_sender.send(SaitoMessage::MempoolNewTransaction { transaction }).unwrap();
                Ok(warp::reply())
            })
        );

    let routes = transactions;

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}