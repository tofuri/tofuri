use colored::*;
use pea_pay::processor::Payment;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let vec = reqwest::get("http://localhost:9331/charges").await?.json::<Vec<(String, Payment)>>().await?;
    for (hash, charge) in vec.iter() {
        println!("{} {:?}", hash.green(), charge);
    }
    Ok(())
}
