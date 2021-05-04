use serde::{Serialize, Deserialize};
use crate::hop::{Hop};
use crate::slip::{Slip};
use crate::helper::{create_timestamp};
use crate::crypto::{Signature, PublicKey};

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum TransactionBroadcastType {
  Normal,
  GoldenTicket,
  Fee,
  Rebroadcast,
  VIP,
  GoldenChunk,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct TransactionBody {
    id:   u32,
    ts:   u64,
    pub   to: Vec<Slip>,
    pub   from: Vec<Slip>,
    sig:  Signature,
    ver:  f32,
    pub typ:  TransactionBroadcastType,
    path: Vec<Hop>,
    pub msg:  Vec<u8>,
    ps:   u8,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Transaction {
    pub body: TransactionBody,
    is_valid: u8,
//    msg_hash: [u8; 32],
//    size: u64,
//    fees_total: u64,
//    fees_usable_for_block_producer: u64,
    fees_cumulative: u64,
//    decrypted_msg: Vec<u8>,

}
impl Transaction {
    pub fn new() -> Transaction {
        return Transaction {
            body: TransactionBody {
                id:   0,
                ts:   create_timestamp(),
                to:   vec![],
                from: vec![],
                sig:  Signature::from_compact(&[0; 64]).unwrap(),
                ver:  0.1,
                typ:  TransactionBroadcastType::Normal,
                path: vec![],
                msg:  vec![],
                ps:   0
            },
            is_valid: 0,
            fees_cumulative: 0
        };
    }
    // new(type TxTypeEnum, sig String, slips [Slip])
    // deserialize(Stream or String)
    // serialize() -> Stream or String
    // validate()
    // getType()
    // addSlip(Slip)
    // getSlips -> [Slip]
    // pushToPath(address String)
    // createRebroadcastTxFrom() -> Transaction
    // Notable missing things: from, to, timestamp
}


impl Clone for TransactionBody {
    fn clone(&self) -> TransactionBody {
        TransactionBody {
            id:   self.id,
            ts:   self.ts,
            to:   self.to.clone(),
            from: self.from.clone(),
            sig:  self.sig,
            ver:  self.ver,
            typ:  self.typ,
            path: self.path.clone(),
            msg:  self.msg.clone(),
            ps:   self.ps
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_new() {
        assert!(false);
    }
}

