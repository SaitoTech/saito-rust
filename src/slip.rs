use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// An enumerated set of `Slip` types
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipBroadcastType {
    Normal,
}

/// A record of owernship of funds on the network
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct SlipID {
    /// `Block` id
    block_id: u64,
    /// `Transaction` id
    tx_id: u64,
    /// `Slip` id
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

impl SlipID {
    /// Create new `SlipID`
    pub fn new(block_id: u64, tx_id: u64, slip_id: u64) -> SlipID {
        SlipID {
            block_id: block_id,
            tx_id: tx_id,
            slip_id: slip_id,
        }
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
}

impl OutputSlip {
    /// Create new `OutputSlip`
    pub fn new(address: PublicKey, broadcast_type: SlipBroadcastType, amount: u64) -> OutputSlip {
        OutputSlip {
            address: address,
            broadcast_type: broadcast_type,
            amount: amount,
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
