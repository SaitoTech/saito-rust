use std::convert::TryInto;

use crate::crypto::SaitoHash;

/// - block_id(starts with latest)
/// - block_hash(starts with latest)
/// - fork_id
/// - synctype(optional)

#[derive(Debug)]
pub struct RequestBlockchainMessage {
    latest_block_id: u64,
    latest_block_hash: SaitoHash,
    fork_id: SaitoHash,
}

impl RequestBlockchainMessage {
    pub fn new(latest_block_id: u64, latest_block_hash: SaitoHash, fork_id: SaitoHash) -> Self {
        RequestBlockchainMessage {
            latest_block_id,
            latest_block_hash,
            fork_id,
        }
    }

    pub fn deserialize(bytes: &Vec<u8>) -> RequestBlockchainMessage {
        let latest_block_id: u64 = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let latest_block_hash: SaitoHash = bytes[8..40].try_into().unwrap();
        let fork_id: SaitoHash = bytes[40..72].try_into().unwrap();

        RequestBlockchainMessage::new(latest_block_id, latest_block_hash, fork_id)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.latest_block_id.to_be_bytes());
        vbytes.extend(&self.latest_block_hash);
        vbytes.extend(&self.fork_id);
        vbytes
    }
    pub fn get_latest_block_id(&self) -> u64 {
        self.latest_block_id
    }
    pub fn get_latest_block_hash(&self) -> &SaitoHash {
        &self.latest_block_hash
    }
    pub fn get_fork_id(&self) -> &SaitoHash {
        &self.fork_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_blockchain_message_serialize() {
        let request_blockchain_message = RequestBlockchainMessage::new(50, [42; 32], [42; 32]);

        let serialized_request_blockchain_message = request_blockchain_message.serialize();
        let deserialized_request_blockchain_message =
            RequestBlockchainMessage::deserialize(&serialized_request_blockchain_message);
        assert_eq!(
            request_blockchain_message.get_latest_block_id(),
            deserialized_request_blockchain_message.get_latest_block_id()
        );
        assert_eq!(
            request_blockchain_message.get_latest_block_hash(),
            deserialized_request_blockchain_message.get_latest_block_hash()
        );
        assert_eq!(
            request_blockchain_message.get_fork_id(),
            deserialized_request_blockchain_message.get_fork_id()
        );
    }
}
