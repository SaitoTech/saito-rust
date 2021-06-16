use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use saito_rust::{
    block::{Block, BlockCore, TREASURY},
    blockchain::AddBlockEvent,
    burnfee::BurnFee,
    keypair::Keypair,
    slip::{OutputSlip, SlipID, SlipType},
    test_utilities,
    time::create_timestamp,
    transaction::{Transaction, TransactionCore, TransactionType},
};

use std::sync::{Arc, Mutex};

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let keypair = Keypair::new();

    println!("CREATE BLOCKCHAIN AND SLIPS");
    let (mut blockchain, mut slips) =
        test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * 100000);
    let prev_block = blockchain.latest_block().unwrap();

    // let arc_slips = Arc::new(Mutex::new(slips));

    //println!("PREVIOUS BLOCK: {:?}", prev_block);
    //let block = test_utilities::make_mock_block(&keypair, prev_block.hash(), prev_block.id() + 1, slips.pop().unwrap().0);

    let mut prev_block_hash = prev_block.hash().clone();
    let mut prev_block_id = prev_block.id();
    let mut prev_burn_fee = prev_block.start_burnfee();
    let mut prev_timestamp = prev_block.timestamp();

    let mut add_block_timestamps = vec![];
    let mut start_ts;
    let mut finish_ts;

    // let result = blockchain.add_block(block.clone());
    // println!("{:?}", result);

    for _ in 0..100 {
        println!("make txs {}", create_timestamp());

        let pairs: Vec<(SlipID, OutputSlip)> = (0..1000)
            .into_iter()
            .map(|_| slips.pop().unwrap())
            .collect();

        let mut txs = pairs
            .into_par_iter()
            .map(|slip_pair| {
                let to_slip = OutputSlip::new(
                    *keypair.public_key(),
                    SlipType::Normal,
                    slip_pair.1.amount(),
                );

                Transaction::create_signature(
                    TransactionCore::new(
                        create_timestamp(),
                        vec![slip_pair.0],
                        vec![to_slip],
                        TransactionType::Normal,
                        (0..1024)
                            .into_par_iter()
                            .map(|_| rand::random::<u8>())
                            .collect(),
                    ),
                    &keypair,
                )
            })
            .collect();

        println!("make blk {}", create_timestamp());
        let timestamp = create_timestamp();
        let block = Block::new(BlockCore::new(
            prev_block_id + 1,
            timestamp,
            prev_block_hash,
            *keypair.public_key(),
            0,
            TREASURY,
            BurnFee::burn_fee_adjustment_calculation(prev_burn_fee, timestamp, prev_timestamp),
            0.0,
            &mut txs,
        ));
        // Block::new_mock(prev_block_hash, &mut txs, prev_block_id + 1);
        println!("make don {}", create_timestamp());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();

        println!("CREATING BLOCK {:?}", block.id());

        start_ts = create_timestamp();
        let result = blockchain.add_block_async(block).await;
        println!("RESULT {:?}", result);
        assert!(result == AddBlockEvent::AcceptedAsLongestChain);
        finish_ts = create_timestamp();
        println!("add block time: {}", finish_ts - start_ts);
        add_block_timestamps.push(finish_ts - start_ts);
    }

    let add_block_sum: u64 = add_block_timestamps.iter().sum();
    let add_block_len: u64 = add_block_timestamps.len() as u64;
    let add_block_avg = add_block_sum as f32 / add_block_len as f32;
    println!("AVERAGE ADD BLOCK TIME: {:?}", add_block_avg);

    Ok(())
}
