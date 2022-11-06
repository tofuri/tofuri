use pea_core::util;
use pea_pay::PaymentProcessor;
use std::error::Error;
const HTTP_API: &str = "http://localhost:9332";
const CONFIRMATIONS: usize = 10;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let keypair = util::keygen();
    let payment_processor = PaymentProcessor::new(HTTP_API.to_string(), keypair.secret.to_bytes(), CONFIRMATIONS);
    let data = payment_processor.check().await;
    println!("{:?}", data);
    Ok(())
}
