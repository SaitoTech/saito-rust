use std::convert::Infallible;
use warp::Filter;
use warp::{body};
use futures::stream::{Stream, StreamExt};

use crate::{constants, storage::Storage};

pub struct Network {}

impl Network {
    pub async fn start(&self) -> crate::Result<()> {
        let blocks= warp::path("blocks")
          .and(warp::path::param().and_then(get_block));

        let transactions = warp::post()
            .and(warp::path("transactions"))
            .and(warp::path::end())
            .and(body::stream())
            .and_then(post_transaction);

        // warp::path("transactions")
        //     .and(warp::path::param().and_then(post_transaction));

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

async fn post_transaction<S, B>(stream: S) -> Result<impl warp::Reply, warp::Rejection>
where
    S: Stream<Item = Result<B, warp::Error>>,
    S: StreamExt,
    B: warp::Buf
{
    // let mut file = File::create("some_binary_file").unwrap();

    let pinnedStream = Box::pin(stream);
    let mut buffer = vec![];
    while let Some(item) = pinnedStream.next().await {
        let mut data = item.unwrap();
        // file.write_all(data.to_bytes().as_ref());
        buffer.put(data.into_buf());
        // file.write_all(data);
    }
    Ok(warp::reply())
}