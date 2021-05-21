extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/proto/message.proto"], &["src/"]).unwrap();
    prost_build::compile_protos(&["src/proto/slip.proto"], &["src/"]).unwrap();
}
