use saito_rust::{
    block::Block, keypair::Keypair, test_utilities,
    time::create_timestamp,
};

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    let keypair = Keypair::new();

	let (mut blockchain, mut slips) =
        test_utilities::make_mock_blockchain_and_slips(&keypair, 3 * 10000);
	let prev_block = blockchain.latest_block().unwrap();
	//println!("PREVIOUS BLOCK: {:?}", prev_block);
    let block = test_utilities::make_mock_block(&keypair, prev_block.hash(), prev_block.id() + 1, slips.pop().unwrap().0);

    let mut prev_block_hash = block.hash().clone();
    let mut prev_block_id = block.id();

    let mut add_block_timestamps = vec![];
    let mut start_ts;
    let mut finish_ts;

    let _result = blockchain.add_block(block.clone());
    // TODO assert something about this result
	
    for _ in 0..10 {
        let mut txs = vec![];

        for _ in 0..10000 {
            txs.push(test_utilities::make_mock_sig_tx(
                &keypair,
                slips.pop().unwrap().0,
                10,
                *keypair.public_key(),
                1024,
            ))
        }

        let block = Block::new_mock(prev_block_hash, &mut txs, prev_block_id + 1);

        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();

		start_ts = create_timestamp();
        blockchain.add_block(block);
        finish_ts = create_timestamp();
        
        add_block_timestamps.push(finish_ts - start_ts);
    }

    let add_block_sum: u64 = add_block_timestamps.iter().sum();
    let add_block_len: u64 = add_block_timestamps.len() as u64;
    let add_block_avg = add_block_sum as f32 / add_block_len as f32;
    println!("AVERAGE ADD BLOCK TIME: {:?}", add_block_avg);

    Ok(())
}
