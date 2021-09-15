use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{
    block::{Block, BlockType},
    blockchain::Blockchain,
    crypto::SaitoHash,
};

lazy_static::lazy_static! {
    pub static ref BLOCKS_DIR_PATH: String = configure_storage();
}

pub struct StorageConfigurer {}

pub fn configure_storage() -> String {
    if cfg!(test) {
        String::from("./data/test/blocks/")
    } else {
        String::from("./data/blocks/")
    }
}

pub struct Storage {}

impl Storage {
    /// read from a path to a Vec<u8>
    pub fn read(path: &str) -> io::Result<Vec<u8>> {
        let mut f = std::fs::File::open(path)?;
        let mut data = Vec::<u8>::new();
        f.read_to_end(&mut data)?;
        Ok(data)
    }

    pub fn write(data: Vec<u8>, filename: &str) {
        let mut buffer = File::create(filename).unwrap();
        buffer.write_all(&data[..]).unwrap();
    }

    pub fn generate_block_filename(block: &Block) -> String {
        let mut filename = BLOCKS_DIR_PATH.clone();

        filename.push_str(&hex::encode(block.get_timestamp().to_be_bytes()));
        filename.push_str(&String::from("-"));
        filename.push_str(&hex::encode(&block.get_hash()));
        filename.push_str(&".sai");
        filename
    }
    pub fn write_block_to_disk(block: &mut Block) {
        if block.get_block_type() == BlockType::Pruned {
            panic!("pruned blocks cannot be saved");
        }
        let filename = Storage::generate_block_filename(block);
        if !Path::new(&filename).exists() {
            let mut buffer = File::create(filename).unwrap();
            let byte_array: Vec<u8> = block.serialize_for_net();
            let test_block = Block::deserialize_for_net(&byte_array);
            buffer.write_all(&byte_array[..]).unwrap();
        }
    }

    pub async fn stream_block_from_disk(block_hash: SaitoHash) -> io::Result<Vec<u8>> {
        let mut filename = BLOCKS_DIR_PATH.clone();

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = tokio::fs::File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        tokio::io::AsyncReadExt::read_to_end(&mut f, &mut encoded).await?;

        Ok(encoded)
    }

    pub async fn stream_json_block_from_disk(block_hash: SaitoHash) -> io::Result<String> {
        let mut filename = BLOCKS_DIR_PATH.clone();
        filename.push_str(&"json/");
        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".json");

        fs::read_to_string(filename)
    }

    pub async fn load_blocks_from_disk(blockchain_lock: Arc<RwLock<Blockchain>>) {
        let mut paths: Vec<_> = fs::read_dir(BLOCKS_DIR_PATH.clone())
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
        for (_pos, path) in paths.iter().enumerate() {
            if !path.path().to_str().unwrap().ends_with(".gitignore") {
                let mut f = File::open(path.path()).unwrap();
                let mut encoded = Vec::<u8>::new();
                f.read_to_end(&mut encoded).unwrap();
                let block = Block::deserialize_for_net(&encoded);

                let mut blockchain = blockchain_lock.write().await;
                blockchain.add_block(block).await;
            }
        }
    }

    pub async fn load_block_from_disk(filename: String) -> Block {
        let file_to_load = &filename;
        let mut f = File::open(file_to_load).unwrap();
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).unwrap();
        Block::deserialize_for_net(&encoded)
    }

    pub async fn delete_block_from_disk(filename: String) -> bool {
        // TODO: get rid of this function or make it useful.
        // it should match the result and provide some error handling.
        let _res = std::fs::remove_file(filename);
        true
    }
}

pub trait Persistable {
    fn save(&self);
    fn load(filename: &str) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Drop for Blockchain {
        fn drop(&mut self) {
            let paths: Vec<_> = fs::read_dir(BLOCKS_DIR_PATH.clone())
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            for (_pos, path) in paths.iter().enumerate() {
                if !path.path().to_str().unwrap().ends_with(".gitignore") {
                    match std::fs::remove_file(path.path()) {
                        Err(err) => {
                            println!("Error cleaning up after tests {}", err);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
