use secp256k1::PublicKey;
use std::hash::Hash;


#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Slip {

    publickey: PublicKey,
    transaction_sig: [u8;32], 
    slip_id: u64,
    amount: u64,
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
            transaction_sig:  [0;32],
            amount:  amount,
            slip_id:  0,
            sliptype: SlipType::Normal,
        }
    }

}



#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn slip_test() {
        let Slip = Slip::new();
    }
}


