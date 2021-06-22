use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use saito_rust::test_utilities::memory_stats::MemoryStats;
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

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let keypair = Keypair::new();

    let (mut blockchain, mut slips) =
        test_utilities::mocks::make_mock_blockchain_and_slips(&keypair, 3 * 100000).await;
    let prev_block = blockchain.latest_block().unwrap();

    let mut prev_block_hash = prev_block.hash().clone();
    let mut prev_block_id = prev_block.id();
    let mut prev_burn_fee = prev_block.start_burnfee();
    let mut prev_timestamp = prev_block.timestamp();

    let mut add_block_timestamps = vec![];
    let mut start_ts;
    let mut finish_ts;

    for _ in 0..100 as i32 {
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
                        (0..102400)
                            .into_par_iter()
                            .map(|_| rand::random::<u8>())
                            .collect(),
                    ),
                    &keypair,
                )
            })
            .collect();

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
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        prev_burn_fee = block.start_burnfee();
        prev_timestamp = block.timestamp();

        start_ts = create_timestamp();
        println!("ADD BLOCK {}", block.id());
        println!("Memory Stats {}", MemoryStats::current());

        let result = blockchain.add_block(block).await;
        assert!(result == AddBlockEvent::AcceptedAsLongestChain);
        finish_ts = create_timestamp();
        add_block_timestamps.push(finish_ts - start_ts);
    }

    let add_block_sum: u64 = add_block_timestamps.iter().sum();
    let add_block_len: u64 = add_block_timestamps.len() as u64;
    let add_block_avg = add_block_sum as f32 / add_block_len as f32;
    println!("AVERAGE ADD BLOCK TIME: {:?}", add_block_avg);

    Ok(())
}
