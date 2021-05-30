use secp256k1::PublicKey;
use std::hash::Hash;


#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Slip {

    /// address `Sectp256K::PublicKey` controlling slip
    publickey: PublicKey,
    /// amount of Saito
    amount: u64,
    /// `Block` id
    block_id: u64,
    /// `Transaction` id
    transaction_id: u64,
    /// `Slip` id
    slip_id: u64,
    /// A enum brodcast type determining how to be processed by consensus
    sliptype: SlipType,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SlipType {
    Normal,
}

impl Slip {

    pub fn new(publickey: PublicKey, amount: u64) -> Self {
        Slip {
            publickey:  publickey,
            amount:  amount,
            block_id:  0,
            transaction_id:  0,
            slip_id:  0,
            sliptype: SlipType::Normal,
        }
    }

    /// Returns address in `Slip`
    pub fn publickey(&self) -> &PublicKey {
        &self.publickey
    }

    /// Returns amount of Saito in `Slip`
    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Returns the `Block` id the slip originated from
    pub fn block_id(&self) -> u64 {
        self.block_id
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn transaction_id(&self) -> u64 {
        self.transaction_id
    }

    /// Returns the `Slip`
    pub fn slip_id(&self) -> u64 {
        self.slip_id
    }

    /// Returns`Slip` type from the enumerated set of `SlipType`
    pub fn sliptype(&self) -> SlipType {
        self.sliptype
    }


    /// Set the `Block` id
    pub fn set_block_id(&mut self, block_id: u64) {
        self.block_id = block_id;
    }

    /// Set the `Transaction` id
    pub fn set_transaction_id(&mut self, transaction_id: u64) {
        self.transaction_id = transaction_id;
    }

    /// Set the `Slip` id
    pub fn set_slip_id(&mut self, slip_id: u64) {
        self.slip_id = slip_id;
    }

}



#[cfg(test)]
mod tests {

    use super::*;
    use crate::keypair::Keypair;

    #[test]
    fn slip_test() {
        let keypair = Keypair::new();
        let mut slip = Slip::new(
            keypair.publickey().clone(),
            10_000_000,
        );

        assert_eq!(slip.publickey(), keypair.publickey());
        assert_eq!(slip.sliptype(), SlipBroadcastType::Normal);
        assert_eq!(slip.amount(), 10_000_000);
        assert_eq!(slip.block_id(), 0);
        assert_eq!(slip.transaction_id(), 0);
        assert_eq!(slip.slip_id(), 0);

        slip.set_block_id(10);
        slip.set_transaction_id(10);
        slip.set_slip_id(10);

        assert_eq!(slip.block_id(), 10);
        assert_eq!(slip.transaction_id(), 10);
        assert_eq!(slip.slip_id(), 10);
    }
}


