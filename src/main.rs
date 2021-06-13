use saito_rust::consensus;
use tokio::signal;

use tokio::time;
use std::time::Duration;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    consensus::run(signal::ctrl_c()).await
}
