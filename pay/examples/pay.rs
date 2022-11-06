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
    payment_processor.reload_chain().await?;
    println!("{:?}", payment_processor);
    let payment = payment_processor.pay(100);
    println!("{:?}", payment);
    let data = payment_processor.check().await;
    println!("{:?}", data);
    loop {
        thread::sleep(Duration::from_secs(1));
        payment_processor.update_chain().await?;
    }
}
