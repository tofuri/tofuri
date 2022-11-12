use colored::Colorize;
use pea_pay_core::Payment;
use std::error::Error;
const HTTP_API: &str = "http://localhost:9331";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let amount = 100;
    let (hash, charge) = reqwest::get(format!("{}/charge/new/{}", HTTP_API, amount)).await?.json::<(String, Payment)>().await?;
    println!("{:?}", charge);
    println!("{}/charge/{}", HTTP_API, hash.green());
    Ok(())
}
