use crate::{
    block::Block,
    crypto::PublicKey,
    keypair::Keypair,
    slip::{OutputSlip, SlipBroadcastType},
    transaction::{Transaction, TransactionType},
};
use secp256k1::Signature;

/// The golden ticket is a data structure containing instructions for picking
/// a winner for paysplit in saito consensus.
#[derive(Debug, Clone, Copy)]
pub struct GoldenTicket {
    // the target `Block` hash to solve for
    target: [u8; 32],
    // the random solution that matches the target hash to some arbitrary level of difficulty
    solution: [u8; 32],
    // `secp256k1::PublicKey` of the node that found the solution
    publickey: PublicKey,
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
pub fn generate_golden_ticket_transaction(
    solution: [u8; 32],
    previous_block: &Block,
    keypair: &Keypair,
) -> Transaction {
    let publickey = keypair.public_key();

    // TODO -- create our Golden Ticket
    // until we implement serializers, paysplit and difficulty functionality this doesn't do much
    let mut previous_block_hash = previous_block.hash();

    if previous_block_hash.is_none() {
      previous_block_hash = Some([0; 32]);
    } 

    let _gt_solution = GoldenTicket::new(previous_block_hash.unwrap(), solution, publickey.clone());

    let winning_address = find_winner(&solution, previous_block);

    // TODO -- calculate captured fees in blocks based on transaction fees
    // for now, we just split the coinbase between the miners and the routers

    let total_fees_for_miners_and_nodes = previous_block.coinbase();

    let miner_share = (total_fees_for_miners_and_nodes as f64 * 0.5).round() as u64;
    let node_share = total_fees_for_miners_and_nodes - miner_share;

    let mut golden_tx_body = Transaction::new(TransactionType::Normal);

    let miner_slip = OutputSlip::new(*publickey, SlipBroadcastType::Normal, miner_share);
    let node_slip = OutputSlip::new(winning_address, SlipBroadcastType::Normal, node_share);

    golden_tx_body.add_output(miner_slip);
    golden_tx_body.add_output(node_slip);

    // toodo -- serialize our golden_ticket solution into our msg field
    // this is used to change the difficulty and paysplit in the upcoming block

    // golden_tx.set_message()

    // todo, sign the transaction and create signature
    // complete once we've added serialization

    // TODO sign transaction with wallet
    let golden_tx =
        Transaction::add_signature(golden_tx_body, Signature::from_compact(&[0; 64]).unwrap());
    golden_tx
}

/// Find the winner of the golden ticket game
///
/// * `solution` - `Hash` of solution we created in golden ticket game
/// * `previous_block` - Previous `Block`
fn find_winner(_solution: &[u8; 32], previous_block: &Block) -> PublicKey {
    // TODO -- use fees paid in the block to determine the block winner with routing algorithm
    // for now, we just use the block creator to determine who the winner is
    // TODO - don't die here
    previous_block.creator().unwrap()
}

/// Generate random data, used for generating solutions in lottery game
pub fn generate_random_data() -> Vec<u8> {
    return (0..32).map(|_| rand::random::<u8>()).collect();
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::crypto::hash;
    use crate::keypair::Keypair;

    #[test]
    fn golden_ticket_test() {
        let keypair = Keypair::new();
        let publickey = keypair.public_key();
        let random_hash = hash(&generate_random_data());
        let golden_ticket = GoldenTicket::new(random_hash, random_hash, *publickey);

        assert_eq!(golden_ticket.publickey, publickey.clone());
        assert_eq!(golden_ticket.target, random_hash);
        assert_eq!(golden_ticket.solution, random_hash);
    }
}
