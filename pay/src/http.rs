use crate::processor::PaymentProcessor;
use async_std::{
    io::{ReadExt, WriteExt},
    net::TcpStream,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{error::Error, io::BufRead};
lazy_static! {
    static ref GET: Regex = Regex::new(r"^GET [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref POST: Regex = Regex::new(r"^POST [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref INDEX: Regex = Regex::new(r" / ").unwrap();
    static ref CHARGES: Regex = Regex::new(r" /charges ").unwrap();
    static ref CHARGE: Regex = Regex::new(r" /charge/[0-9A-Fa-f]* ").unwrap();
    static ref CHARGE_NEW: Regex = Regex::new(r" /charge/new/[0-9]* ").unwrap();
}
pub async fn handler(mut stream: TcpStream, payment_processor: &mut PaymentProcessor) -> Result<String, Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    if GET.is_match(&first) {
        get(&mut stream, payment_processor, &first).await?;
    } else {
        c405(&mut stream).await?;
    };
    stream.flush().await?;
    Ok(first)
}
async fn get(stream: &mut TcpStream, payment_processor: &mut PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
    if INDEX.is_match(first) {
        get_index(stream).await?;
    } else if CHARGES.is_match(first) {
        get_charges(stream, payment_processor).await?;
    } else if CHARGE.is_match(first) {
        get_charge(stream, payment_processor, first).await?;
    } else if CHARGE_NEW.is_match(first) {
        get_charge_new(stream, payment_processor, first).await?;
    } else {
        c404(stream).await?;
    };
    Ok(())
}
async fn get_index(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *

{} = {{ version = \"{}\" }}
{}/tree/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_REPOSITORY"),
                env!("GIT_HASH"),
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn get_charges(stream: &mut TcpStream, payment_processor: &PaymentProcessor) -> Result<(), Box<dyn Error>> {
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&payment_processor.get_charges())?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn get_charge(stream: &mut TcpStream, payment_processor: &PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
    let hash = hex::decode(CHARGE.find(first).ok_or("GET CHARGE 1")?.as_str().trim().get(8..).ok_or("GET CHARGE 2")?)?;
    let payment = payment_processor.get_charge(&hash);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&payment)?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn get_charge_new(stream: &mut TcpStream, payment_processor: &mut PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
    let amount: u128 = CHARGE_NEW
        .find(first)
        .ok_or("GET CHARGE 1")?
        .as_str()
        .trim()
        .get(12..)
        .ok_or("GET CHARGE 2")?
        .parse()?;
    let (hash, payment) = payment_processor.charge(amount);
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Type: application/json

{}",
                serde_json::to_string(&(hash, payment))?
            )
            .as_bytes(),
        )
        .await?;
    Ok(())
}
async fn c404(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    stream.write_all("HTTP/1.1 404 Not Found".as_bytes()).await?;
    Ok(())
}
async fn c405(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    stream.write_all("HTTP/1.1 405 Method Not Allowed".as_bytes()).await?;
    Ok(())
}
