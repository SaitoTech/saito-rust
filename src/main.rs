#[tokio::main]
async fn main() -> saito_rust::Result<()> {
    saito_rust::server::run().await
}
