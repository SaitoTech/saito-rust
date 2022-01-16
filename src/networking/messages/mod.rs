pub(crate) mod handlers;
mod message_interface;
mod request_response;

use secp256k1::serde::ser::{Error, Ok};
use secp256k1::serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};

use crate::crypto::SaitoHash;
use crate::networking::messages::message_interface::Message;
use crate::request_response;

///
/// Data Object for SNDBLKHD
/// `block_hash` - hash of block we wish to inform our peer about
///
// #[derive(Debug)]
#[derive(Clone, Serialize, Deserialize)]
pub struct SendBlockHeadMessage {
    block_hash: SaitoHash,
    pub request_identifier: u32,
}
request_response!(SendBlockHeadMessage);

impl Serialize for SendBlockHeadMessage {
    fn serialize<S>(&self, serializer: S) -> Result<Ok, Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

impl Deserialize for SendBlockHeadMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, serde::de::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

impl Debug for SendBlockHeadMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Message for SendBlockHeadMessage {
    const TYPE_ID: u64 = 201;
}
