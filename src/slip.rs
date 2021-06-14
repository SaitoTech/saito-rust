use crate::crypto::Sha256Hash;
use secp256k1::PublicKey;
pub use secp256k1::Signature;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// An enumerated set of `Slip` types
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}

/// A record of owernship of funds on the network
impl Hash for SlipID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.slip_ordinal.to_ne_bytes());
        state.write(&self.tx_id)
    }
}

/// A record of owernship of funds on the network
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub struct SlipID {
    tx_id: Sha256Hash,
    /// `Slip` id
    slip_ordinal: u64,
}

/// An object that holds concrete data not subjective to state of chain
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub struct OutputSlip {
    /// An `Secp256K1::PublicKey` determining who owns the `Slip`
    address: PublicKey,
    /// A enum brodcast type determining how to be processed by consensus
    broadcast_type: SlipType,
    /// Amount of Saito
    amount: u64,
}

impl SlipID {
    /// Create new `SlipID`
    pub fn default() -> Self {
        SlipID {
            tx_id: [0; 32],
            slip_ordinal: 0,
        }
    }
    /// Create new `SlipID`
    pub fn new(tx_id: Sha256Hash, slip_ordinal: u64) -> Self {
        SlipID {
            tx_id,
            slip_ordinal,
        }
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn tx_id(&self) -> Sha256Hash {
        self.tx_id
    }

    /// Returns the `Slip`
    pub fn slip_ordinal(&self) -> u64 {
        self.slip_ordinal
    }
}

impl OutputSlip {
    /// Create new `OutputSlip`
    pub fn default() -> OutputSlip {
        let public_key: PublicKey = PublicKey::from_str(
            "0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",
        )
        .unwrap();
        OutputSlip {
            address: public_key,
            broadcast_type: SlipType::Normal,
            amount: 10,
        }
    }
    /// Create new `OutputSlip`
    pub fn new(address: PublicKey, broadcast_type: SlipType, amount: u64) -> OutputSlip {
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

    /// Returns`Slip` type from the enumerated set of `SlipType`
    pub fn broadcast_type(&self) -> SlipType {
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
    fn output_slip_test() {
        let keypair = Keypair::new();
        let slip = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 10_000_000);

        assert_eq!(slip.address(), keypair.public_key());
        assert_eq!(slip.broadcast_type(), SlipType::Normal);
        assert_eq!(slip.amount(), 10_000_000);
    }

    #[test]
    fn input_slip_test() {
        let input = SlipID::new([0; 32], 0);
        assert_eq!(input.tx_id(), [0; 32]);
        assert_eq!(input.slip_ordinal(), 0);
    }
}
