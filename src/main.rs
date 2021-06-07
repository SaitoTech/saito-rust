// use saito_rust::consensus;
// use tokio::signal;
use saito_rust::blockchain::{AddBlockEvent, Blockchain};
use saito_rust::test_utilities;
use std::{thread, time};

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    //consensus::run(signal::ctrl_c()).await
    let mut blockchain = Blockchain::new();
    let block = test_utilities::make_mock_block([0; 32], 0);
    let mut prev_block_hash = block.hash().clone();
    let mut prev_block_id = block.id();
    let result: AddBlockEvent = blockchain.add_block(block.clone());
    //println!("{:?}", result);
    assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
    loop {
        thread::sleep(time::Duration::from_millis(1000));

        let block = test_utilities::make_mock_block(prev_block_hash, prev_block_id + 1);
        prev_block_hash = block.hash().clone();
        prev_block_id = block.id();
        let result: AddBlockEvent = blockchain.add_block(block.clone());
        println!("{:?}", result);
    }
    //Ok(())
}
