use serde::{Deserialize, Serialize};

/// SlipType is a human-readable indicator of the slip-type, such
/// as in a normal transaction, a VIP-transaction, a rebroadcast
/// transaction or a golden ticket, etc.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
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
    publickey: [u8; 33],
    #[serde_as(as = "[_; 64]")]
    uuid: [u8; 64],
    amount: u64,
    slip_type: SlipType,
}

impl SlipCore {
    pub fn new(publickey: [u8; 33], uuid: [u8; 64], amount: u64, slip_type: SlipType) -> Self {
        Self {
            publickey,
            uuid,
            amount,
            slip_type,
        }
    }
}

impl Default for SlipCore {
    fn default() -> Self {
        Self::new([0; 33], [0; 64], 0, SlipType::Normal)
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

    pub fn serialize_for_signature(&self) -> Vec<u8> {
        let mut vbytes: Vec<u8> = vec![];
        vbytes.extend(&self.core.publickey);
        vbytes.extend(&self.core.uuid);
        vbytes.extend(&self.core.amount.to_be_bytes());
        vbytes.extend(&(self.core.slip_type as u32).to_be_bytes());

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
        let slip_core = SlipCore::new([0; 33], [0; 64], 0, SlipType::Normal);
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
}
