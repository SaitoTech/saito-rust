use crate::block::{Block, BlockCore};
use crate::crypto::SaitoHash;
use crate::slip::{Slip, SlipCore, SlipType};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionCore, TransactionType};
use crate::wallet::Wallet;

pub struct MockTimestampGenerator {
    timestamp: u64,
}

impl MockTimestampGenerator {
    pub fn new() -> MockTimestampGenerator {
        MockTimestampGenerator {
            timestamp: create_timestamp(),
        }
    }
    pub fn next(&mut self) -> u64 {
        self.timestamp += 1000;
        self.timestamp
    }
}

pub fn make_mock_block(previous_block_hash: SaitoHash, block_id: u64) -> Block {
    // let mock_input =     pub fn new(
    //     publickey: [u8; 33],
    //     uuid: [u8; 64],
    //     amount: u64,
    //     slip_ordinal: u8,
    //     slip_type: SlipType,
    // ) -> Self {

    let wallet = Wallet::new();
    let mock_input = Slip::new(SlipCore::new(
        wallet.get_publickey(),
        [0; 64],
        1,
        0,
        SlipType::Normal,
    ));
    let mock_output = Slip::new(SlipCore::new(
        wallet.get_publickey(),
        [0; 64],
        1,
        0,
        SlipType::Normal,
    ));
    let mut transaction = Transaction::new(TransactionCore::new(
        create_timestamp(),
        vec![mock_input],
        vec![mock_output],
        vec![],
        TransactionType::Normal,
        [0; 64],
    ));
    transaction.sign(wallet.get_privatekey());
    let mock_core = BlockCore::new(
        block_id,
        create_timestamp(),
        previous_block_hash,
        wallet.get_publickey(),
        [2; 32],
        [3; 64],
        0,
        0,
        0,
    );
    let mut block = Block::new(mock_core);
    block.set_transactions(&mut vec![transaction]);
    block.set_merkle_root(block.generate_merkle_root());
    block.set_hash(block.generate_hash());
    block
}
pub fn make_mock_invalid_block(previous_block_hash: SaitoHash, block_id: u64) -> Block {
    let mut mock_block = make_mock_block(previous_block_hash, block_id);
    mock_block.set_merkle_root([0; 32]);
    mock_block
}

// pub fn make_mock_block_with_tx(
//     previous_block_hash: SaitoHash,
//     block_id: u64,
//     tx: Transaction,
// ) -> Block {
//     Block::new_mock(previous_block_hash, &mut vec![tx.clone()], block_id)
// }
// pub fn make_mock_tx(input: SlipID, amount: u64, to: PublicKey) -> Transaction {
//     let to_slip = OutputSlip::new(to, SlipType::Normal, amount);
//     Transaction::new(
//         Signature::from_compact(&[0; 64]).unwrap(),
//         vec![],
//         create_timestamp(),
//         vec![input.clone()],
//         vec![to_slip.clone()],
//         TransactionType::Normal,
//         vec![104, 101, 108, 108, 111],
//     )
// }
