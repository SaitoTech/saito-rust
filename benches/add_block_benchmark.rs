use criterion::{criterion_group, criterion_main, Criterion};
use saito_rust::{
    keypair::Keypair,
    slip::{OutputSlip, SlipType},
    transaction::{Transaction, TransactionCore},
};

// fn bench_add_block(c: &mut Criterion) {
//     let keypair = Keypair::new();
//     c.bench_function("add block", move |b| {
//         let (mut blockchain, slips) = test_utilities::make_mock_blockchain_and_slips(&keypair, 2);
//         let mut prev_block_hash = [0; 32];
//         let mut prev_block_id = 0;
//         b.iter_batched(
//             || {
//                 // let (blockchain, slips) =
//                 //         test_utilities::make_mock_blockchain_and_slips(&keypair, 1);
//                 // let prev_block_hash = [0; 32];
//                 // let prev_block_id = 0;
//                 let block = saito_rust::test_utilities::make_mock_block(
//                     &keypair,
//                     prev_block_hash,
//                     prev_block_id + 1,
//                     slips.first().unwrap().0,
//                 );
//                 prev_block_hash = block.hash();
//                 prev_block_id = block.id();
//                 block
//             },
//             |block| blockchain.add_block(black_box(block)),
//             BatchSize::NumIterations(1),
//         )
//     });
// }

fn create_output(keypair: &Keypair) -> OutputSlip {
    OutputSlip::new(keypair.public_key().clone(), SlipType::Normal, 10000)
}

fn create_transaction_core() -> TransactionCore {
    TransactionCore::default()
}

fn create_tx_signature(core: TransactionCore, keypair: &Keypair) -> Transaction {
    Transaction::create_signature(core, keypair)
}

fn bench_create_output(c: &mut Criterion) {
    let keypair = Keypair::new();
    c.bench_function("create outputs", |b| b.iter(|| create_output(&keypair)));
}

fn bench_tx_core(c: &mut Criterion) {
    c.bench_function("create transaction core", |b| {
        b.iter(|| create_transaction_core())
    });
}

fn bench_tx_sig(c: &mut Criterion) {
    let keypair = Keypair::new();
    c.bench_function("create tx signature", |b| {
        b.iter(|| create_tx_signature(create_transaction_core(), &keypair))
    });
}

criterion_group!(benches, bench_create_output, bench_tx_core, bench_tx_sig);
criterion_main!(benches);
