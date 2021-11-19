use std::convert::TryInto;

use crate::crypto::SaitoHash;
///
/// Data Object for SNDBLKHD
/// `block_hash` - hash of block we wish to inform our peer about
///
#[derive(Debug)]
pub struct SendBlockHeadMessage {
    block_hash: SaitoHash,
}

impl SendBlockHeadMessage {
    pub fn new(block_hash: SaitoHash) -> Self {
        SendBlockHeadMessage { block_hash }
    }

    pub fn deserialize(bytes: &Vec<u8>) -> SendBlockHeadMessage {
        let block_hash: SaitoHash = bytes[0..32].try_into().unwrap();

        SendBlockHeadMessage::new(block_hash)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.block_hash);
        vbytes
    }

    pub fn get_block_hash(&self) -> &SaitoHash {
        &self.block_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_send_block_head_message_serialize() {
        let mock_block_hash = [1; 32];
        let send_block_head_message = SendBlockHeadMessage::new(mock_block_hash);

        let serialized_send_block_head_message = send_block_head_message.serialize();
        let deserialized_send_block_head_message =
            SendBlockHeadMessage::deserialize(&serialized_send_block_head_message);
        assert_eq!(
            send_block_head_message.get_block_hash(),
            deserialized_send_block_head_message.get_block_hash()
        );
    }
}
