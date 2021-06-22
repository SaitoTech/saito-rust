use saito_rust::consensus;
use tokio::signal;
use tracing_subscriber;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    tracing_subscriber::fmt::init();
    consensus::run(signal::ctrl_c()).await
}
