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
use std::env;

use saito_rust::consensus;
use log::{info, warn, debug, error};

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // set default RUST_LOG to "info"
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    
    tracing_subscriber::fmt::init();
    debug!("this is a debug {}", "message");
    warn!("this is a warninig");
    info!("this is info");
    error!("this is printed by default");
    consensus::run().await
}
