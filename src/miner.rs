use crate::{
    block::Block,
    crypto::PublicKey,
    keypair::Keypair,
    slip::Slip,
    transaction::Transaction,
};

#[derive(Debug, Clone, Copy)]
pub struct Miner {
    // `secp256k1::PublicKey` of the node that found the solution
    publickey: PublicKey,
}

#[derive(Debug, Clone, Copy)]
pub struct GoldenTicket {
    // the target `Block` hash to solve for
    target: [u8; 32],
    // the random solution that matches the target hash to some arbitrary level of difficulty
    solution: [u8; 32],
    // `secp256k1::PublicKey` of the node that found the solution
    publickey: PublicKey,
}

impl Miner {
    /// Create new `GoldenTicket`
    pub fn new(publickey: PublicKey) -> Self {
        Miner {
            publickey,
        }
    }
}

impl GoldenTicket {
    /// Create new `GoldenTicket`
    pub fn new(target: [u8; 32], solution: [u8; 32], publickey: PublicKey) -> Self {
        GoldenTicket {
            target,
            solution,
            publickey,
        }
    }
}


/// Create a new `GoldenTicket` `Transaction`
///
/// * `solution` - `Hash` of solution we created in golden ticket game
/// * `previous_block` - previous `Block` reference
/// * `keypair` - `Keypair`
pub fn generate_golden_ticket(
    solution: [u8; 32],
    previous_block: &Block,
    keypair: &Keypair,
) -> Transaction {

    let mut tx = Transaction::new();
    tx

}



/// Generate random data, used for generating solutions in lottery game
pub fn generate_random_data() -> Vec<u8> {
    return (0..32).map(|_| rand::random::<u8>()).collect();
}

