use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct APIMessage {
    pub message_name: [u8; 8],
    pub message_id: u32,
    pub message_data: Vec<u8>,
}

impl APIMessage {
    pub fn new(message_name: &str, message_id: u32, message_data: Vec<u8>) -> APIMessage {
        APIMessage {
            message_name: String::from(message_name).as_bytes().try_into().unwrap(),
            message_id: message_id,
            message_data: message_data,
        }
    }
    pub fn message_name(&self) -> &[u8; 8] {
        &self.message_name
    }
    pub fn message_name_as_str(&self) -> String {
        String::from_utf8_lossy(&self.message_name).to_string()
    }
    pub fn message_id(&self) -> u32 {
        self.message_id
    }
    pub fn message_data(&self) -> &Vec<u8> {
        &self.message_data
    }
    pub fn deserialize(bytes: &Vec<u8>) -> APIMessage {
        let message_name: [u8; 8] = bytes[0..8].try_into().unwrap();
        let message_id: u32 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let message_data = bytes[12..].try_into().unwrap();
        APIMessage {
            message_name: message_name,
            message_id: message_id,
            message_data: message_data,
        }
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.message_name);
        vbytes.extend(&self.message_id.to_be_bytes());
        vbytes.extend(&self.message_data);
        vbytes
    }
}
