use crate::{
    blockchain::UtxoSet,
    crypto::{SaitoHash, SaitoPublicKey, SaitoUTXOSetKey},
};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

use enum_variant_count_derive::TryFromByte;
use std::convert::{TryFrom, TryInto};

/// The size of a serilized slip in bytes.
pub const SLIP_SIZE: usize = 75;

/// SlipType is a human-readable indicator of the slip-type, such
/// as in a normal transaction, a VIP-transaction, a rebroadcast
/// transaction or a golden ticket, etc.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq, TryFromByte)]
pub enum SlipType {
    Normal,
    ATR,
    VipInput,
    VipOutput,
    MinerInput,
    MinerOutput,
    RouterInput,
    RouterOutput,
    StakerInput,
    StakerOutput,
    Other, // need more than one value for TryFromBytes
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Slip {
    #[serde_as(as = "[_; 33]")]
    publickey: SaitoPublicKey,
    uuid: SaitoHash,
    amount: u64,
    slip_ordinal: u8,
    slip_type: SlipType,
    #[serde_as(as = "[_; 74]")]
    utxoset_key: SaitoUTXOSetKey,
    is_utxoset_key_set: bool,
}

impl Slip {
    #[allow(clippy::clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            publickey: [0; 33],
            uuid: [0; 32],
            amount: 0,
            slip_ordinal: 0,
            slip_type: SlipType::Normal,
            utxoset_key: [0; 74],
            is_utxoset_key_set: false,
        }
    }

    pub fn validate(&self, utxoset: &UtxoSet) -> bool {
        if self.get_amount() > 0 {
            match utxoset.get(&self.utxoset_key) {
                Some(value) => *value == 1,
                None => false,
            }
        } else {
            true
        }
    }

    pub fn on_chain_reorganization(
        &self,
        utxoset: &mut AHashMap<SaitoUTXOSetKey, u64>,
        _lc: bool,
        slip_value: u64,
    ) {
        if self.get_amount() > 0 {
            utxoset.entry(self.utxoset_key).or_insert(slip_value);
        }
    }

    //
    // Getters and Setters
    //
    pub fn get_publickey(&self) -> SaitoPublicKey {
        self.publickey
    }

    pub fn get_amount(&self) -> u64 {
        self.amount
    }

    pub fn get_uuid(&self) -> SaitoHash {
        self.uuid
    }

    pub fn get_slip_ordinal(&self) -> u8 {
        self.slip_ordinal
    }

    pub fn get_slip_type(&self) -> SlipType {
        self.slip_type
    }

    pub fn set_publickey(&mut self, publickey: SaitoPublicKey) {
        self.publickey = publickey;
    }

    pub fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    pub fn set_uuid(&mut self, uuid: SaitoHash) {
        self.uuid = uuid;
    }

    pub fn set_slip_ordinal(&mut self, slip_ordinal: u8) {
        self.slip_ordinal = slip_ordinal;
    }

    pub fn set_slip_type(&mut self, slip_type: SlipType) {
        self.slip_type = slip_type;
    }

    //
    // Serialization
    //
    pub fn serialize_input_for_signature(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.publickey);
        vbytes.extend(&self.uuid);
        vbytes.extend(&self.amount.to_be_bytes());
        vbytes.extend(&(self.slip_ordinal.to_be_bytes()));
        vbytes.extend(&(self.slip_type as u32).to_be_bytes());
        vbytes
    }

    //
    // Serialization
    //
    // output slips are signed as zero'd out byte arrays. we have
    // a separate function to handle them as otherwise we may
    // generate an incorrect signature after we have updated the
    // transaction outputs with the proper UUID for insertion into
    // the utxoset.
    //
    pub fn serialize_output_for_signature(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.publickey);
        vbytes.extend(&[0; 32]);
        vbytes.extend(&self.amount.to_be_bytes());
        vbytes.extend(&(self.slip_ordinal.to_be_bytes()));
        vbytes.extend(&(self.slip_type as u32).to_be_bytes());
        vbytes
    }

    pub fn generate_utxoset_key(&mut self) {
        self.utxoset_key = self.get_utxoset_key();
        self.is_utxoset_key_set = true;
    }

    // 33 bytes publickey
    // 32 bytes uuid
    // 8 bytes amount
    // 1 byte slip_ordinal
    pub fn get_utxoset_key(&self) -> SaitoUTXOSetKey {
        let mut res: Vec<u8> = vec![];
        res.extend(&self.get_publickey());
        res.extend(&self.get_uuid());
        res.extend(&self.get_amount().to_be_bytes());
        res.extend(&self.get_slip_ordinal().to_be_bytes());

        res[0..74].try_into().unwrap()

        //        res
    }

    pub fn deserialize_from_net(bytes: Vec<u8>) -> Slip {
        let publickey: SaitoPublicKey = bytes[..33].try_into().unwrap();
        let uuid: SaitoHash = bytes[33..65].try_into().unwrap();
        let amount: u64 = u64::from_be_bytes(bytes[65..73].try_into().unwrap());
        let slip_ordinal: u8 = bytes[73];
        let slip_type: SlipType = SlipType::try_from(bytes[SLIP_SIZE - 1]).unwrap();
        let mut slip = Slip::new();

        slip.set_publickey(publickey);
        slip.set_uuid(uuid);
        slip.set_amount(amount);
        slip.set_slip_ordinal(slip_ordinal);
        slip.set_slip_type(slip_type);

        slip
    }
    pub fn serialize_for_net(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.publickey);
        vbytes.extend(&self.uuid);
        vbytes.extend(&self.amount.to_be_bytes());
        vbytes.extend(&self.slip_ordinal.to_be_bytes());
        vbytes.extend(&(self.slip_type as u8).to_be_bytes());
        vbytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slip_new_test() {
        let mut slip = Slip::new();
        assert_eq!(slip.get_publickey(), [0; 33]);
        assert_eq!(slip.get_uuid(), [0; 32]);
        assert_eq!(slip.get_amount(), 0);
        assert_eq!(slip.get_slip_type(), SlipType::Normal);
        assert_eq!(slip.get_slip_ordinal(), 0);

        slip.set_publickey([1; 33]);
        assert_eq!(slip.get_publickey(), [1; 33]);

        slip.set_amount(100);
        assert_eq!(slip.get_amount(), 100);

        slip.set_uuid([30; 32]);
        assert_eq!(slip.get_uuid(), [30; 32]);

        slip.set_slip_ordinal(1);
        assert_eq!(slip.get_slip_ordinal(), 1);

        slip.set_slip_type(SlipType::MinerInput);
        assert_eq!(slip.get_slip_type(), SlipType::MinerInput);
    }

    #[test]
    fn slip_serialize_for_signature_test() {
        let slip = Slip::new();
        assert_eq!(slip.serialize_input_for_signature(), vec![0; 78]);
    }

    #[test]
    fn slip_get_utxoset_key_test() {
        let slip = Slip::new();
        assert_eq!(slip.get_utxoset_key(), [0; 74]);
    }

    #[test]
    fn slip_serialization_for_net_test() {
        let slip = Slip::new();
        let serialized_slip = slip.serialize_for_net();
        assert_eq!(serialized_slip.len(), 75);
        let deserilialized_slip = Slip::deserialize_from_net(serialized_slip);
        assert_eq!(slip, deserilialized_slip);
    }
}
