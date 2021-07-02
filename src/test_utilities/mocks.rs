use crate::block::{Block, BlockCore};
use crate::burnfee::BurnFee;
use crate::crypto::SaitoHash;
use crate::slip::{Slip, SlipCore, SlipType};
use crate::transaction::{Transaction, TransactionCore, TransactionType};
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
    let mock_input = Slip::new(SlipCore::new(
        wallet.get_publickey(),
        [0; 32],
        1,
        0,
        SlipType::Normal,
    ));
    let mock_output = Slip::new(SlipCore::new(
        wallet.get_publickey(),
        [0; 32],
        1,
        0,
        SlipType::Normal,
    ));
    let mut transaction = Transaction::new(TransactionCore::new(
        timestamp,
        vec![mock_input],
        vec![mock_output],
        vec![],
        TransactionType::Normal,
        [0; 64],
    ));
    transaction.sign(wallet.get_privatekey());
    let mock_core = BlockCore::new(
        block_id,
        timestamp,
        previous_block_hash,
        wallet.get_publickey(),
        [2; 32],
        [3; 64],
        0,
        burnfee,
        0,
    );
    let mut block = Block::new(mock_core);
    block.set_transactions(&mut vec![transaction]);
    block.set_merkle_root(block.generate_merkle_root());
    block.set_hash(block.generate_hash());
    block
}
