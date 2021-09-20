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

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    consensus::run().await
}

