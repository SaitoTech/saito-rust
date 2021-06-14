use std::convert::Infallible;
use warp::Filter;

use crate::storage::Storage;

pub struct Network {}

impl Network {
    pub async fn start(&self) -> crate::Result<()> {
        let blocks = warp::path("blocks");

        let get_blocks = warp::path::param().and_then(get_block);

        let _get_blocks = blocks.and(get_blocks);

        warp::serve(_get_blocks).run(([127, 0, 0, 1], 3030)).await;

        Ok(())
    }
}

async fn get_block(str_block_hash: String) -> Result<impl warp::Reply, Infallible> {
    let storage = Storage::new(None);

    println!("{:?}", str_block_hash.clone());

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
