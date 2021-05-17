use saito_rust::server;
use tokio::signal;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {
    server::run(signal::ctrl_c()).await
}
