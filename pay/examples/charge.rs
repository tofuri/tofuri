use pea_pay::processor::Payment;
use serde::{Deserialize, Serialize};
use std::error::Error;
#[derive(Serialize, Deserialize, Debug)]
struct Body {
    amount: u128,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let amount = 100;
    let charge = reqwest::get(format!("http://localhost:9331/charge/new/{}", amount)).await?.json::<Payment>().await?;
    println!("{:?}", charge);
    Ok(())
}
