use serde::{Serialize, Deserialize};
use crate::crypto::PublicKey;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Slip {
    body: SlipBody,
    lc: u8,
    is_valid: u8,
    pub spent_status: SlipSpentStatus,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct SlipBody {
    add: PublicKey,
    typ: SlipBroadcastType,
    amt: u64,
    bid: u32,
    tid: u32,
    sid: u32,
    bsh: [u8; 32],
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum SlipBroadcastType {
  Normal,
  GoldenTicket,
  Fee,
  Rebroadcast,
  VIP,
  GoldenChunk,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum SlipSpentStatus {
  Unspent,
  Spent,
  Pending,
}

impl Slip {
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

    pub fn return_amt(&self) -> u64 {
        return self.body.amt;
    }

    pub fn set_ids(&mut self, bid: u32, tid: u32, sid: u32) {
        self.body.bid = bid;
        self.body.tid = tid;
        self.body.sid = sid;
    }

    pub fn set_amt(&mut self, amt: u64) {
        self.body.amt = amt;
    }

    pub fn set_broadcast_type(&mut self, broadcast_type: SlipBroadcastType) {
        self.body.typ = broadcast_type;
    }

    pub fn set_spent_status(&mut self, spent_status: SlipSpentStatus) {
        self.spent_status = spent_status;
    }

    pub fn return_add(&self) -> PublicKey {
        return self.body.add;
    }

    // implication is slip type is immutable
    pub fn return_signature_source(&self) -> Vec<u8> {
       return bincode::serialize(&self.body).unwrap();
    }

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

