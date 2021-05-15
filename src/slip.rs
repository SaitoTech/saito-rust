use serde::{Serialize, Deserialize};
use secp256k1::PublicKey;

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


// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
// pub struct SlipBody2 {
//     /// An `Sectp256K::PublicKey` determining who owns the `Slip`
//     address: [u8;32],
// }

/// An enumerated set of `Slip` types
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum SlipBroadcastType {
    Normal,
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

    #[test]
    fn slip_test() {
        let block_hash: [u8; 32] = rand::random();
        let keypair = Keypair::new();
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
    fn slip_serialization() {
        let mock_private_key = "da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16";
        let keypair = Keypair::from_secret_hex(mock_private_key).unwrap();
        let block_hash: [u8; 32] = [0;32];
        let mut slip = Slip::new(
            keypair.public_key().clone(),
            SlipBroadcastType::Normal,
            10_000_000,
        );
        // let xs: Vec<u8> = bincode::serialize(&slip.body).unwrap();
        //
        // println!("Slip:\n{:?}\n serializes into byte array \n{:?}", slip, xs);
        // println!("lenght: {}", xs.len());
        // // let result = match IntoIterator::into_iter(xs) {
        // //     mut iter => loop {
        // //         let next;
        // //         match iter.next() {
        // //             Some(val) => next = val,
        // //             None => break,
        // //         };
        // //         let x = next;
        // //         let () = { println!("{}", x); };
        // //     }
        // // };
        //
        //
        // let xd: SlipBody = bincode::deserialize(&xs).unwrap();
        // println!("BincodedSlip:\n{:?}\n deserializes into \n{:?}", xs, xd);


        let testbytes: [u8; 32] = [12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35, 12, 11, 23, 32, 31, 42, 43, 35];
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();

        println!("{:?}", serout);
        println!("{:?}", serout.len());

        let testbytes: u64 = 10000000;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);

        let testbytes: u32 = 10000000;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);

        let testbytes: i64 = 10000000;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);

        let testbytes: i32 = 10000000;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);

        let testbytes: f64 = 10000000.0;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);

        let testbytes: f32 = 0.0000000000000000000000000000000001;
        println!("{}", testbytes);
        let testbytes: f32 = 1.0000000000000000000000000000000001;
        println!("{}", testbytes);
        let testbytes: f32 = 10000000000000000000000000000.9 - 10000000000000000000000000000.0;
        println!("{}", testbytes);
        let testbytes: f32 = 10000000.9 - 10000000.0;
        println!("{}", testbytes);

        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("{:?}", serout);
        let xd: f32 = bincode::deserialize(&serout).unwrap();
        println!("{}", xd);

        let testbytes: SlipBroadcastType = SlipBroadcastType::Normal;
        let serout: Vec<u8> = bincode::serialize(&testbytes).unwrap();
        println!("SlipBroadcastType {:?}", serout);
    }

    #[test]
    fn slip_bincode_serialization() {
        let keypair = Keypair::from_secret_hex("da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16").unwrap();
        let slip = Slip::new(
            keypair.public_key().clone(),
            SlipBroadcastType::Normal,
            10_000_000,
        );

        let serout: Vec<u8> = bincode::serialize(&slip).unwrap();
        println!("{:?}", serout.len());
    }

    #[test]
    fn slip_cbor_serialization() {
        let keypair = Keypair::from_secret_hex("da79fe6d86347e8f8dc71eb3dbab9ba5623eaaed6c5dd0bb257c0d631faaff16").unwrap();
        let slip = Slip::new(
            keypair.public_key().clone(),
            SlipBroadcastType::Normal,
            10_000_000,
        );

        let bytes = serde_cbor::to_vec(&slip).unwrap();
        println!("{:?}", bytes.len());
    }
}

