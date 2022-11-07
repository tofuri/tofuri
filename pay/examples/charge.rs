use pea_core::util;
use pea_pay::PaymentProcessor;
use std::{error::Error, thread, time::Duration};
const HTTP_API: &str = "http://localhost:9332";
const CONFIRMATIONS: usize = 10;
const EXPIRES_AFTER_SECS: u32 = 60;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let keypair = util::keygen();
    let mut payment_processor = PaymentProcessor::new(HTTP_API.to_string(), keypair.secret.to_bytes(), CONFIRMATIONS, EXPIRES_AFTER_SECS);
    println!("{:?}", payment_processor);
    let payment = payment_processor.charge(10000000000);
    println!("{:?}", payment);
    loop {
        thread::sleep(Duration::from_millis(500));
        let payments = payment_processor.check().await?;
        println!("{:?}", payments);
    }
}
