# saito-rust

A high-performance implementation of Saito in Rust.

This project will serve as the reference implementation for other code-bases. It will also be a flexible implementation which can be easily extended to support various testnet implementations which we envision. For example, we may use different epoch times or different sources of randomness for the Golden Ticket "Lottery Game" instead of sha256 difficult hashes(e.g. PoS or even 3rd party sources of randomness like BTC block hashes).

## Contributing

The next month of sprint planning can always [be found on our github kanban board](https://github.com/orgs/SaitoTech/projects/5).

PRs should be made against the main branch in [the saito-rust project](https://github.com/SaitoTech/saito-rust). 

PRs must pass rust fmt check and should have full test coverage.

## Roadmap

A long-view backlog [can be seen on Trello](https://trello.com/b/gWPiQFZ1/saito-rust-project-management) for those who may be interested in such things. We use this for coordinating the team at a high level, to keep us focused on meaningful goals(user stories), and to try to produce an accurate roadmap. If you're looking for something to code, the sprint planning board will be better.

https://trello.com/b/gWPiQFZ1/saito-rust-project-management

## Documentation

- https://saitotech.github.io/saito-rust/saito_rust/index.html
- [Architecture doc](ARCHITECTURE.md)

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

Format code according to the [Rust style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md). cargo fmt make this easy.

### Benchmarks

```
cargo bench
```

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
