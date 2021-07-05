use crate::block::Block;
use crate::burnfee::BurnFee;
use crate::crypto::SaitoHash;
use crate::slip::{Slip, SlipType};
use crate::transaction::Transaction;
use crate::wallet::Wallet;

pub fn make_mock_block(
    prev_timestamp: u64,
    previous_burnfee: u64,
    previous_block_hash: SaitoHash,
    block_id: u64,
) -> Block {
    let step: u64 = 10000;
    let timestamp = prev_timestamp + step;
    let burnfee = BurnFee::return_burnfee_for_block_produced_at_current_timestamp_in_nolan(
        previous_burnfee,
        timestamp,
        prev_timestamp,
    );
    let wallet = Wallet::new();
    let mut mock_input = Slip::new();
    mock_input.set_publickey(wallet.get_publickey());
    mock_input.set_uuid([0; 32]);
    mock_input.set_amount(1);
    mock_input.set_slip_ordinal(0);
    mock_input.set_slip_type(SlipType::Normal);

    let mut mock_output = Slip::new();
    mock_output.set_publickey(wallet.get_publickey());
    mock_output.set_uuid([0; 32]);
    mock_output.set_amount(1);
    mock_output.set_slip_ordinal(0);
    mock_output.set_slip_type(SlipType::Normal);

    let mut transaction = Transaction::new();

    transaction.add_input(mock_input);
    transaction.add_output(mock_output);

    transaction.sign(wallet.get_privatekey());

    let mut block = Block::new();

    block.set_id(block_id);
    block.set_timestamp(timestamp);
    block.set_previous_block_hash(previous_block_hash);
    block.set_creator(wallet.get_publickey());
    block.set_merkle_root([2; 32]);
    block.set_signature([3; 64]);
    block.set_difficulty(0);
    block.set_treasury(0);
    block.set_burnfee(burnfee);

    block.set_transactions(&mut vec![transaction]);
    block.set_merkle_root(block.generate_merkle_root());
    block.set_hash(block.generate_hash());

    block
}
