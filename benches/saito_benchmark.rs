use criterion::{criterion_group, criterion_main, Criterion};
use saito_rust::{
    keypair::Keypair,
    slip::{Slip, SlipBroadcastType}
};

fn slip_bincode_serialize(slip: Slip) {
    let xs: Vec<u8> = bincode::serialize(&slip).unwrap();
    let xd: Slip = bincode::deserialize(&xs).unwrap();
}

fn bench_slip_bincode_serialize(c: &mut Criterion) {
    let keypair = Keypair::new();
    let slip = Slip::new(
        keypair.public_key().clone(),
        SlipBroadcastType::Normal,
        10_000_000,
    );

    c.bench_function("slip bincode seriliazation", |b| b.iter(|| slip_bincode_serialize(slip)));
}


fn slip_cbor_serialize(slip: Slip) {
    let bytes = serde_cbor::to_vec(&slip).unwrap();
    let slp: Slip = serde_cbor::from_slice(&bytes).unwrap();
}

fn bench_slip_cbor_serialize(c: &mut Criterion) {
    let keypair = Keypair::new();
    let slip = Slip::new(
        keypair.public_key().clone(),
        SlipBroadcastType::Normal,
        10_000_000,
    );

    c.bench_function("slip cbor seriliazation", |b| b.iter(|| slip_cbor_serialize(slip)));
}

criterion_group!(
    benches,
    bench_slip_bincode_serialize,
    bench_slip_cbor_serialize
);
criterion_main!(benches);