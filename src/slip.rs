use secp256k1::PublicKey;
use std::convert::{TryFrom, TryInto};
use std::mem::size_of_val;
use std::mem::size_of;
use std::{mem, slice};
pub use enum_variant_count_derive::TryFromByte;

/// A record of owernship of funds on the network
#[derive(Debug, PartialEq, Clone, Copy)]
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
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SlipBody {
    /// An `Sectp256K::PublicKey` determining who owns the `Slip`
    address: PublicKey,
    /// A enum brodcast type determining how to be processed by consensus
    broadcast_type: SlipBroadcastType,
    /// Amount of Saito
    amount: u64,
}

/// An enumerated set of `Slip` types
#[derive(Debug, PartialEq, Clone, Copy, TryFromByte)]
pub enum SlipBroadcastType {
    Normal,
    Other,
    Another,
}





impl Slip {
    /// Create new `Slip` with default type `SlipBroadcastType::Normal`
    ///
    /// * `address` - `Publickey` address to assign ownership
    /// * `broadcast_type` - `SlipBroadcastType` of `Slip`
    /// * `amount` - `u64` amount of Saito contained in the `Slip`
    pub fn deserialize(bytes: [u8; 42]) -> Slip {
        // Slip::new(address: PublicKey, broadcast_type: SlipBroadcastType, amount: u64);
        let public_key: PublicKey = PublicKey::from_slice(&bytes[..33]).unwrap();
        let broadcast_type: SlipBroadcastType = SlipBroadcastType::try_from(bytes[41]).unwrap();
        let mut amount_bytes: [u8;8] = [0;8];
        amount_bytes.clone_from_slice(&bytes[33..41]);
        let amount = u64::from_be_bytes(amount_bytes);
        Slip::new(public_key, broadcast_type, amount)
    }
    
    pub fn serialize(&self) -> [u8; 42] {
        let mut ret = [0; 42];
        let serialized_pubkey: [u8; 33] = self.body.address.serialize();
        ret[..33].clone_from_slice(&serialized_pubkey);
        let serialized_amount: &[u8] = unsafe { 
            slice::from_raw_parts((&self.body.amount as *const u64) as *const u8, mem::size_of::<u64>())
        };
        ret[33..41].clone_from_slice(&serialized_amount);
        let serialized_broadcast_type = self.body.broadcast_type as u8;
        ret[41] = serialized_broadcast_type;
        ret
    }
    
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;
    use rand;
    use std::{mem, slice};
    use std::convert::{TryFrom, TryInto};
    use std::mem::size_of_val;
    use std::mem::size_of;
    
    #[test]
    fn slip_test() {
        let keypair = Keypair::new();
        let block_hash: [u8; 32] = rand::random();
        let mut slip = Slip::new(
            keypair.public_key().clone(),
            SlipBroadcastType::Normal,
            10_000_000,
        );

        assert_eq!(slip.address(), keypair.public_key());
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
    fn slip_serialization_play() {
        
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::from_secret_hex(mock_private_key).unwrap();
        let block_hash: [u8; 32] = [0;32];
        let amount = std::u64::MAX;
        let mut slip = Slip::new(
            *keypair.public_key(),
            SlipBroadcastType::Another,
            amount,
        );
        let serialized_slip = slip.serialize();
        println!("{:?}", serialized_slip);
        
        
        
        let testbytes: [u8; 32] = [12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35];
        
        let p: *const PublicKey = &slip.body.address;
        let p: *const u8 = p as *const u8;
        let s: &[u8] = unsafe { 
            slice::from_raw_parts(p, mem::size_of::<PublicKey>())
        };
        println!("from_raw_parts u64: {:?}", s);
        println!("            length: {:?}", s.len());
        
        //[u8; constants::PUBLIC_KEY_SIZE]
        let serialized_pubkey: [u8; 33] = slip.body.address.serialize();
        println!("from_raw_parts u64: {:?}", serialized_pubkey);
        println!("            length: {:?}", serialized_pubkey.len());
        
        let slip_type: SlipBroadcastType = SlipBroadcastType::Normal;
        //let length = std::ptr::read(slip_type.as_ptr()); 
        let val3: SlipBroadcastType = (2 as u8).try_into().unwrap();
        dbg!(val3);
        dbg!(SlipBroadcastType::Another as u32);
        
        let amount: u64 = 256*256*256*256*256*256*256;
        
        let s: &[u8] = unsafe { 
            slice::from_raw_parts((&amount as *const u64) as *const u8, mem::size_of::<u64>())
        };
        println!("from_raw_parts u64: {:?}", s);
        
        //SlipBroadcastType::debug();
    }
    
    
}


