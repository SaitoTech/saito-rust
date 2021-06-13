use saito_rust::consensus;
use tokio::signal;

use tokio::time;
use std::time::Duration;

#[tokio::main]
pub async fn main() -> saito_rust::Result<()> {


        let mut interval = time::interval(Duration::from_millis(1000));
        interval.tick().await;
println!("1");
        interval.tick().await;
println!("2");

    consensus::run(signal::ctrl_c()).await
}
