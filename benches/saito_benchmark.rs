use criterion::{criterion_group, criterion_main, Criterion};
use saito_rust::{
    slip::{Slip, SlipBroadcastType}
};
use saito_rust::slip_proto as proto;
use secp256k1::PublicKey;
use std::io::Cursor;
use std::str::FromStr;
use prost::Message;

pub mod greeter {
    include!(concat!(env!("OUT_DIR"), "/greeter.rs"));
}

//pub mod slip_proto {
//    include!(concat!(env!("OUT_DIR"), "/slip_proto.rs"));
//}

fn make_mock_slip() -> Slip {
    let public_key: PublicKey = PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072").unwrap();
    Slip::new_mock(
        public_key,
        SlipBroadcastType::Normal,
        10_000_000,
        (0..512)
        .map(|_| rand::random::<u8>())
        .collect()
    )
}

fn slip_bespoke_serialize(slip: &Slip) {
    let _serialized_slip: [u8; 1024] = slip.serialize();
}

fn bench_slip_bespoke_serialize(c: &mut Criterion) {
    let slip = make_mock_slip();
    c.bench_function("slip bespoke seriliazation", |b| b.iter(|| slip_bespoke_serialize(&slip)));
}


fn slip_bespoke_serialize2(slip: &Slip) {
    let _serialized_slip = slip.serialize2();
}

fn bench_slip_bespoke_serialize2(c: &mut Criterion) {
    let slip = make_mock_slip();
    c.bench_function("slip bespoke seriliazation", |b| b.iter(|| slip_bespoke_serialize2(&slip)));
}

fn slip_bincode_serialize(slip: &Slip) {
    let _xs: Vec<u8> = bincode::serialize(&slip).unwrap();
}

fn bench_slip_bincode_serialize(c: &mut Criterion) {
    let slip = make_mock_slip();
    c.bench_function("slip bincode seriliazation", |b| b.iter(|| slip_bincode_serialize(&slip)));
}

fn slip_cbor_serialize(slip: &Slip) {
    let _bytes = serde_cbor::to_vec(&slip).unwrap();
}

fn bench_slip_cbor_serialize(c: &mut Criterion) {
    let slip = make_mock_slip();

    c.bench_function("slip cbor seriliazation", |b| b.iter(|| slip_cbor_serialize(&slip)));
}

fn slip_proto_serialize(slip: &Slip) {
    let proto: proto::Slip = slip.clone().into();
    let mut buf = Vec::new();
    buf.reserve(proto.encoded_len());
    proto.encode(&mut buf).unwrap();
}

fn bench_slip_proto_serialize(c: &mut Criterion) {
    let slip = make_mock_slip();

    c.bench_function("slip protocol bufs seriliazation", |b| b.iter(|| slip_proto_serialize(&slip)));
}

criterion_group!(
    benches,
    bench_slip_bincode_serialize,
    bench_slip_cbor_serialize,
    bench_slip_proto_serialize,
    bench_slip_bespoke_serialize,
    bench_slip_bespoke_serialize2,
);
criterion_main!(benches);