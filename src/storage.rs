use std::{
    fs::{self, File},
    io::{self, Read, Write, BufRead},
    path::Path,
    sync::Arc,
};
use crate::slip::{Slip, SlipType};
use crate::crypto::{SaitoPublicKey};
use crate::blockchain::{MAX_TOKEN_SUPPLY};

use tokio::sync::RwLock;

use crate::{block::Block, blockchain::Blockchain, crypto::SaitoHash};

pub const BLOCKS_DIR_PATH: &'static str = "./data/blocks/";
pub const ISSUANCE_FILE_PATH: &'static str = "./data/issuance/issuance";
pub const EARLYBIRDS_FILE_PATH: &'static str = "./data/issuance/earlybirds";
pub const DEFAULT_FILE_PATH: &'static str = "./data/issuance/default";

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
        let mut filename = String::from(BLOCKS_DIR_PATH).clone();

        filename.push_str(&hex::encode(block.get_timestamp().to_be_bytes()));
        filename.push_str(&String::from("-"));
        filename.push_str(&hex::encode(&block.get_hash()));
        filename.push_str(&".sai");
        filename
    }
    pub fn write_block_to_disk(block: &mut Block) {
        let filename = Storage::generate_block_filename(block);
        if !Path::new(&filename).exists() {
            let mut buffer = File::create(filename).unwrap();
            let byte_array: Vec<u8> = block.serialize_for_net();
            buffer.write_all(&byte_array[..]).unwrap();
        }
    }

    pub async fn stream_block_from_disk(block_hash: SaitoHash) -> io::Result<Vec<u8>> {
        let mut filename = String::from(BLOCKS_DIR_PATH).clone();

        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".sai");

        let mut f = tokio::fs::File::open(filename).await?;
        let mut encoded = Vec::<u8>::new();
        tokio::io::AsyncReadExt::read_to_end(&mut f, &mut encoded).await?;

        Ok(encoded)
    }

    pub async fn stream_json_block_from_disk(block_hash: SaitoHash) -> io::Result<String> {
        let mut filename = String::from(BLOCKS_DIR_PATH).clone();
        filename.push_str(&"json/");
        filename.push_str(&hex::encode(block_hash));
        filename.push_str(&".json");

        fs::read_to_string(filename)
    }

    pub async fn load_blocks_from_disk(blockchain_lock: Arc<RwLock<Blockchain>>) {
        let mut paths: Vec<_> = fs::read_dir(String::from(BLOCKS_DIR_PATH).clone())
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
            if path.path().to_str().unwrap() != String::from(BLOCKS_DIR_PATH).clone() + "empty"
                && path.path().to_str().unwrap()
                    != String::from(BLOCKS_DIR_PATH).clone() + ".gitignore"
            {
                let mut f = File::open(path.path()).unwrap();
                let mut encoded = Vec::<u8>::new();
                f.read_to_end(&mut encoded).unwrap();
                let mut block = Block::deserialize_for_net(&encoded);

                //
                // the hash needs calculation separately after loading
                //
                if block.get_hash() == [0; 32] {
                    block.generate_hashes();
                }

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
        let mut block = Block::deserialize_for_net(&encoded);

        //
        // the hash needs calculation separately after loading
        //
        if block.get_hash() == [0; 32] {
            block.generate_hashes();
        }

        return block;
    }

    pub async fn delete_block_from_disk(filename: String) -> bool {
        let _res = std::fs::remove_file(filename);
        true
    }



    //
    // token issuance functions below
    //
    pub fn return_token_supply_slips_from_disk() -> Vec<Slip> {

	let mut v : Vec<Slip> = vec![];
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

	for i in 0 .. v.len() { tokens_issued += v[i].get_amount(); }

        if let Ok(lines) = Storage::read_lines_from_file(DEFAULT_FILE_PATH) {
            for line in lines {
                if let Ok(ip) = line {
		    let mut s = Storage::convert_issuance_into_slip(ip);
		    s.set_amount(MAX_TOKEN_SUPPLY-tokens_issued);
println!("DEFA {}", s.get_amount());
		    v.push(s);
                }
            }
        }
	
	return v;
    }

    pub fn read_lines_from_file<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }

    pub fn convert_issuance_into_slip(line : std::string::String) -> Slip {
        let mut iter = line.split_whitespace();
	let tmp = iter.next().unwrap();
	let tmp2 = iter.next().unwrap();
	let typ = iter.next().unwrap();

	let amt : u64 = tmp.parse::<u64>().unwrap();
	let tmp3 = tmp2.as_bytes();

	let mut add : SaitoPublicKey = [0; 33];
	for i in 0..33 { add[i] = tmp3[i]; }

	let mut slip = Slip::new();
        slip.set_publickey(add);
        slip.set_amount(amt);
	if typ.eq("VipOutput") { slip.set_slip_type(SlipType::VipOutput); }
	if typ.eq("StakerDeposit") { slip.set_slip_type(SlipType::StakerDeposit); }
	if typ.eq("Normal") { slip.set_slip_type(SlipType::Normal); }
	
	return slip;
    }

}

pub trait Persistable {
    fn save(&self);
    fn load(filename: &str) -> Self;
}



#[cfg(test)]
mod tests {

    use super::*;

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



