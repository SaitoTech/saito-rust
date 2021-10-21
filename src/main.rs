/*!
# Saito Command Line Interface

## Help

```bash
saito_rust help
```

## Example Usage

```bash
saito_rust --password=asdf --key_path=test/testwallet
```

## Dev

To run from source:

```bash
cargo run -- --help
cargo run -- --password=asdf --key_path=test/testwallet
```
*/
use saito_rust::consensus;
use std::env;

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // set default RUST_LOG level
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::init();

    let level = env::var("RUST_LOG").unwrap(); //safe to unwrap, set above
    println!(
        "LOG LEVEL SET TO: {}. To set log level use RUST_LOG=[trace, info, debug, warn, error]",
        level
    );

    consensus::run().await
}
