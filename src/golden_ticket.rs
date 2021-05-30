use crate::{
    block::Block,
    crypto::PublicKey,
    keypair::Keypair,
    slip::Slip,
    transaction::Transaction,
};

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
    let publickey = keypair.publickey();

    // TODO -- create our Golden Ticket
    // until we implement serializers, paysplit and difficulty functionality this doesn't do much
    
    let previous_block_hash = previous_block.clone_hash();
    //let previous_block_hash = previous_block.hash;

    let _gt_solution = GoldenTicket::new(previous_block_hash, solution, publickey.clone());

    let winning_address = find_winner(&solution, previous_block);

    // TODO -- calculate captured fees in blocks based on transaction fees
    // for now, we just split the coinbase between the miners and the routers

    let mut golden_tx = Transaction::new_mock();
    
    let total_fees_for_miners_and_nodes = &previous_block.coinbase();
    
    let miner_share = (*total_fees_for_miners_and_nodes as f64 * 0.5).round() as u64;
    let node_share = total_fees_for_miners_and_nodes - miner_share;
    let miner_slip = Slip::new(*publickey, miner_share);
    let node_slip = Slip::new(winning_address, node_share);
     
    // TODO - DO NOT INTERACT WITH INTERNALS Block.fn or Transaction.fn unless we design explicitly otherwise
    //golden_tx.core.add_output(miner_slip);
    //golden_tx.core.add_output(node_slip);    
    
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
    previous_block.creator()
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
        let publickey = keypair.publickey();
        let random_hash = hash(&generate_random_data());
        let golden_ticket = GoldenTicket::new(random_hash, random_hash, *publickey);

        assert_eq!(golden_ticket.publickey, publickey.clone());
        assert_eq!(golden_ticket.target, random_hash);
        assert_eq!(golden_ticket.solution, random_hash);
    }
}
