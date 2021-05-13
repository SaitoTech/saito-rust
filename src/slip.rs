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

/// An enumerated set of `Slip` types
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum SlipBroadcastType {
    Normal,
}

impl Slip {
    /// Create new `Slip` with default type `SlipBroadcastType::Normal`
    ///
    /// * `address` - `Publickey` addressress to assign ownership
    /// * `broadcast_type` - `SlipBroadcastType` of `Slip`
    /// * `amount` - `u64` amount of Saito coantined in the `Slip`
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

    /// Returns addressress in `Slip`
    pub fn get_addressress(&self) -> PublicKey {
        return self.body.address;
    }

    /// Returns`Slip` type from the enumerated set of `SlipBroadcastType`
    pub fn get_type(&self) -> SlipBroadcastType {
        return self.body.broadcast_type;
    }

    /// Returns amount of Saito in `Slip`
    pub fn get_amount(&self) -> u64 {
        return self.body.amount;
    }

    /// Returns the `Block` id the slip originated from
    pub fn get_block_id(&self) -> u64 {
        return self.block_id;
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn get_tx_id(&self) -> u64 {
        return self.tx_id;
    }

    /// Returns the `Slip`
    pub fn get_slip_id(&self) -> u64 {
        return self.slip_id;
    }

    /// Returns the `Block` hash the slip originated from
    pub fn get_block_hash(&self) -> [u8; 32] {
        return self.block_hash;
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
        let keypair = Keypair::new().unwrap();
        let mut slip = Slip::new(
            keypair.get_public_key(),
            SlipBroadcastType::Normal,
            10_000_000,
        );

        assert_eq!(slip.get_addressress(), keypair.get_public_key());
        assert_eq!(slip.get_type(), SlipBroadcastType::Normal);
        assert_eq!(slip.get_amount(), 10_000_000);
        assert_eq!(slip.get_block_id(), 0);
        assert_eq!(slip.get_tx_id(), 0);
        assert_eq!(slip.get_slip_id(), 0);
        assert_eq!(slip.get_block_hash(), [0; 32]);

        slip.set_block_id(10);
        slip.set_tx_id(10);
        slip.set_slip_id(10);
        slip.set_block_hash(block_hash);

        assert_eq!(slip.get_block_id(), 10);
        assert_eq!(slip.get_tx_id(), 10);
        assert_eq!(slip.get_slip_id(), 10);
        assert_eq!(slip.get_block_hash(), block_hash);
    }
}

