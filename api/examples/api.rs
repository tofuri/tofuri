use pea_api as api;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", api::get::index("localhost:9332").await?);
    println!("{:?}", api::get::sync("localhost:9332").await?);
    Ok(())
}
