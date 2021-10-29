use crate::blockchain::MAX_TOKEN_SUPPLY;
use crate::crypto::SaitoPublicKey;
use crate::slip::{Slip, SlipType};
use std::{
    fs::{self, File},
    io::{self, BufRead, Read, Write},
    path::Path,
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{
    block::{Block, BlockType},
    blockchain::Blockchain,
};

lazy_static::lazy_static! {
    pub static ref BLOCKS_DIR_PATH: String = configure_storage();
}

pub const ISSUANCE_FILE_PATH: &'static str = "./data/issuance/issuance";
pub const EARLYBIRDS_FILE_PATH: &'static str = "./data/issuance/earlybirds";
pub const DEFAULT_FILE_PATH: &'static str = "./data/issuance/default";

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

    pub fn file_exists(filename: &str) -> bool {
        let path = Path::new(&filename);
        path.exists()
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
            let byte_array: Vec<u8> = block.serialize_for_net(BlockType::Full);
            buffer.write_all(&byte_array[..]).unwrap();
        }
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
                let mut block = Block::deserialize_for_net(&encoded);
                let mut blockchain = blockchain_lock.write().await;
                block.generate_metadata();
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

    //
    // token issuance functions below
    //
    pub fn return_token_supply_slips_from_disk() -> Vec<Slip> {
        let mut v: Vec<Slip> = vec![];
        let mut tokens_issued = 0;

        if let Ok(lines) = Storage::read_lines_from_file(ISSUANCE_FILE_PATH) {
            for line in lines {
                if let Ok(ip) = line {
                    let s = Storage::convert_issuance_into_slip(ip);
                    v.push(s);
                }
            }
        }
        if let Ok(lines) = Storage::read_lines_from_file(EARLYBIRDS_FILE_PATH) {
            for line in lines {
                if let Ok(ip) = line {
                    let s = Storage::convert_issuance_into_slip(ip);
                    v.push(s);
                }
            }
        }

        for i in 0..v.len() {
            tokens_issued += v[i].get_amount();
        }

        if let Ok(lines) = Storage::read_lines_from_file(DEFAULT_FILE_PATH) {
            for line in lines {
                if let Ok(ip) = line {
                    let mut s = Storage::convert_issuance_into_slip(ip);
                    s.set_amount(MAX_TOKEN_SUPPLY - tokens_issued);
                    v.push(s);
                }
            }
        }

        return v;
    }

    pub fn read_lines_from_file<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }

    pub fn convert_issuance_into_slip(line: std::string::String) -> Slip {
        let mut iter = line.split_whitespace();
        let tmp = iter.next().unwrap();
        let tmp2 = iter.next().unwrap();
        let typ = iter.next().unwrap();

        let amt: u64 = tmp.parse::<u64>().unwrap();
        let tmp3 = tmp2.as_bytes();

        let mut add: SaitoPublicKey = [0; 33];
        for i in 0..33 {
            add[i] = tmp3[i];
        }

        let mut slip = Slip::new();
        slip.set_publickey(add);
        slip.set_amount(amt);
        if typ.eq("VipOutput") {
            slip.set_slip_type(SlipType::VipOutput);
        }
        if typ.eq("StakerDeposit") {
            slip.set_slip_type(SlipType::StakerDeposit);
        }
        if typ.eq("Normal") {
            slip.set_slip_type(SlipType::Normal);
        }

        return slip;
    }
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
                            eprintln!("Error cleaning up after tests {}", err);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn read_issuance_file_test() {
        let slips = Storage::return_token_supply_slips_from_disk();
        let mut total_issuance = 0;

        for i in 0..slips.len() {
            total_issuance += slips[i].get_amount();
        }

        assert_eq!(total_issuance, MAX_TOKEN_SUPPLY);
    }
}
