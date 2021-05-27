use crate::block::Block;

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

pub const BLOCKS_DIR: &str = "./data/blocks/";
// pub const BLOCKS_DIR: &str = "./";

pub struct Storage {
    blocks_dir_path: Option<String>,
}

impl Storage {
    pub fn new(dir_path: Option<String>) -> Self {
        Storage {
            blocks_dir_path: dir_path,
        }
    }

    pub async fn write_block_to_disk(&self, block: Block) -> io::Result<()> {
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

    pub async fn read_block_from_disk(&self, block_hash: [u8; 32]) -> io::Result<Block> {
        let mut filename = String::from(BLOCKS_DIR);

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        f.read_to_end(&mut encoded).await?;

        Ok(Block::from(encoded))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{block::Block, keypair::Keypair};

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
    async fn storage_write_block_to_disk() {
        let dir_path = String::from("./src/data/blocks/");
        let keypair = Keypair::new();
        let block = Block::new(*keypair.public_key(), [0; 32]);
        let storage = Storage::new(Some(dir_path));
        let result = storage.write_block_to_disk(block).await;
        assert_eq!(result.unwrap(), ());

        // TODO -- add unwind_panic to teardown when assert failes
        teardown().expect("Teardown failed");
    }

    #[tokio::test]
    async fn storage_read_block_to_disk() {
        let dir_path = String::from("./src/data/blocks/");
        let storage = Storage::new(Some(dir_path));
        let keypair = Keypair::new();
        let block = Block::new(*keypair.public_key(), [0; 32]);
        let block_hash = block.hash();
        storage.write_block_to_disk(block).await.unwrap();
        match storage.read_block_from_disk(block_hash).await {
            Ok(_block) => {
                assert!(true);
            }
            Err(_err) => {}
        }

        // TODO -- add unwind_panic to teardown when assert failes
        teardown().expect("Teardown failed");
    }
}
