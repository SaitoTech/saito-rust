use criterion::{black_box, BatchSize};
use criterion::{criterion_group, criterion_main, Criterion};
use saito_rust::keypair::Keypair;
use saito_rust::test_utilities;

fn bench_add_block(c: &mut Criterion) {
    let keypair = Keypair::new();

    c.bench_function("add block", move |b| {
        let (mut blockchain, slips) = test_utilities::make_mock_blockchain_and_slips(&keypair, 2);
        let mut prev_block_hash = [0; 32];
        let mut prev_block_id = 0;
        b.iter_batched(
            || {
                // let (blockchain, slips) =
                //         test_utilities::make_mock_blockchain_and_slips(&keypair, 1);
                // let prev_block_hash = [0; 32];
                // let prev_block_id = 0;
                let block = saito_rust::test_utilities::make_mock_block(
                    &keypair,
                    prev_block_hash,
                    prev_block_id + 1,
                    slips.first().unwrap().0,
                );
                prev_block_hash = block.hash();
                prev_block_id = block.id();
                block
            },
            |block| blockchain.add_block(black_box(block)).await,
            BatchSize::NumIterations(1),
        )
    });
}

criterion_group!(benches, bench_add_block);
criterion_main!(benches);
