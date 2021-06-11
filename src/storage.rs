use crate::block::Block;
use crate::crypto::Sha256Hash;

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use std::io::prelude::*;

pub const BLOCKS_DIR: &str = "./src/data/blocks/";

pub struct Storage {
    blocks_dir_path: Option<String>,
}

impl Storage {
    pub fn new(dir_path: Option<String>) -> Self {
        Storage {
            blocks_dir_path: dir_path,
        }
    }

    pub async fn write_block_to_disk_async(&self, block: Block) -> io::Result<()> {
        let mut filename = match self.blocks_dir_path.clone() {
            Some(dir) => dir,
            None => String::from(BLOCKS_DIR),
        };

        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");

        let mut buffer = File::create(filename).await?;

        let byte_array: Vec<u8> = block.into();
        buffer.write_all(&byte_array[..]).await?;

        Ok(())
    }

    pub async fn read_block_from_disk_async(&self, block_hash: Sha256Hash) -> io::Result<Block> {
        let mut filename = match self.blocks_dir_path.clone() {
            Some(dir) => dir,
            None => String::from(BLOCKS_DIR),
        };

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).await?;

        Ok(Block::from(encoded))
    }

    pub async fn stream_block_from_disk(&self, block_hash: Sha256Hash) -> io::Result<Vec<u8>> {
        let mut filename = match self.blocks_dir_path.clone() {
            Some(dir) => dir,
            None => String::from(BLOCKS_DIR),
        };

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).await?;

        Ok(encoded)
    }

    pub fn write_block_to_disk(&self, block: Block) -> io::Result<()> {
        let mut filename = match self.blocks_dir_path.clone() {
            Some(dir) => dir,
            None => String::from(BLOCKS_DIR),
        };

        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");

        let mut buffer = std::fs::File::create(filename)?;

        let byte_array: Vec<u8> = block.into();
        buffer.write_all(&byte_array[..])?;

        Ok(())
    }

    pub fn read_block_from_disk(&self, block_hash: Sha256Hash) -> io::Result<Block> {
        let mut filename = match self.blocks_dir_path.clone() {
            Some(dir) => dir,
            None => String::from(BLOCKS_DIR),
        };

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = std::fs::File::open(filename)?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded)?;

        Ok(Block::from(encoded))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::Block;

    fn teardown() -> io::Result<()> {
        let dir_path = String::from("./src/data/blocks/");
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }
        std::fs::File::create("./src/data/blocks/empty")?;

        Ok(())
    }

    #[tokio::test]
    async fn storage_write_block_to_disk_async() {
        let dir_path = String::from("./src/data/blocks/");
        let block = Block::default();
        let storage = Storage::new(Some(dir_path));
        let result = storage.write_block_to_disk_async(block).await;
        assert_eq!(result.unwrap(), ());

        // TODO -- add unwind_panic to teardown when assert failes
        teardown().expect("Teardown failed");
    }

    #[tokio::test]
    async fn storage_read_block_to_disk_async() {
        let dir_path = String::from("./src/data/blocks/");
        let storage = Storage::new(Some(dir_path));
        let block = Block::default();
        let block_hash = block.hash();

        storage.write_block_to_disk_async(block).await.unwrap();
        match storage.read_block_from_disk_async(block_hash).await {
            Ok(_block) => {
                assert!(true);
                teardown().expect("Teardown failed");
            }
            Err(_err) => {
                teardown().expect("Teardown failed");
            }
        }

        // TODO -- add unwind_panic to teardown when assert failes
    }
}
