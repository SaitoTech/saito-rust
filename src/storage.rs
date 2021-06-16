use crate::block::Block;
use crate::crypto::Sha256Hash;

use std::fs::rename;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use std::fs;
use std::fs::ReadDir;
use std::io::prelude::*;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Storage {
    blocks_dir_path: String,
}

impl Storage {
    pub fn new(dir_path: String) -> Self {
        fs::create_dir_all(dir_path.clone()).unwrap();
        Storage {
            blocks_dir_path: dir_path,
        }
    }
    /// Write an array of u8 to a filename in the blocks directory
    pub async fn write_to_blocks_dir_async(
        &self,
        filename: &str,
        byte_array: &[u8],
    ) -> io::Result<()> {
        let mut full_filename = self.blocks_dir_path.clone();
        full_filename.push_str(&filename);
        let mut buffer = File::create(full_filename).await?;
        buffer.write_all(&byte_array[..]).await?;
        Ok(())
    }

    /// read from a filename in the blocks directory
    pub fn read_from_blocks_dir(&self, filename: &str) -> io::Result<Vec<u8>> {
        let mut full_filename = self.blocks_dir_path.clone();
        full_filename.push_str(&filename);
        let mut f = std::fs::File::open(full_filename)?;
        let mut data = Vec::<u8>::new();
        f.read_to_end(&mut data)?;

        Ok(data)
    }
    /// read from a path to a Vec<u8>
    pub fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut f = std::fs::File::open(path)?;
        let mut data = Vec::<u8>::new();
        f.read_to_end(&mut data)?;
        Ok(data)
    }

    /// list all the files in the blocks directory
    pub fn list_files_in_blocks_dir(&self) -> ReadDir {
        fs::read_dir(self.blocks_dir_path.clone()).unwrap()
    }

    pub fn roll_forward(&self, block: &Block) {
        let mut filename = self.blocks_dir_path.clone();
        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");
        println!("rollforward! {}", filename);
        match Path::new(&filename[..]).exists() {
            true => {
                println!("FOUND");
                let mut new_filename = self.blocks_dir_path.clone();
                new_filename.push_str(&hex::encode(block.id().to_be_bytes()));
                new_filename.push_str(&String::from("-"));
                new_filename.push_str(&block.hash_as_hex().clone());
                new_filename.push_str(&".sai");
                println!("new filename {}", new_filename);
                rename(filename, new_filename).unwrap();
            }
            false => {
                println!("NOT FOUND");
                self.write_block_to_disk(block, true).unwrap();
            } // move file
        }
    }

    pub fn roll_back(&self, block: &Block) {
        let mut filename = self.blocks_dir_path.clone();
        filename.push_str(&hex::encode(block.id().to_be_bytes()));
        filename.push_str(&String::from("-"));
        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");

        let mut new_filename = self.blocks_dir_path.clone();
        new_filename.push_str(&block.hash_as_hex().clone());
        new_filename.push_str(&".sai");
        println!("rollback {} {}", filename, new_filename);
        rename(filename, new_filename).unwrap();
    }

    pub async fn roll_forward_async(&self, block: &Block) {
        let mut filename = self.blocks_dir_path.clone();
        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");
        println!("rollforward! {}", filename);
        match Path::new(&filename[..]).exists() {
            true => {
                println!("FOUND");
                let mut new_filename = self.blocks_dir_path.clone();
                new_filename.push_str(&hex::encode(block.id().to_be_bytes()));
                new_filename.push_str(&String::from("-"));
                new_filename.push_str(&block.hash_as_hex().clone());
                new_filename.push_str(&".sai");
                println!("new filename {}", new_filename);
                rename(filename, new_filename).unwrap();
            }
            false => {
                println!("NOT FOUND");
                self.write_block_to_disk_async(block, true).await.unwrap();
            } // move file
        }
    }

    pub async fn write_block_to_disk_async(
        &self,
        block: &Block,
        is_longest_chain: bool,
    ) -> io::Result<()> {
        let mut filename = String::from("");
        if is_longest_chain {
            filename.push_str(&hex::encode(block.id().to_be_bytes()));
            filename.push_str(&String::from("-"));
        }
        filename.push_str(&block.hash_as_hex().clone());
        filename.push_str(&".sai");
        let block_bytes: Vec<u8> = block.into();
        self.write_to_blocks_dir_async(&filename, &block_bytes[..])
            .await
    }

    pub async fn read_block_from_disk_async(&self, block_hash: Sha256Hash) -> io::Result<Block> {
        let mut filename = self.blocks_dir_path.clone();

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).await?;

        Ok(Block::from(encoded))
    }

    pub async fn stream_block_from_disk(&self, block_hash: Sha256Hash) -> io::Result<Vec<u8>> {
        let mut filename = self.blocks_dir_path.clone();

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).await?;

        Ok(encoded)
    }

    pub fn write_block_to_disk(&self, block: &Block, is_longest_chain: bool) -> io::Result<()> {
        let mut filename = self.blocks_dir_path.clone();
        if is_longest_chain {
            filename.push_str(&hex::encode(block.id().to_be_bytes()));
            filename.push_str(&String::from("-"));
        }
        filename.push_str(&block.hash_as_hex());
        filename.push_str(&".sai");

        let mut buffer = std::fs::File::create(filename)?;

        let byte_array: Vec<u8> = block.into();
        buffer.write_all(&byte_array[..])?;

        Ok(())
    }

    pub fn read_block_from_disk(&self, block_hash: Sha256Hash) -> io::Result<Block> {
        let mut filename = self.blocks_dir_path.clone();
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

    fn teardown(dir_path: String) -> io::Result<()> {
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
        let dir_path = String::from("./data/test/blocks/");
        let block = &Block::default();
        let storage = Storage::new(dir_path.clone());
        let result = storage.write_block_to_disk_async(block, true).await;
        assert_eq!(result.unwrap(), ());

        // TODO -- add unwind_panic to teardown when assert failes
        teardown(dir_path).expect("Teardown failed");
    }

    #[tokio::test]
    async fn storage_read_block_to_disk_async() {
        let dir_path = String::from("./data/test/blocks/");
        let storage = Storage::new(dir_path.clone());
        let block = &Block::default();
        let block_hash = block.hash();

        storage
            .write_block_to_disk_async(block, true)
            .await
            .unwrap();
        match storage.read_block_from_disk_async(block_hash).await {
            Ok(_block) => {
                assert!(true);
                teardown(dir_path).expect("Teardown failed");
            }
            Err(_err) => {
                teardown(dir_path).expect("Teardown failed");
            }
        }

        // TODO -- add unwind_panic to teardown when assert failes
    }
}
