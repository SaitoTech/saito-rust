use secp256k1::PublicKey;
use std::hash::Hash;


#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Slip {

    /// address `Sectp256K::PublicKey` controlling slip
    add: PublicKey,
    /// amount of Saito
    amt: u64,
    /// hash of bloch containing slip
    hash: Option<[u8; 32]>,
    /// `Block` id
    bid: u64,
    /// `Transaction` id
    tid: u64,
    /// `Slip` id
    sid: u64,
    /// A enum brodcast type determining how to be processed by consensus
    sliptype: SlipType,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}

impl Slip {

    pub fn new(address: PublicKey) -> Self {
        Slip {
            add:  address,
            amt:  0,
            hash: None,
            bid:  0,
            tid:  0,
            sid:  0,
            sliptype: SlipType::Normal,
        }
    }
    pub fn new(address: PublicKey, amount: u64) -> Self {
        Slip {
            add:  address,
            amt:  amount,
            hash: None,
            bid:  0,
            tid:  0,
            sid:  0,
            sliptype: SlipType::Normal,
        }
    }

    /// Returns address in `Slip`
    pub fn add(&self) -> &PublicKey {
        &self.add
    }

    /// Returns amount of Saito in `Slip`
    pub fn amt(&self) -> u64 {
        self.amt
    }

///
/// TODO - help appreciated here - david
///
    /// Returns the block hash with slip
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.hash
    }

    /// Returns the `Block` id the slip originated from
    pub fn bid(&self) -> u64 {
        self.bid
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn tid(&self) -> u64 {
        self.tid
    }

    /// Returns the `Slip`
    pub fn sid(&self) -> u64 {
        self.sid
    }

    /// Returns`Slip` type from the enumerated set of `SlipType`
    pub fn sliptype(&self) -> SlipType {
        self.sliptype
    }


    /// Set the `Block` id
    pub fn set_bid(&mut self, bid: u64) {
        self.bid = bid;
    }

    /// Set the `Transaction` id
    pub fn set_tid(&mut self, tid: u64) {
        self.tid = tid;
    }

    /// Set the `Slip` id
    pub fn set_sid(&mut self, sid: u64) {
        self.sid = sid;
    }

//
// TODO
//
    /// Set the `Block` hash
///    pub fn set_sid(&mut self, sid: u64) {
///    }

    /// Set the `SlipType` 
///    pub fn set_sid(&mut self, sid: u64) {
///        self.sliptype = sliptype;
///    }

    /// Set the address
///    pub fn set_add(&mut self, sid: u64) {
///        self.add = add;
///    }

}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;

    #[test]
    fn slip_test() {
        let keypair = Keypair::new();
        let mut slip = Slip::new(
            keypair.public_key().clone(),
            10_000_000,
        );

        assert_eq!(slip.add(), keypair.public_key());
        assert_eq!(slip.sliptype(), SlipBroadcastType::Normal);
        assert_eq!(slip.amt(), 10_000_000);
        assert_eq!(slip.bid(), 0);
        assert_eq!(slip.tid(), 0);
        assert_eq!(slip.sid(), 0);

        slip.set_bid(10);
        slip.set_tid(10);
        slip.set_sid(10);

        assert_eq!(slip.bid(), 10);
        assert_eq!(slip.tid(), 10);
        assert_eq!(slip.sid(), 10);
    }
}
