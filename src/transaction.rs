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

    pub fn add_to_slip(&mut self, slip: Slip) {
        self.body.to.push(slip);
    }

    pub fn add_from_slip(&mut self, slip: Slip) {
        self.body.from.push(slip)
    }

    pub fn get_msg(self) -> Vec<u8> {
        return self.body.msg;
    }

    pub fn set_msg(&mut self, msg: Vec<u8>) {
        self.body.msg = msg;
    }

    pub fn get_to_slips(&self) -> Vec<Slip> {
        return self.body.to.clone();
    }

    pub fn get_from_slips(&self) -> Vec<Slip> {
        return self.body.from.clone();
    }

    pub fn set_to_slips(&mut self, slips: Vec<Slip>) {
        self.body.to = slips;
    }

    pub fn set_from_slips(&mut self, slips: Vec<Slip>) {
        self.body.from = slips;
    }

    pub fn get_path(&self) -> Vec<Hop> {
        return self.body.path.clone();
    }

    pub fn get_fees_usable(&self, publickey: &PublicKey) -> u64 {
        //
        // we want to cache this value and reuse it in the future;
        //
        let input_fees: u64 = self.body.from
            .iter()
            .filter(|slip| &slip.return_add() == publickey)
            .map(|slip| slip.return_amt())
            .sum();

        let output_fees: u64 = self.body.to
            .iter()
            .filter(|slip| &slip.return_add() == publickey)
            .map(|slip| slip.return_amt())
            .sum();

        // need stronger checks here for validation
        // this should not be possible to do unless your
        // receiving the block reward

        if input_fees < output_fees {
            return 0;
        }

        return input_fees - output_fees;
    }

    pub fn set_id(&mut self, id: u32) {
        self.body.id = id;
    }

    pub fn get_tx_type(&self) -> TransactionBroadcastType {
        return self.body.typ;
    }

    pub fn set_tx_type(&mut self, tx_type: TransactionBroadcastType) {
        self.body.typ = tx_type;
    }

    //
    // TODO - calculate based on path information not 1
    //
    pub fn get_work_available(&self, _publickey: &str) -> u64 {
        return 100_000;
    }

    pub fn get_signature_source(&self) -> Vec<u8> {
        return bincode::serialize(&self.body).unwrap();
    }

    pub fn set_sig(&mut self, sig: Signature) {
        self.body.sig = sig
    }

    pub fn get_fees_total(&self) -> u64 {
        //
        // we want to cache this value and reuse it in the future;
        //
        let input_fees: u64 = self.body.from
            .iter()
            .map(|slip| slip.return_amt())
            .sum();

        let output_fees: u64 = self.body.to
            .iter()
            .map(|slip| slip.return_amt())
            .sum();

        // need stronger checks here for validation
        // this should not be possible to do unless your
        // receiving the block reward

        if input_fees < output_fees {
            return 0;
        }

        return input_fees - output_fees;
    }

    pub fn calculate_cumulative_fees(&mut self, last_fees: u64) -> u64 {
        let total_fees = self.get_fees_total();
        let mut cumulative_fees = 0;

        for i in 0..self.body.path.len() {
            cumulative_fees += (total_fees as f32 / (2_u32).pow(i as u32) as f32).round() as u64
        }

        // cache the value so we only need to run this once
        self.fees_cumulative = cumulative_fees;

        return cumulative_fees;
    }

    pub fn get_fees_cumulative(&self) -> u64 {
        return self.fees_cumulative;
    }

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

