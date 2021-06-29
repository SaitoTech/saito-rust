use std::{
    fs::{self, File},
    io::{Read, Write},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{block::Block, blockchain::Blockchain};

pub struct Storage {
    blocks_dir_path: String,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            blocks_dir_path: String::from("./data/blocks/"),
        }
    }

    pub fn write_block_to_disk(&self, block: &Block) {
        let mut filename = self.blocks_dir_path.clone();
        filename.push_str(&hex::encode(block.get_timestamp().to_be_bytes()));
        filename.push_str(&String::from("-"));
        filename.push_str(&hex::encode(&block.generate_hash()));
        filename.push_str(&".sai");

        let mut buffer = File::create(filename).unwrap();

        let byte_array: Vec<u8> = block.serialize_for_net();
        buffer.write_all(&byte_array[..]).unwrap();
    }
    pub async fn load_blocks_from_disk(&self, blockchain_lock: Arc<RwLock<Blockchain>>) {
        let mut paths: Vec<_> = fs::read_dir(self.blocks_dir_path.clone())
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        paths.sort_by(|a, b| {
            let a_metadata = fs::metadata(a.path()).unwrap();
            let b_metadata = fs::metadata(b.path()).unwrap();
            a_metadata
                .modified()
                .unwrap()
                .partial_cmp(&b_metadata.modified().unwrap())
                .unwrap()
        });
        for (pos, path) in paths.iter().enumerate() {
            if path.path().to_str().unwrap() != self.blocks_dir_path.clone() + "empty" {
                let mut f = File::open(path.path()).unwrap();
                let mut encoded = Vec::<u8>::new();
                f.read_to_end(&mut encoded).unwrap();
                let block = Block::deserialize_for_net(encoded);
                //let mut blockchain = Handle::current().block_on();
                println!("get lock...");
                let mut blockchain = blockchain_lock.write().await;
                println!("adding...");
                blockchain.add_block(block).await;
                println!("Loaded block {} of {}", pos, paths.len() - 1);
            }
        }
    }
}
