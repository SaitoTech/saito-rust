use crate::{constants, storage::Storage, transaction::Transaction};

use std::convert::Infallible;
use warp::{Buf, Filter};
use warp::{body};

pub struct Network {}

impl Network {
    pub async fn start(&self) -> crate::Result<()> {
        let blocks= warp::path("blocks")
            .and(warp::path::param().and_then(get_block));

        let transactions = warp::post()
            .and(warp::path("transactions"))
            .and(warp::path::end())
            .and(body::aggregate())
            .and_then(post_transaction);

        let routes = blocks.or(transactions);

        warp::serve(routes)
            .run(([127, 0, 0, 1], 3030)).await;

        Ok(())
    }
}

async fn get_block(str_block_hash: String) -> Result<impl warp::Reply, Infallible> {
    let storage = Storage::new(String::from(constants::BLOCKS_DIR));

    let mut block_hash = [0u8; 32];
    hex::decode_to_slice(str_block_hash, &mut block_hash).expect("Failed to parse hash");

    match storage.stream_block_from_disk(block_hash).await {
        Ok(block_bytes) => Ok(block_bytes),
        Err(_err) => {
            eprintln!("{:?}", _err);
            Ok(vec![])
        }
    }
}

async fn post_transaction(mut body: impl Buf) -> Result<impl warp::Reply, warp::Rejection> {
    let mut buffer = vec![];
    while body.has_remaining() {
        buffer.append(&mut body.chunk().to_vec());
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    let tx = Transaction::from(buffer);
    println!("{:?}", tx.signature());

    Ok(warp::reply())
}
