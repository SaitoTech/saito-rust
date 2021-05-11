use crate::slip::{Slip};
use secp256k1::{PublicKey, Signature};

use std::time::{SystemTime, UNIX_EPOCH};

pub fn create_timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    return since_the_epoch.as_millis() as u64;
}

// TODO -- documentation
#[derive(Debug, PartialEq, Clone)]
pub struct Hop {
    pub from: PublicKey,
    pub to: PublicKey,
    sig: Signature,
}

impl Hop {
    pub fn new(to: PublicKey, from: PublicKey, sig: Signature) -> Hop {
        return Hop { to, from, sig }
    }
}
#[derive(Debug, PartialEq)]
pub struct Transaction {
    body: TransactionBody
}

#[derive(Debug, PartialEq)]
pub struct TransactionBody {
    id:       u64,
    ts:       u64,
    pub to:   Vec<Slip>,
    pub from: Vec<Slip>,
    sig:      Signature,
    pub typ:  TransactionBroadcastType,
    path:     Vec<Hop>,
    pub msg:  Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TransactionBroadcastType {
    Normal,
    GoldenTicket,
    Fee,
    Rebroadcast,
    VIP,
    GoldenChunk,
}

impl Transaction {
    pub fn new(typ: TransactionBroadcastType) -> Transaction {
        return Transaction {
            body: TransactionBody {
                id:   0,
                ts:   create_timestamp(),
                to:   vec![],
                from: vec![],
                sig:  Signature::from_compact(&[0; 64]).unwrap(),
                typ,
                path: vec![],
                msg:  vec![],
            },
        };
    }

    pub fn get_id(&self) -> u64 {
        return self.body.id
    }

    pub fn get_timestamp(&self) -> u64 {
        return self.body.ts;
    }

    pub fn get_to_slips(&self) -> Vec<Slip> {
        return self.body.to.clone();
    }

    pub fn get_from_slips(&self) -> Vec<Slip> {
        return self.body.from.clone();
    }

    pub fn get_signature(&self) -> Signature {
        return self.body.sig;
    }

    pub fn get_type(&self) -> TransactionBroadcastType {
        return self.body.typ.clone();
    }

    pub fn get_path(&self) -> Vec<Hop> {
        return self.body.path.clone();
    }

    pub fn get_message(&self) -> Vec<u8> {
        return self.body.msg.clone();
    }

    pub fn set_id(&mut self, id: u64) {
        self.body.id = id;
    }

    pub fn set_to_slips(&mut self, slips: Vec<Slip>) {
        self.body.to = slips;
    }

    pub fn add_to_slip(&mut self, slip: Slip) {
        self.body.to.push(slip);
    }

    pub fn set_from_slips(&mut self, slips: Vec<Slip>) {
        self.body.from = slips;
    }

    pub fn add_from_slip(&mut self, slip: Slip) {
        self.body.from.push(slip);
    }

    pub fn set_signature(&mut self, sig: Signature) {
        self.body.sig = sig;
    }

    pub fn set_path(&mut self, path: Vec<Hop>) {
        self.body.path = path;
    }

    pub fn add_hop_to_path(&mut self, path: Hop) {
        self.body.path.push(path);
    }

    pub fn set_message(&mut self, msg: Vec<u8>) {
        self.body.msg = msg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        slip::{Slip, SlipBroadcastType},
        keypair::{Keypair}
    };

    #[test]
    fn transaction_test() {
        let mut tx = Transaction::new(TransactionBroadcastType::Normal);

        assert_eq!(tx.get_id(), 0);
        assert_eq!(tx.get_to_slips(), vec![]);
        assert_eq!(tx.get_from_slips(), vec![]);
        assert_eq!(tx.get_signature(), Signature::from_compact(&[0; 64]).unwrap());
        assert_eq!(tx.get_type(), TransactionBroadcastType::Normal);
        assert_eq!(tx.get_path(), vec![]);
        assert_eq!(tx.get_message(), vec![]);

        let keypair = Keypair::new().unwrap();
        let to_slip = Slip::new(
            keypair.get_public_key(),
            SlipBroadcastType::Normal,
            0
        );
        let from_slip = Slip::new(
            keypair.get_public_key(),
            SlipBroadcastType::Normal,
            0
        );

        tx.add_to_slip(to_slip);
        tx.add_from_slip(from_slip);

        assert_eq!(tx.get_to_slips(), vec![to_slip.clone()]);
        assert_eq!(tx.get_from_slips(), vec![from_slip.clone()]);

        // TODO -- test sig, path, and msg
    }
}
