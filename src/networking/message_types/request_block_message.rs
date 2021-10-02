use std::convert::{TryFrom, TryInto};

use crate::crypto::SaitoHash;

use super::send_blockchain_message::SyncType;

/// Data Object for REQBLOCK. Is used as a  payload in an APIMessage message field.
/// - `block_id` - (optional)
/// - `block_hash` - (optional)
/// - `synctype` - Full or Lite(SPV)(optional)
pub struct RequestBlockMessage {
    block_id: Option<u64>,
    block_hash: Option<SaitoHash>,
    sync_type: Option<SyncType>,
}

/// a bitmask indicating which fields are filled in the RequestBlockMessage.
pub type RequestBlockMessageOptionsMask = u8;

pub const BLOCK_ID_MASK: u8 = 1;
pub const BLOCK_HASH_MASK: u8 = 2;
pub const SYNC_TYPE_MASK: u8 = 4;

trait RequestBlockMessageOptionsMaskTrait {
    fn new(has_block_id: bool, has_block_hash: bool, has_sync_type: bool) -> Self;
    fn has_block_id(&self) -> bool;
    fn has_block_hash(&self) -> bool;
    fn has_sync_type(&self) -> bool;
}
impl RequestBlockMessageOptionsMaskTrait for RequestBlockMessageOptionsMask {
    fn new(has_block_id: bool, has_block_hash: bool, has_sync_type: bool) -> Self {
        let mut ret = 0;
        if has_block_id {
            ret += BLOCK_ID_MASK
        }
        if has_block_hash {
            ret += BLOCK_HASH_MASK
        }
        if has_sync_type {
            ret += SYNC_TYPE_MASK
        }
        ret
    }
    fn has_block_id(&self) -> bool {
        self & BLOCK_ID_MASK == BLOCK_ID_MASK
    }
    fn has_block_hash(&self) -> bool {
        self & BLOCK_HASH_MASK == BLOCK_HASH_MASK
    }
    fn has_sync_type(&self) -> bool {
        self & SYNC_TYPE_MASK == SYNC_TYPE_MASK
    }
}

impl RequestBlockMessage {
    pub fn new(
        block_id: Option<u64>,
        block_hash: Option<SaitoHash>,
        sync_type: Option<SyncType>,
    ) -> Self {
        RequestBlockMessage {
            block_id,
            block_hash,
            sync_type,
        }
    }

    pub fn deserialize(bytes: &Vec<u8>) -> RequestBlockMessage {
        let request_block_message_options_mask: RequestBlockMessageOptionsMask = bytes[0];
        let mut block_id = None;
        let mut block_hash = None;
        let mut sync_type = None;
        if request_block_message_options_mask.has_block_id() {
            block_id = Some(u64::from_be_bytes(bytes[1..9].try_into().unwrap()));
        }
        if request_block_message_options_mask.has_block_hash() {
            block_hash = Some(bytes[9..41].try_into().unwrap());
        }
        if request_block_message_options_mask.has_sync_type() {
            sync_type = Some(SyncType::try_from(bytes[41]).unwrap());
        }
        RequestBlockMessage::new(block_id, block_hash, sync_type)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        let request_block_message_options_mask = RequestBlockMessageOptionsMask::new(
            self.block_id.is_some(),
            self.block_hash.is_some(),
            self.sync_type.is_some(),
        );
        vbytes.push(request_block_message_options_mask);
        if self.block_id.is_some() {
            vbytes.extend(&self.block_id.unwrap().to_be_bytes());
        } else {
            vbytes.extend(vec![0; 8]);
        }
        if self.block_hash.is_some() {
            vbytes.extend(&self.block_hash.unwrap());
        } else {
            vbytes.extend(vec![0; 32]);
        }
        if self.sync_type.is_some() {
            vbytes.extend(&(self.sync_type.unwrap() as u8).to_be_bytes());
        } else {
            vbytes.extend(vec![0]);
        }
        vbytes
    }
    pub fn get_block_id(&self) -> Option<u64> {
        self.block_id
    }
    pub fn get_block_hash(&self) -> &Option<SaitoHash> {
        &self.block_hash
    }
    pub fn get_fork_id(&self) -> &Option<SyncType> {
        &self.sync_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_block_message_serialize() {
        let request_block_message_empty = RequestBlockMessage::new(None, None, None);
        let request_block_message_full =
            RequestBlockMessage::new(Some(1), Some([42; 32]), Some(SyncType::Full));
        let serialized_request_block_message_empty = request_block_message_empty.serialize();
        let serialized_request_block_message_full = request_block_message_full.serialize();
        assert_eq!(0, serialized_request_block_message_empty[0]);
        assert_eq!(7, serialized_request_block_message_full[0]);
        assert_eq!(serialized_request_block_message_empty.len(), 42);
        assert_eq!(serialized_request_block_message_full.len(), 42);
        let deserialized_request_block_message_empty =
            RequestBlockMessage::deserialize(&serialized_request_block_message_empty);
        let deserialized_request_block_message_full =
            RequestBlockMessage::deserialize(&serialized_request_block_message_full);
        assert_eq!(
            request_block_message_empty.get_block_id(),
            deserialized_request_block_message_empty.get_block_id()
        );
        assert_eq!(
            request_block_message_full.get_block_id(),
            deserialized_request_block_message_full.get_block_id()
        );
        assert_eq!(
            request_block_message_empty.get_block_hash(),
            deserialized_request_block_message_empty.get_block_hash()
        );
        assert_eq!(
            request_block_message_full.get_block_hash(),
            deserialized_request_block_message_full.get_block_hash()
        );
        assert_eq!(
            request_block_message_empty.get_fork_id(),
            deserialized_request_block_message_empty.get_fork_id()
        );
        assert_eq!(
            request_block_message_full.get_fork_id(),
            deserialized_request_block_message_full.get_fork_id()
        );
    }
}
