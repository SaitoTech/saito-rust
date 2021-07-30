use std::{
    fs::{self, File},
    io::{Read, Write},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{block::Block, blockchain::Blockchain};

pub const BLOCKS_DIR_PATH: &'static str = "./data/blocks/";

pub struct Storage {
    blocks_dir_path: String,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            blocks_dir_path: String::from("./data/blocks/"),
        }
    }

    pub fn write_block_to_disk(&self, block: &mut Block) {
        let mut filename = self.blocks_dir_path.clone();

        filename.push_str(&hex::encode(block.get_timestamp().to_be_bytes()));
        filename.push_str(&String::from("-"));
        filename.push_str(&hex::encode(&block.generate_hash()));
        filename.push_str(&".sai");

	block.set_filename(filename.clone());

//println!("trying to write to disk: {}", filename);

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
            if path.path().to_str().unwrap() != self.blocks_dir_path.clone() + "empty"
                && path.path().to_str().unwrap() != self.blocks_dir_path.clone() + ".gitignore"
            {
                let mut f = File::open(path.path()).unwrap();
                let mut encoded = Vec::<u8>::new();
                f.read_to_end(&mut encoded).unwrap();
                let mut block = Block::deserialize_for_net(encoded);

                //
                // the hash needs calculation separately after loading
                //
                if block.get_hash() == [0; 32] {
                    let block_hash = block.generate_hash();
                    block.set_hash(block_hash);
                }
                println!("loading block with hash: {:?}", block.get_hash());

                let mut blockchain = blockchain_lock.write().await;
                blockchain.add_block(block).await;
                println!("Loaded block {} of {}", pos, paths.len() - 1);
            }
        }
    }
    pub async fn load_block_from_disk(filename : String) -> Block {
println!("Filename is: {}", filename);
        //let file_to_load = BLOCKS_DIR_PATH.to_string() + &filename;
        let file_to_load = &filename;
        let mut f = File::open(file_to_load).unwrap();
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).unwrap();
        let mut block = Block::deserialize_for_net(encoded);

        //
        // the hash needs calculation separately after loading
        //
        if block.get_hash() == [0; 32] {
            let block_hash = block.generate_hash();
            block.set_hash(block_hash);
        }
        println!("loaded block: {}", filename);

	return block;
    }

}
