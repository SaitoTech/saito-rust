use std::{
    fs::{self, File},
    io::{self, Read, Write},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{block::Block, blockchain::Blockchain, crypto::SaitoHash};

pub struct Storage {
    blocks_dir_path: String,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            blocks_dir_path: String::from("./data/blocks/"),
        }
    }

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

    pub fn write_block_to_disk(&self, block: &Block) {
        // our binary filename
        let mut bin_filename = self.blocks_dir_path.clone();
        bin_filename.push_str(&"bin/");
        bin_filename.push_str(&hex::encode(&block.generate_hash()));
        bin_filename.push_str(&".sai");

        // our json filename
        let mut json_filename = self.blocks_dir_path.clone();
        json_filename.push_str(&"json/");
        json_filename.push_str(&hex::encode(&block.generate_hash()));
        json_filename.push_str(&".json");

        // write our block to binary file
        let mut buffer = File::create(bin_filename).unwrap();
        let byte_array: Vec<u8> = block.serialize_for_net();
        buffer.write_all(&byte_array[..]).unwrap();

        // write our block to json file
        if let Ok(block_json) = serde_json::to_string(block) {
            fs::write(json_filename, block_json).unwrap();
        }
    }

    pub async fn stream_block_from_disk(&self, block_hash: SaitoHash) -> io::Result<Vec<u8>> {
        let mut filename = self.blocks_dir_path.clone();

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = tokio::fs::File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        tokio::io::AsyncReadExt::read_to_end(&mut f, &mut encoded).await?;

        Ok(encoded)
    }

    pub async fn stream_json_block_from_disk(&self, block_hash: SaitoHash) -> io::Result<String> {
        let mut filename = self.blocks_dir_path.clone();
        filename.push_str(&"json/");
        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".json");

        fs::read_to_string(filename)
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
        for (_pos, path) in paths.iter().enumerate() {
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
                // println!("loading block with hash: {:?}", block.get_hash());

                let mut blockchain = blockchain_lock.write().await;
                blockchain.add_block(block).await;
                // println!("Loaded block {} of {}", pos, paths.len() - 1);
            }
        }
    }
}
