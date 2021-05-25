use serde::{Serialize, Deserialize};
use secp256k1::PublicKey;
use crate::slip_proto as proto;
use std::convert::{TryFrom, TryInto};
use std::{mem, slice};
pub use enum_variant_count_derive::TryFromByte;

/// A record of owernship of funds on the network
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct Slip {
    /// Contains concrete data which is not relative to state of the chain
    body: SlipBody,
    /// `Block` id
    block_id: u64,
    /// `Transaction` id
    tx_id: u64,
    /// `Slip` id
    slip_id: u64,
    /// The `Block` hash the slip originated from
    block_hash: [u8; 32],
}

/// An object that holds concrete data not subjective to state of chain
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct SlipBody {
    /// An `Sectp256K::PublicKey` determining who owns the `Slip`
    address: PublicKey,
    /// A enum brodcast type determining how to be processed by consensus
    broadcast_type: SlipBroadcastType,
    /// Amount of Saito
    amount: u64,
}

/// An enumerated set of `Slip` types
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, TryFromByte)]
pub enum SlipBroadcastType {
    Normal,
    Other, // Added a few more values here just for testing purposes
    Another, // TODO: DELETE THESE
}

impl Slip {
    /// Create new `Slip` with default type `SlipBroadcastType::Normal`
    ///
    /// * `address` - `Publickey` address to assign ownership
    /// * `broadcast_type` - `SlipBroadcastType` of `Slip`
    /// * `amount` - `u64` amount of Saito contained in the `Slip`
    pub fn new(address: PublicKey, broadcast_type: SlipBroadcastType, amount: u64) -> Slip {
        return Slip {
            body: SlipBody {
                address,
                broadcast_type,
                amount,
            },
            block_id: 0,
            tx_id: 0,
            slip_id: 0,
            block_hash: [0; 32],
        };
    }
    
    pub fn deserialize(bytes: [u8; 42]) -> Slip {
        let public_key: PublicKey = PublicKey::from_slice(&bytes[..33]).unwrap();
        let broadcast_type: SlipBroadcastType = SlipBroadcastType::try_from(bytes[41]).unwrap();
        let amount = u64::from_be_bytes(bytes[33..41].try_into().unwrap());
        Slip::new(public_key, broadcast_type, amount)
    }
    
    pub fn serialize(&self) -> [u8; 42] {
        let mut ret = [0; 42];
        ret[..33].clone_from_slice(&self.body.address.serialize());
        unsafe {
            ret[33..41].clone_from_slice(&slice::from_raw_parts((&self.body.amount as *const u64) as *const u8, mem::size_of::<u64>()));
        }
        ret[41] = self.body.broadcast_type as u8;
        ret
    }
    /// Returns address in `Slip`
    pub fn address(&self) -> &PublicKey {
        &self.body.address
    }

    /// Returns`Slip` type from the enumerated set of `SlipBroadcastType`
    pub fn broadcast_type(&self) -> SlipBroadcastType {
        self.body.broadcast_type
    }

    /// Returns amount of Saito in `Slip`
    pub fn amount(&self) -> u64 {
        self.body.amount
    }

    /// Returns the `Block` id the slip originated from
    pub fn block_id(&self) -> u64 {
        self.block_id
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn tx_id(&self) -> u64 {
        self.tx_id
    }

    /// Returns the `Slip`
    pub fn slip_id(&self) -> u64 {
        self.slip_id
    }

    /// Returns the `Block` hash the slip originated from
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    // Set the `Block` id
    pub fn set_block_id(&mut self, block_id: u64) {
        self.block_id = block_id;
    }

    // Set the `Transaction` id
    pub fn set_tx_id(&mut self, tx_id: u64) {
        self.tx_id = tx_id;
    }

    // Set the `Slip` id
    pub fn set_slip_id(&mut self, slip_id: u64) {
        self.slip_id = slip_id;
    }

    // Set the `Block` hash
    pub fn set_block_hash(&mut self, block_hash: [u8; 32]) {
        self.block_hash = block_hash;
    }
    
    
}
impl Into<proto::Slip> for Slip {
    fn into(self) -> proto::Slip {
        proto::Slip {
            body: Some(proto::SlipBody {
                address: self.body.address.serialize().to_vec(),
                broadcast_type: self.broadcast_type() as i32,
                amount: self.body.amount,
            }),
            block_id: self.block_id,
            tx_id: self.tx_id,
            slip_id: self.slip_id,
            block_hash: self.block_hash.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand;
    use std::str::FromStr;
    #[test]
    fn slip_test() {
        let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        let block_hash: [u8; 32] = rand::random();
        let mut slip = Slip::new(
            public_key,
            SlipBroadcastType::Normal,
            10_000_000,
        );

        assert_eq!(slip.address(), &public_key);
        assert_eq!(slip.broadcast_type(), SlipBroadcastType::Normal);
        assert_eq!(slip.amount(), 10_000_000);
        assert_eq!(slip.block_id(), 0);
        assert_eq!(slip.tx_id(), 0);
        assert_eq!(slip.slip_id(), 0);
        assert_eq!(slip.block_hash(), [0; 32]);

        slip.set_block_id(10);
        slip.set_tx_id(10);
        slip.set_slip_id(10);
        slip.set_block_hash(block_hash);

        assert_eq!(slip.block_id(), 10);
        assert_eq!(slip.tx_id(), 10);
        assert_eq!(slip.slip_id(), 10);
        assert_eq!(slip.block_hash(), block_hash);
    }
    
    #[test]
    fn slip_custom_serialization() {
        let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        let amount = std::u64::MAX;
        let slip = Slip::new(
            public_key,
            SlipBroadcastType::Another,
            amount,
        );
        let serialized_slip: [u8; 42] = slip.serialize();
        let deserialized_slip: Slip = Slip::deserialize(serialized_slip);
        assert_eq!(slip, deserialized_slip);
        println!("{}", slip.body.address);
    }
    
    #[test]
    fn slip_bincode_serialization() {
        let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        let slip = Slip::new(
            public_key,
            SlipBroadcastType::Normal,
            10_000_000,
        );

        let serout: Vec<u8> = bincode::serialize(&slip).unwrap();
        println!("{:?}", serout.len());
    }

    #[test]
    fn slip_cbor_serialization() {
        let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
        let slip = Slip::new(
            public_key,
            SlipBroadcastType::Normal,
            10_000_000,
        );

        let bytes = serde_cbor::to_vec(&slip).unwrap();
        println!("{:?}", bytes.len());
    }
    
    #[test]
    fn slip_proto_serialization() {
        
    }
}


