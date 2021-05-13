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

Format code according to the [Rust style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md)

### Github Actions

GH Actions are located here: [.github/workflows](.github/workflows)

- cargo docs
  Is creating and deploying the docs to GH pages

- [rustfmt](https://github.com/rust-lang/rustfmt#checking-style-on-a-ci-server) (**required**)
  Is checking if the code is formatted according to rust style guidelines

- cargo build & test
  Tries to build the code and run all tests

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
