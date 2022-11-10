use pea_pay::processor::Payment;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let amount = 100;
    let charge = reqwest::get(format!("http://localhost:9331/charge/new/{}", amount)).await?.json::<Payment>().await?;
    println!("{:?}", charge);
    Ok(())
}
