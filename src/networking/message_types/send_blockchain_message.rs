use crate::crypto::SaitoHash;
use macros::TryFromByte;
use std::convert::{TryFrom, TryInto};

/// Data Object for SNDCHAIN. Is used as a payload in an APIMessage message field.
/// `block_id`
/// `block_hash`
/// `timestamp`
/// (future work) pre_hash: the hash which is hashed with the previous block_hash to generate the hash of the current block.
/// (future work) number of transactions: the number of transactions in the block for the recipient
pub const BLOCKCHAIN_BLOCK_DATA_SIZE: usize = 84;

#[derive(Debug, Copy, PartialEq, Clone, TryFromByte)]
pub enum SyncType {
    Full,
    Lite,
}
#[derive(Debug)]
pub struct SendBlockchainBlockData {
    pub block_id: u64,
    pub block_hash: SaitoHash,
    pub timestamp: u64,
    pub pre_hash: SaitoHash,
    pub number_of_transactions: u32,
}
#[derive(Debug)]
pub struct SendBlockchainMessage {
    sync_type: SyncType,
    starting_hash: SaitoHash,
    blocks_data: Vec<SendBlockchainBlockData>,
}

impl SendBlockchainMessage {
    pub fn new(
        sync_type: SyncType,
        starting_hash: SaitoHash,
        blocks_data: Vec<SendBlockchainBlockData>,
    ) -> Self {
        SendBlockchainMessage {
            sync_type,
            starting_hash,
            blocks_data,
        }
    }

    pub fn get_sync_type(&self) -> &SyncType {
        &self.sync_type
    }
    pub fn get_starting_hash(&self) -> &SaitoHash {
        &self.starting_hash
    }
    pub fn get_blocks_data(&self) -> &Vec<SendBlockchainBlockData> {
        &self.blocks_data
    }
    pub fn deserialize(bytes: &Vec<u8>) -> SendBlockchainMessage {
        let sync_type: SyncType = SyncType::try_from(bytes[0]).unwrap();
        let starting_hash: SaitoHash = bytes[1..33].try_into().unwrap();
        let blocks_data_len: usize = u32::from_be_bytes(bytes[33..37].try_into().unwrap()) as usize;
        let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
        let start_of_block_data = 37;
        for n in 0..blocks_data_len {
            let start_of_data: usize =
                start_of_block_data as usize + n as usize * BLOCKCHAIN_BLOCK_DATA_SIZE;

            let block_id: u64 =
                u64::from_be_bytes(bytes[start_of_data..start_of_data + 8].try_into().unwrap());
            let block_hash: SaitoHash = bytes[start_of_data + 8..start_of_data + 40]
                .try_into()
                .unwrap();
            let timestamp: u64 = u64::from_be_bytes(
                bytes[start_of_data + 40..start_of_data + 48]
                    .try_into()
                    .unwrap(),
            );
            let pre_hash: SaitoHash = bytes[start_of_data + 48..start_of_data + 80]
                .try_into()
                .unwrap();
            let number_of_transactions: u32 = u32::from_be_bytes(
                bytes[start_of_data + 80..start_of_data + 84]
                    .try_into()
                    .unwrap(),
            );
            blocks_data.push(SendBlockchainBlockData {
                block_id,
                block_hash,
                timestamp,
                pre_hash,
                number_of_transactions,
            });
        }
        SendBlockchainMessage {
            sync_type,
            starting_hash,
            blocks_data,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&(self.sync_type as u8).to_be_bytes());
        vbytes.extend(&self.starting_hash);
        vbytes.extend(&(self.blocks_data.len() as u32).to_be_bytes());
        for blocks_data in &self.blocks_data {
            vbytes.extend(&blocks_data.block_id.to_be_bytes());
            vbytes.extend(&blocks_data.block_hash);
            vbytes.extend(&blocks_data.timestamp.to_be_bytes());
            vbytes.extend(&blocks_data.pre_hash);
            vbytes.extend(&blocks_data.number_of_transactions.to_be_bytes());
        }
        vbytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_send_blockchain_message_serialize() {
        let mut blocks_data: Vec<SendBlockchainBlockData> = vec![];
        blocks_data.push(SendBlockchainBlockData {
            block_id: 1,
            block_hash: [1; 32],
            timestamp: 1,
            pre_hash: [1; 32],
            number_of_transactions: 1,
        });
        blocks_data.push(SendBlockchainBlockData {
            block_id: 2,
            block_hash: [2; 32],
            timestamp: 2,
            pre_hash: [2; 32],
            number_of_transactions: 2,
        });
        let send_blockchain_message =
            SendBlockchainMessage::new(SyncType::Full, [1; 32], blocks_data);

        let serialized_send_blockchain_message = send_blockchain_message.serialize();
        let deserialized_send_blockchain_message =
            SendBlockchainMessage::deserialize(&serialized_send_blockchain_message);
        assert_eq!(
            send_blockchain_message.get_sync_type(),
            deserialized_send_blockchain_message.get_sync_type()
        );
        assert_eq!(
            send_blockchain_message.get_starting_hash(),
            deserialized_send_blockchain_message.get_starting_hash()
        );
        let block_data_in = send_blockchain_message.get_blocks_data();
        let block_data_out = deserialized_send_blockchain_message.get_blocks_data();
        assert_eq!(block_data_in[0].block_id, block_data_out[0].block_id);
        assert_eq!(block_data_in[0].timestamp, block_data_out[0].timestamp);
        assert_eq!(block_data_in[0].pre_hash, block_data_out[0].pre_hash);
        assert_eq!(
            block_data_in[0].number_of_transactions,
            block_data_out[0].number_of_transactions
        );
        assert_eq!(block_data_in[1].block_id, block_data_out[1].block_id);
        assert_eq!(block_data_in[1].timestamp, block_data_out[1].timestamp);
        assert_eq!(block_data_in[1].pre_hash, block_data_out[1].pre_hash);
        assert_eq!(
            block_data_in[1].number_of_transactions,
            block_data_out[1].number_of_transactions
        );
    }
}
