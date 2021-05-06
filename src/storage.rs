use std::fs::{File};
use std::io::prelude::*;
use std::str;

use data_encoding::HEXLOWER;

use crate::block::{Block, BlockBody};
// use crate::helper::create_timestamp;

pub const BLOCKS_DIR: &str = "./data/blocks/";

pub struct Storage {
    pub dest: String,
    pub blocks_dir: String,
}

impl Storage {
    pub fn new() -> Storage {
        return Storage {
            dest: String::from("data"),
            blocks_dir: String::from("./data/blocks"),
        }
    }

    pub fn write_block_to_disk(blk: Block) {
        let mut filename = String::from(BLOCKS_DIR);

        // filename.push_str(&create_timestamp().to_string());
        // filename.push_str(&"-");
        filename.push_str(&blk.get_bsh_as_hex());
        filename.push_str(&".sai");

        println!("FILENAME: {}", filename);

        let encode: Vec<u8> = bincode::serialize(blk.get_body()).unwrap();
        let mut f = File::create(filename).unwrap();
        f.write_all(&encode[..]).unwrap();
    }

    pub fn read_block_from_disk(bsh: [u8; 32]) -> Block {
        let mut encoded = Vec::<u8>::new();
        let mut filename = String::from(BLOCKS_DIR);

        filename.push_str(&HEXLOWER.encode(&bsh));
        filename.push_str(&".sai");

        println!("ATTEMPTING TO FETCH BLOCK FROM DISK {}", filename);
        let mut r = File::open(filename)
          .expect("Could not find block at this location");

        r.read_to_end(&mut encoded).unwrap();
        let body: BlockBody = bincode::deserialize(&encoded[..]).unwrap();

        return Block::create_from_block_body(body);
    }
}
