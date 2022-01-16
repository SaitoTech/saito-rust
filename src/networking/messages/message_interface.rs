use bincode::Options;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::Write;

pub trait Message:
    Serialize + Deserialize + Send + Sync + Unpin + std::fmt::Debug + 'static
{
    const TYPE_ID: u64;

    // Does message stuff and is called by network
    fn serialize_message<W: WriteBytesExt>(
        &self,
        writer: &mut W,
    ) -> Result<usize, SerializingError> {
        //todo
    }

    fn deserialize_message<R: ReadBytesExt>(reader: &mut R) -> Result<Self, SerializingError> {
        //todo
    }
}

pub trait RequestMessage: Message {
    fn set_request_identifier(&mut self, request_identifier: u32);
}

pub trait ResponseMessage: Message {
    fn get_request_identifier(&self) -> u32;
}
