use crate::processor::PaymentProcessor;
use async_std::{
    io::{ReadExt, WriteExt},
    net::TcpStream,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{error::Error, io::BufRead};
lazy_static! {
    static ref GET: Regex = Regex::new(r"^GET .* HTTP/1.1$").unwrap();
    static ref INDEX: Regex = Regex::new(r" / ").unwrap();
    static ref CHARGES: Regex = Regex::new(r" /charges ").unwrap();
    static ref CHARGE: Regex = Regex::new(r" /charge/[0-9A-Fa-f]* ").unwrap();
    static ref CHARGE_NEW: Regex = Regex::new(r" /charge/new/[0-9]* ").unwrap();
}
pub async fn handler(mut stream: TcpStream, payment_processor: &mut PaymentProcessor) -> Result<String, Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    write(&mut stream, if GET.is_match(&first) { get(payment_processor, &first) } else { c405() }?).await?;
    stream.flush().await?;
    Ok(first)
}
fn get(payment_processor: &mut PaymentProcessor, first: &str) -> Result<String, Box<dyn Error>> {
    if INDEX.is_match(first) {
        get_index()
    } else if CHARGES.is_match(first) {
        get_charges(payment_processor)
    } else if CHARGE.is_match(first) {
        get_charge(payment_processor, first)
    } else if CHARGE_NEW.is_match(first) {
        get_charge_new(payment_processor, first)
    } else {
        c404()
    }
}
async fn write(stream: &mut TcpStream, string: String) -> Result<(), Box<dyn Error>> {
    stream.write_all(string.as_bytes()).await?;
    Ok(())
}
fn json(string: String) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
        string
    )
}
fn text(string: String) -> String {
    format!(
        "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *

{}",
        string
    )
}
fn get_index() -> Result<String, Box<dyn Error>> {
    Ok(text(format!(
        "\
{} = {{ version = \"{}\" }}
{}/tree/{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("GIT_HASH"),
    )))
}
fn get_charges(payment_processor: &PaymentProcessor) -> Result<String, Box<dyn Error>> {
    Ok(json(serde_json::to_string(&payment_processor.get_charges())?))
}
fn get_charge(payment_processor: &PaymentProcessor, first: &str) -> Result<String, Box<dyn Error>> {
    let hash = hex::decode(CHARGE.find(first).ok_or("GET CHARGE 1")?.as_str().trim().get(8..).ok_or("GET CHARGE 2")?)?;
    let payment = payment_processor.get_charge(&hash);
    Ok(json(serde_json::to_string(&payment)?))
}
fn get_charge_new(payment_processor: &mut PaymentProcessor, first: &str) -> Result<String, Box<dyn Error>> {
    let amount: u128 = CHARGE_NEW
        .find(first)
        .ok_or("GET CHARGE 1")?
        .as_str()
        .trim()
        .get(12..)
        .ok_or("GET CHARGE 2")?
        .parse()?;
    let (hash, payment) = payment_processor.charge(amount);
    Ok(json(serde_json::to_string(&(hash, payment))?))
}
fn c404() -> Result<String, Box<dyn Error>> {
    Ok("HTTP/1.1 404 Not Found".to_string())
}
fn c405() -> Result<String, Box<dyn Error>> {
    Ok("HTTP/1.1 405 Method Not Allowed".to_string())
}
