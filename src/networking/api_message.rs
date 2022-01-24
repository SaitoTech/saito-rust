use crate::peer::Peer;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// The core data type for transporting data across the Saito Network.
/// See the Network module doc for more details.
///
/// ```bytes
/// ERROR___
/// RESULT__
/// ```
///
/// ```bytes
/// SHAKINIT
/// SHAKCOMP
/// REQCHAIN
/// SNDCHAIN
/// REQBLKHD
/// SNDBLKHD
/// SNDTRANS
/// REQBLOCK
/// SNDKYLST
/// ```
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
            message_id,
            message_data,
        }
    }
    pub fn new_for_peer(message_name: &str, message_data: Vec<u8>, peer: &mut Peer) -> APIMessage {
        peer.request_count += 1;
        APIMessage::new(message_name, peer.request_count - 1, message_data)
    }
    pub fn get_message_name(&self) -> &[u8; 8] {
        &self.message_name
    }
    pub fn get_message_name_as_string(&self) -> String {
        String::from_utf8_lossy(&self.message_name).to_string()
    }
    pub fn get_message_id(&self) -> u32 {
        self.message_id
    }
    pub fn get_message_data(&self) -> &Vec<u8> {
        &self.message_data
    }
    pub fn get_into_message_data(self) -> Vec<u8> {
        self.message_data
    }
    pub fn get_message_data_as_string(&self) -> String {
        String::from_utf8_lossy(&self.message_data).to_string()
    }
    pub fn deserialize(bytes: &[u8]) -> APIMessage {
        let message_name: [u8; 8] = bytes[0..8].try_into().unwrap();
        let message_id: u32 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let message_data = bytes[12..].try_into().unwrap();
        APIMessage {
            message_name,
            message_id,
            message_data,
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

#[cfg(test)]
mod tests {
    use crate::networking::api_message::APIMessage;
    use std::convert::TryInto;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_message_serialize() {
        let api_message = APIMessage {
            message_name: String::from("HLLOWRLD").as_bytes().try_into().unwrap(),
            message_id: 1,
            message_data: String::from("SOMEDATA").as_bytes().try_into().unwrap(),
        };
        let serialized_api_message = api_message.serialize();

        let deserialized_api_message = APIMessage::deserialize(&serialized_api_message);
        assert_eq!(api_message, deserialized_api_message);
    }
}
