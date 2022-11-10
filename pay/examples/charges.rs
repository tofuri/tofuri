use pea_pay::processor::Payment;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let charge = reqwest::get("http://localhost:9331/charges").await?.json::<Payment>().await?;
    println!("{:?}", charge);
    Ok(())
}
