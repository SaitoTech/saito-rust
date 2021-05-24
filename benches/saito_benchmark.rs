use criterion::{criterion_group, criterion_main, Criterion};
use saito_rust::{
    keypair::Keypair,
    slip::{Slip, SlipBroadcastType}
};

use saito_rust::slip_proto as proto;
use std::io::Cursor;

use prost::Message;

pub mod greeter {
    include!(concat!(env!("OUT_DIR"), "/greeter.rs"));
}

//pub mod slip_proto {
//    include!(concat!(env!("OUT_DIR"), "/slip_proto.rs"));
//}

fn slip_bespoke_serialize(slip: Slip) {

}

fn bench_slip_bespoke_serialize(c: &mut Criterion) {
    let keypair = Keypair::new();
    let slip = Slip::new(
        keypair.public_key().clone(),
        SlipBroadcastType::Normal,
        10_000_000,
    );

    c.bench_function("slip bespoke seriliazation", |b| b.iter(|| slip_bespoke_serialize(slip)));
}

fn slip_bincode_serialize(slip: Slip) {
    let xs: Vec<u8> = bincode::serialize(&slip).unwrap();
    //let xd: Slip = bincode::deserialize(&xs).unwrap();
    assert!(xs.len() > 0);
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
    assert!(bytes.len() > 0);
    //let slp: Slip = serde_cbor::from_slice(&bytes).unwrap();
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

fn slip_proto_serialize(slip: Slip) {
    let proto: proto::Slip = slip.clone().into();
    let mut buf = Vec::new();
    buf.reserve(proto.encoded_len());
    proto.encode(&mut buf).unwrap();
    assert!(buf.len() > 0);
    //let decoded_slip = proto::Slip::decode(&mut Cursor::new(buf)).unwrap();
}

fn bench_slip_proto_serialize(c: &mut Criterion) {
    let keypair = Keypair::new();
    let slip = Slip::new(
        keypair.public_key().clone(),
        SlipBroadcastType::Normal,
        10_000_000,
    );

    c.bench_function("slip protocol bufs seriliazation", |b| b.iter(|| slip_proto_serialize(slip)));
}

criterion_group!(
    benches,
    bench_slip_bincode_serialize,
    bench_slip_cbor_serialize,
    bench_slip_proto_serialize
);
criterion_main!(benches);