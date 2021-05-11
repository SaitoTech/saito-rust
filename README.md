# saito-rust

A work-in-progress draft implementation of the saito core in rust

## Documentation

- https://saitotech.github.io/saito-rust/saito_rust/index.html
- [Architecture doc](ARCHITECTURE.md)

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

Publish certain rust functionalities (as bin) as a npm package

- https://blog.woubuc.be/post/publishing-rust-binary-on-npm
- https://github.com/EverlastingBugstopper/binary-install
