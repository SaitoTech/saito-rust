# Welcome to Saito

Saito is a **Tier 1 Blockchain Protocol** that incentivizes the provision of **high-throughput** network infrastructure. The network accomplishes with a consensus mechanism that pays the nodes in the peer-to-peer network for the collection and sharing of fees.

Saito-Rust is an implementation of Saito Consensus written in Rust for use by high-throughput routing nodes. It aims to offer the simplest and most scalable implementation of Saito Consensus.

If you need to get in touch with us, please reach out anytime.

The Saito Team  
dev@saito.tech

## Dev workflow

### Deps

```
rustup component add rustfmt
```

### Run the node

```
cargo run
```

### Tests

```
cargo test
```

### Code formatting

```
cargo fmt
```

### VSCode

Extensions:

- https://github.com/rust-lang/vscode-rust

## Create release

```
cargo build --release
```

## Further steps

Publish rust bin as npm package

- https://blog.woubuc.be/post/publishing-rust-binary-on-npm
- https://github.com/EverlastingBugstopper/binary-install
