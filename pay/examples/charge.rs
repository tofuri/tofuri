use pea_address as address;
use pea_core::util;
use pea_pay::PaymentProcessor;
use std::{error::Error, thread, time::Duration};
const HTTP_API: &str = "http://localhost:9332";
const CONFIRMATIONS: usize = 10;
const EXPIRES_AFTER_SECS: u32 = 60;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let keypair = util::keygen();
    let address = address::public::encode(&keypair.public.to_bytes());
    println!("{}", address);
    let mut payment_processor = PaymentProcessor::new(HTTP_API.to_string(), keypair, CONFIRMATIONS, EXPIRES_AFTER_SECS, address);
    println!("{:?}", payment_processor);
    let payment = payment_processor.charge(10000000000);
    println!("{:?}", payment);
    loop {
        thread::sleep(Duration::from_millis(500));
        let payments = payment_processor.check().await?;
        println!("{:?}", payments);
    }
}
