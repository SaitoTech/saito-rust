use secp256k1::{PublicKey};

/// A UTXO slip containing record of owernship of funds on the network
#[derive(Debug, PartialEq)]
pub struct Slip {
    /// Contains concrete data which is not relative to state of the chain
    body: SlipBody,
    /// Flag determining if the `Slip` is part of the longest chain
    lc: bool,
    /// `SlipSpentStatus` indicating the state of the slip in the network
    pub spent_status: SlipSpentStatus,
}

/// An object that holds concrete data not subjective to state of chain
#[derive(Debug, PartialEq)]
pub struct SlipBody {
    /// An `Sectp256K::PublicKey` determining who owns the `Slip`
    add: PublicKey,
    /// A enum brodcast type determining how to be processed by consensus
    typ: SlipBroadcastType,
    /// Amount of Saito
    amt: u64,
    /// `Block` id
    bid: u64,
    /// `Transaction` id
    tid: u64,
    /// `Slip` id
    sid: u64,
    /// The `Block` hash the slip originated from
    bsh: [u8; 32],
}

/// An enumerated set of `Slip` types
#[derive(Debug, PartialEq, Clone)]
pub enum SlipBroadcastType {
    Normal,
    GoldenTicket,
    Fee,
    Rebroadcast,
    VIP,
    GoldenChunk,
}

/// Current status of the `Slip` state with regards to it's usage in the network
#[derive(Debug, PartialEq, Clone)]
pub enum SlipSpentStatus {
    Unspent,
    Spent,
    Pending,
}

impl Slip {
    /// Create new `Slip` with default type `SlipBroadcastType::Normal`
    ///
    /// * `add` - `Publickey` address to assign ownership
    /// * `typ` - `SlipBroadcastType` of `Slip`
    /// * `amt` - `u64` amount of Saito coantined in the `Slip`
    pub fn new(add: PublicKey, typ: SlipBroadcastType, amt: u64) -> Slip {
        return Slip {
            body: SlipBody {
                add,
                typ,
                amt,
                bid: 0,
                tid: 0,
                sid: 0,
                bsh: [0; 32],
            },
            lc: false,
            spent_status: SlipSpentStatus::Unspent,
        }
    }

    /// Returns address in `Slip`
    pub fn get_address(&self) -> PublicKey {
        return self.body.add;
    }

    /// Returns`Slip` type from the enumerated set of `SlipBroadcastType`
    pub fn get_type(&self) -> SlipBroadcastType {
        return self.body.typ.clone();
    }

    /// Returns amount of Saito in `Slip`
    pub fn get_amt(&self) -> u64 {
        return self.body.amt;
    }

    /// Returns the `Block` id the slip originated from
    pub fn get_bid(&self) -> u64 {
        return self.body.bid;
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn get_tid(&self) -> u64 {
        return self.body.tid;
    }

    /// Returns the `Slip`
    pub fn get_sid(&self) -> u64 {
        return self.body.sid;
    }

    /// Returns the `Block` hash the slip originated from
    pub fn get_block_hash(&self) -> [u8; 32] {
        return self.body.bsh;
    }

    /// Returns if the Slip is part of the longest chain
    pub fn is_longest_chain(&self) -> bool {
        return self.lc
    }

    /// Returns if the Slip is spent
    pub fn is_spent(&self) -> bool {
        self.spent_status == SlipSpentStatus::Spent
    }

    /// Returns if the Slip is unspent
    pub fn is_unspent(&self) -> bool {
        self.spent_status == SlipSpentStatus::Unspent
    }

    /// Returns if the Slip is pending in a block
    pub fn is_pending(&self) -> bool {
        self.spent_status == SlipSpentStatus::Pending
    }

    // Set the `Block` id
    pub fn set_bid(&mut self, bid: u64) {
        self.body.bid = bid;
    }

    // Set the `Transaction` id
    pub fn set_tid(&mut self, tid: u64) {
        self.body.tid = tid;
    }

    // Set the `Slip` id
    pub fn set_sid(&mut self, sid: u64) {
        self.body.sid = sid;
    }

    // Set the `Block` hash
    pub fn set_block_hash(&mut self, bsh: [u8; 32]) {
        self.body.bsh = bsh;
    }

    // Set if `Slip` belongs to longest chain
    pub fn set_longest_chain(&mut self, lc: bool) {
        return self.lc = lc
    }

    // Set the `SlipSpentStatus`
    pub fn set_status(&mut self, status: SlipSpentStatus) {
        return self.spent_status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::{Keypair};
    use rand;

    #[test]
    fn slip_test() {
        let keypair = Keypair::new().unwrap();
        let bsh: [u8; 32] = rand::random();
        let mut slip = Slip::new(
            keypair.get_public_key(),
            SlipBroadcastType::Normal,
            10_000_000
        );

        assert_eq!(slip.get_address(), keypair.get_public_key());
        assert_eq!(slip.get_type(), SlipBroadcastType::Normal);
        assert_eq!(slip.get_amt(), 10_000_000);
        assert_eq!(slip.get_bid(), 0);
        assert_eq!(slip.get_tid(), 0);
        assert_eq!(slip.get_sid(), 0);
        assert_eq!(slip.get_block_hash(), [0; 32]);
        assert!(!slip.is_longest_chain());
        assert!(slip.is_unspent());

        slip.set_bid(10);
        slip.set_tid(10);
        slip.set_sid(10);
        slip.set_block_hash(bsh);
        slip.set_longest_chain(true);
        slip.set_status(SlipSpentStatus::Spent);

        assert_eq!(slip.get_bid(), 10);
        assert_eq!(slip.get_tid(), 10);
        assert_eq!(slip.get_sid(), 10);
        assert_eq!(slip.get_block_hash(), bsh);
        assert!(slip.is_longest_chain());
        assert!(slip.is_spent());
    }
}