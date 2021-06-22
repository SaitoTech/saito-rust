use secp256k1::{PublicKey, Signature};

use crate::block::{Block, BlockCore, TREASURY};
use crate::blockchain::Blockchain;
use crate::crypto::Sha256Hash;
use crate::keypair::Keypair;
use crate::slip::{OutputSlip, SlipID, SlipType};
use crate::time::create_timestamp;
use crate::transaction::{Transaction, TransactionCore, TransactionType};

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

pub async fn make_mock_blockchain_and_slips(
    keypair: &Keypair,
    slip_count: u64,
) -> (Blockchain, Vec<(SlipID, OutputSlip)>) {
    let mut blockchain = Blockchain::new_mock(String::from("data/test/blocks/"));
    let mut slips = vec![];

    println!("CREATING GOLDEN TRANSACTION");
    let mut golden_tx_core = TransactionCore::default();
    golden_tx_core.set_type(TransactionType::GoldenTicket);

    let coinbase = 50_000_0000_0000;

    println!("CREAITNG SLIPS");
    for _i in 0..slip_count {
        let slip_share = (coinbase as f64 / slip_count as f64).round() as u64;
        let output = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, slip_share);
        golden_tx_core.add_output(output);
    }

    println!("SIGNING TX");
    let golden_tx = Transaction::create_signature(golden_tx_core, keypair);

    let tx_hash = golden_tx.hash();
    golden_tx
        .core
        .outputs()
        .iter()
        .enumerate()
        .for_each(|(idx, output)| slips.push((SlipID::new(tx_hash, idx as u64), output.clone())));

    println!("CREATING BLOCK");
    let block_core = BlockCore::new(
        0,
        create_timestamp(),
        [0; 32],
        keypair.public_key().clone(),
        coinbase,
        TREASURY,
        10.0,
        0.0,
        &mut vec![golden_tx],
    );

    let block = Block::new(block_core);

    blockchain.add_block(block).await;
    // TODO assert something about this result

    (blockchain, slips)
}

pub fn make_mock_block_empty(previous_block_hash: Sha256Hash, block_id: u64) -> Block {
    Block::new_mock(previous_block_hash, &mut vec![], block_id)
}

pub fn make_mock_block(
    keypair: &Keypair,
    previous_block_hash: Sha256Hash,
    block_id: u64,
    from_slip: SlipID,
) -> Block {
    let to_slip = OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 10);
    let tx_core = TransactionCore::new(
        create_timestamp(),
        vec![from_slip.clone()],
        vec![to_slip.clone()],
        TransactionType::Normal,
        vec![104, 101, 108, 108, 111],
    );

    let tx = Transaction::create_signature(tx_core, keypair);

    Block::new_mock(previous_block_hash, &mut vec![tx.clone()], block_id)
}

pub fn make_mock_block_with_tx(
    previous_block_hash: Sha256Hash,
    block_id: u64,
    tx: Transaction,
) -> Block {
    Block::new_mock(previous_block_hash, &mut vec![tx.clone()], block_id)
}
pub fn make_mock_tx(input: SlipID, amount: u64, to: PublicKey) -> Transaction {
    let to_slip = OutputSlip::new(to, SlipType::Normal, amount);
    Transaction::new(
        Signature::from_compact(&[0; 64]).unwrap(),
        vec![],
        create_timestamp(),
        vec![input.clone()],
        vec![to_slip.clone()],
        TransactionType::Normal,
        vec![104, 101, 108, 108, 111],
    )
}

pub fn make_mock_sig_tx(
    keypair: &Keypair,
    input: SlipID,
    amount: u64,
    to: PublicKey,
    msg_bytes: u64,
) -> Transaction {
    // println!("make slip {}", create_timestamp());
    let to_slip = OutputSlip::new(to, SlipType::Normal, amount);
    // println!("make core {}", create_timestamp());
    let tx_core = TransactionCore::new(
        create_timestamp(),
        vec![input],
        vec![to_slip],
        TransactionType::Normal,
        (0..msg_bytes).map(|_| rand::random::<u8>()).collect(),
    );
    // println!("make sign {}", create_timestamp());
    Transaction::create_signature(tx_core, keypair)
}
