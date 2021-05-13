use criterion::{black_box, criterion_group, criterion_main, Criterion};
use saito_rust::{
    keypair::Keypair,
    slip::{Slip, SlipBroadcastType}
};

fn criterion_benchmark(c: &mut Criterion) {
    let keypair = Keypair::new();
    let slip = Slip::new(
        keypair.public_key().clone(),
        SlipBroadcastType::Normal,
        10_000_000,
    );

    c.bench_function("slip serilaization", |b| b.iter(|| {
        let xs: Vec<u8> = bincode::serialize(&slip).unwrap();
        // let xd: i32 = bincode::deserialize(&xs).unwrap();
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);