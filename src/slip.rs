use crate::crypto::{SaitoPublicKey, SaitoSignature, SaitoUTXOSetKey};
use serde::{Deserialize, Serialize};

use ahash::AHashMap;

use enum_variant_count_derive::TryFromByte;
use std::convert::{TryFrom, TryInto};

/// The size of a serilized slip in bytes.
pub const SLIP_SIZE: usize = 107;

/// SlipType is a human-readable indicator of the slip-type, such
/// as in a normal transaction, a VIP-transaction, a rebroadcast
/// transaction or a golden ticket, etc.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, TryFromByte)]
pub enum SlipType {
    Normal,
    Other, // need more than one value for TryFromBytes
}

/// SlipCore is a self-contained object containing only the minimal
/// information needed about a slip. It exists to simplify block
/// serialization and deserialization until we have custom functions
/// that handle this at speed.
///
/// This is a private variable. Access to variables within the SlipCore
/// should be handled through getters and setters in the block which
/// surrounds it.
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SlipCore {
    #[serde_as(as = "[_; 33]")]
    publickey: SaitoPublicKey,
    #[serde_as(as = "[_; 64]")]
    uuid: SaitoSignature,
    amount: u64,
    slip_ordinal: u8,
    slip_type: SlipType,
}

impl SlipCore {
    pub fn new(
        publickey: [u8; 33],
        uuid: [u8; 64],
        amount: u64,
        slip_ordinal: u8,
        slip_type: SlipType,
    ) -> Self {
        Self {
            publickey,
            uuid,
            amount,
            slip_ordinal,
            slip_type,
        }
    }
}

impl Default for SlipCore {
    fn default() -> Self {
        Self::new([0; 33], [0; 64], 0, 0, SlipType::Normal)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Slip {
    core: SlipCore,
}

impl Slip {
    pub fn new(core: SlipCore) -> Self {
        Self { core }
    }

    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.core.publickey
    }

    pub fn get_amount(&self) -> u64 {
        self.core.amount
    }

    pub fn get_uuid(&self) -> SaitoSignature {
        self.core.uuid
    }

    pub fn get_slip_ordinal(&self) -> u8 {
        self.core.slip_ordinal
    }

    pub fn get_slip_type(&self) -> SlipType {
        self.core.slip_type
    }

    pub fn set_publickey(&mut self, publickey: SaitoPublicKey) {
        self.core.publickey = publickey;
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.core.amount = amount;
    }

    pub fn set_uuid(&mut self, uuid: SaitoSignature) {
        self.core.uuid = uuid;
    }

    pub fn set_slip_ordinal(&mut self, slip_ordinal: u8) {
        self.core.slip_ordinal = slip_ordinal;
    }

    pub fn set_slip_type(&mut self, slip_type: SlipType) {
        self.core.slip_type = slip_type;
    }

    pub fn serialize_for_signature(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.publickey);
        vbytes.extend(&self.core.uuid);
        vbytes.extend(&self.core.amount.to_be_bytes());
        vbytes.extend(&(self.core.slip_type as u32).to_be_bytes());
        vbytes
    }

    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        _lc: bool,
        slip_value: u64,
    ) {
        let slip_key = self.get_utxoset_key();
        utxoset.entry(slip_key).or_insert(slip_value);
    }

    pub fn get_utxoset_key(&self) -> SaitoUTXOSetKey {
        let mut res: Vec<u8> = vec![];
        res.extend(&self.get_publickey());
        res.extend(&self.get_uuid());
        res.extend(&self.get_amount().to_be_bytes());
        res
    }

    pub fn validate(&self) -> bool {
        true
    }
    pub fn deserialize_from_net(bytes: Vec<u8>) -> Slip {
        let tx_id: SaitoPublicKey = bytes[..33].try_into().unwrap();
        let uuid: SaitoSignature = bytes[33..97].try_into().unwrap();
        let amount: u64 = u64::from_be_bytes(bytes[97..105].try_into().unwrap());
        let slip_ordinal: u8 = bytes[105];
        let slip_type: SlipType = SlipType::try_from(bytes[SLIP_SIZE - 1]).unwrap();
        //u64::from_be_bytes(bytes[32..40].try_into().unwrap());
        Slip::new(SlipCore::new(tx_id, uuid, amount, slip_ordinal, slip_type))
    }
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.publickey);
        vbytes.extend(&self.core.uuid);
        vbytes.extend(&self.core.amount.to_be_bytes());
        vbytes.extend(&self.core.slip_ordinal.to_be_bytes());
        vbytes.extend(&(self.core.slip_type as u8).to_be_bytes());
        vbytes
    }
}

impl Default for Slip {
    fn default() -> Self {
        Self::new(SlipCore::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slip_core_default_test() {
        let slip_core = SlipCore::default();
        assert_eq!(slip_core.publickey, [0; 33]);
        assert_eq!(slip_core.uuid, [0; 64]);
        assert_eq!(slip_core.amount, 0);
        assert_eq!(slip_core.slip_type, SlipType::Normal);
    }

    #[test]
    fn slip_core_new_test() {
        let slip_core = SlipCore::new([0; 33], [0; 64], 0, 0, SlipType::Normal);
        assert_eq!(slip_core.publickey, [0; 33]);
        assert_eq!(slip_core.uuid, [0; 64]);
        assert_eq!(slip_core.amount, 0);
        assert_eq!(slip_core.slip_type, SlipType::Normal);
    }

    #[test]
    fn slip_default_test() {
        let slip = Slip::default();
        assert_eq!(slip.core.publickey, [0; 33]);
        assert_eq!(slip.core.uuid, [0; 64]);
        assert_eq!(slip.core.amount, 0);
        assert_eq!(slip.core.slip_type, SlipType::Normal);
    }

    #[test]
    fn slip_new_test() {
        let slip = Slip::new(SlipCore::default());
        assert_eq!(slip.core.publickey, [0; 33]);
        assert_eq!(slip.core.uuid, [0; 64]);
        assert_eq!(slip.core.amount, 0);
        assert_eq!(slip.core.slip_type, SlipType::Normal);
    }

    #[test]
    fn slip_serialization_for_net_test() {
        let slip = Slip::new(SlipCore::default());
        let serialized_slip = slip.serialize_for_net();
        assert_eq!(serialized_slip.len(), 107);
        let deserilialized_slip = Slip::deserialize_from_net(serialized_slip);
        assert_eq!(slip, deserilialized_slip);
    }
}
