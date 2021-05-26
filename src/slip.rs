use secp256k1::PublicKey;
use std::hash::Hash;
use crate::crypto::SECP256K1Hash;

/// A record of owernship of funds on the network
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct InputSlip {
    // tx_id: SECP256K1Hash,
    // slip_id: u64
    block_id: u64,
    tx_id: u64,
    slip_id: u64,
}
/// An object that holds concrete data not subjective to state of chain
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct OutputSlip {
    /// An `Sectp256K::PublicKey` determining who owns the `Slip`
    address: PublicKey,
    /// A enum brodcast type determining how to be processed by consensus
    broadcast_type: SlipBroadcastType,
    /// Amount of Saito
    amount: u64,
}
/// A full slip can be sent to a client so that it can be used to create inputs or to compute
/// a balance or transaction history
pub struct FullSlip {
    hash_id: SECP256K1Hash,
    output: OutputSlip
}
impl InputSlip {
    pub fn new(block_id: u64, tx_id: u64, slip_id: u64) -> InputSlip {
        InputSlip {
            block_id,
            tx_id,
            slip_id,
        }
    }
}

impl OutputSlip {
    /// Create new `OutputSlip`
    ///
    /// * `address` - `Publickey` address to assign ownership
    /// * `broadcast_type` - `SlipBroadcastType` of `OutputSlip`
    /// * `amount` - `u64` amount of Saito contained in the `OutputSlip`
    pub fn new(address: PublicKey, broadcast_type: SlipBroadcastType, amount: u64) -> Self {
        OutputSlip {
            address,
            broadcast_type,
            amount,
        }
    }
    
    /// Returns address in `Slip`
    pub fn address(&self) -> &PublicKey {
        &self.address
    }

    /// Returns`Slip` type from the enumerated set of `SlipBroadcastType`
    pub fn broadcast_type(&self) -> SlipBroadcastType {
        self.broadcast_type
    }

    /// Returns amount of Saito in `Slip`
    pub fn amount(&self) -> u64 {
        self.amount
    }

}

/// An enumerated set of `Slip` types
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipBroadcastType {
    Normal,
}

// impl Slip {
//     /// Create new `Slip`
//     ///
//     /// * `slip_body` - `OutputSlip`
//     /// * `block_id` - `u64`
//     /// * `tx_id` - `u64` ordinal of transaction in serialized `Block`
//     /// * `slip_id` - `u64` ordinal of slip in serialized `Transaction`
//     pub fn new(slip_body: OutputSlip, block_id: u64, tx_id: u64, slip_id: u64) -> Self {
//         Slip {
//             body: slip_body,
//             block_id: block_id,
//             tx_id: tx_id,
//             slip_id: slip_id,
//         }
//     }
// 
//     /// Returns the `Block` id the slip originated from
//     pub fn block_id(&self) -> u64 {
//         self.block_id
//     }
// 
//     /// Returns the `Transaction` id the slip originated from
//     pub fn tx_id(&self) -> u64 {
//         self.tx_id
//     }
// 
//     /// Returns the `Slip`
//     pub fn slip_id(&self) -> u64 {
//         self.slip_id
//     }
// 
//     /// Set the `Block` id
//     pub fn set_block_id(&mut self, block_id: u64) {
//         self.block_id = block_id;
//     }
// 
//     /// Set the `Transaction` id
//     pub fn set_tx_id(&mut self, tx_id: u64) {
//         self.tx_id = tx_id;
//     }
// 
//     /// Set the `Slip` id
//     pub fn set_slip_id(&mut self, slip_id: u64) {
//         self.slip_id = slip_id;
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;

    #[test]
    fn slip_test() {
        let keypair = Keypair::new();
        let slip = OutputSlip::new(
            keypair.public_key().clone(),
            SlipBroadcastType::Normal,
            10_000_000,
        );

        assert_eq!(slip.address(), keypair.public_key());
        assert_eq!(slip.broadcast_type(), SlipBroadcastType::Normal);
        assert_eq!(slip.amount(), 10_000_000);
    }
}
