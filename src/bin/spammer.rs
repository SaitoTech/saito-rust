use saito_rust::{block::Block, blockchain::AddBlockEvent, keypair::Keypair, test_utilities, time::create_timestamp, transaction::{Transaction, TransactionType}};
use secp256k1::Signature;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let keypair = Keypair::new();

	println!("CREATE BLOCKCHAIN AND SLIPS");
	let (mut blockchain, mut slips) =
        test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * 1000000);
	let prev_block = blockchain.latest_block().unwrap();
	//println!("PREVIOUS BLOCK: {:?}", prev_block);
    let block = test_utilities::make_mock_block(&keypair, prev_block.hash(), prev_block.id() + 1, slips.pop().unwrap().0);

    let mut prev_block_hash = block.hash().clone();
    let mut prev_block_id = block.id();

    let mut add_block_timestamps = vec![];
    let mut start_ts;
    let mut finish_ts;

    let result = blockchain.add_block(block.clone());
	println!("{:?}", result);

    for _ in 0..100 {
        let mut txs = vec![];
        println!("make txs {}", create_timestamp());
        for _ in 0..10000 {
            txs.push(Transaction::new(
                Signature::from_compact(&[0; 64]).unwrap(),
                vec![],
                create_timestamp(),
                vec![slips.pop().unwrap().0],
                vec![],
                TransactionType::Normal,
                vec![],
            ));
        }
        println!("make blk {}", create_timestamp());
        let block = Block::new_mock(prev_block_hash, &mut txs, prev_block_id + 1);
        println!("make don {}", create_timestamp());
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();

		println!("CREATING BLOCK {:?}", block.id());

        start_ts = create_timestamp();
        blockchain.add_block(block);
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
