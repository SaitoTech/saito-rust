use serde::{Serialize, Deserialize};
use crate::crypto::PublicKey;


/// Slips contain records of what publickeys own what Saito
/// in the network
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Slip {
    /// `SlipBody` containing record data
    body: SlipBody,
    /// Flag determining if the `Slip` is part of the longest chain
    lc: u8,
    /// Flag indicating cached validation of `Slip`
    is_valid: u8,
    /// `SlipSpentStatus` indicating the state of the slip in the network
    pub spent_status: SlipSpentStatus,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct SlipBody {
    /// Publickey determining who owns the `Slip`
    add: PublicKey,
    /// `Slip` brodcast type determining how it's handled by consensus
    typ: SlipBroadcastType,
    /// Amount of Saito in the `Slip`
    amt: u64,
    /// `Slip` `Block` id
    bid: u32,
    /// `Slip` `Transaction` id
    tid: u32,
    /// `Slip` id
    sid: u32,
    /// /// `Slip` `Block` hash
    bsh: [u8; 32],
}

/// `SlipBroadcastType`
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum SlipBroadcastType {
    Normal,
    GoldenTicket,
    Fee,
    Rebroadcast,
    VIP,
    GoldenChunk,
}
/// Current status of the `Slip` state with regards to it's usage in the
/// network
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum SlipSpentStatus {
    Unspent,
    Spent,
    Pending,
}

impl Slip {
    /// Create new `Slip`
    ///
    /// * `publickey` - `Publickey` address to assign ownership
    pub fn new(publickey: PublicKey) -> Slip {
        return Slip {
            body: SlipBody {
                add: publickey,
                typ: SlipBroadcastType::Normal,
                amt: 0,
                bid: 0,
                tid: 0,
                sid: 0,
                bsh: [0; 32],
            },
            lc: 0,
            is_valid: 0,
            spent_status: SlipSpentStatus::Unspent,
        }
    }

    /// Returns amount in `Slip`
    pub fn return_amt(&self) -> u64 {
        return self.body.amt;
    }

    /// Set ids of `Slip`
    ///
    /// * `bid` - `Block` id
    /// * `tid` - `Transaction` id
    /// * `sid` - `Slip` id
    pub fn set_ids(&mut self, bid: u32, tid: u32, sid: u32) {
        self.body.bid = bid;
        self.body.tid = tid;
        self.body.sid = sid;
    }

    /// Set amount of `Slip`
    pub fn set_amt(&mut self, amt: u64) {
        self.body.amt = amt;
    }

    /// Set `SlipBroadcastType` of `Slip`
    pub fn set_broadcast_type(&mut self, broadcast_type: SlipBroadcastType) {
        self.body.typ = broadcast_type;
    }

    /// Set `SlipSpentStatus` of `Slip`
    pub fn set_spent_status(&mut self, spent_status: SlipSpentStatus) {
        self.spent_status = spent_status;
    }

    /// Returns `Publickey` address of `Slip`
    pub fn return_add(&self) -> PublicKey {
        return self.body.add;
    }

    /// Returns serialized byte array of `Slip`
    pub fn return_signature_source(&self) -> Vec<u8> {
        // implication is slip type is immutable
       return bincode::serialize(&self.body).unwrap();
    }

    /// Set `Block` hash of `Slip`
    pub fn set_bsh(&mut self, bsh: [u8; 32]) {
        self.body.bsh = bsh;
    }
}


#[cfg(test)]
mod test {
    #[test]
    fn test_new() {
        assert!(false);
    }
    #[test]
    fn test_deserialize() {
        assert!(false);
    }
    #[test]
    fn test_serialize() {
        assert!(false);
    }
    #[test]
    fn test_validate() {
        assert!(false);
    }
}

