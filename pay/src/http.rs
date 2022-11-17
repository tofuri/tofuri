use crate::processor::PaymentProcessor;
use colored::*;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use std::{error::Error, io::BufRead};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
lazy_static! {
    static ref GET: Regex = Regex::new(r"^GET [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref POST: Regex = Regex::new(r"^POST [/_0-9A-Za-z]+ HTTP/1.1$").unwrap();
    static ref INDEX: Regex = Regex::new(r" / ").unwrap();
    static ref CHARGES: Regex = Regex::new(r" /charges ").unwrap();
    static ref CHARGE: Regex = Regex::new(r" /charge/[0-9A-Fa-f]* ").unwrap();
    static ref CHARGE_NEW: Regex = Regex::new(r" /charge/new/[0-9]* ").unwrap();
}
pub async fn next(listener: &tokio::net::TcpListener) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
    Ok(listener.accept().await?.0)
}
pub async fn handler(mut stream: tokio::net::TcpStream, payment_processor: &mut PaymentProcessor) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    info!(
        "{} {} {}",
        "API".cyan(),
        first.green(),
        match stream.peer_addr() {
            Ok(addr) => addr.to_string().yellow(),
            Err(err) => err.to_string().red(),
        }
    );
    if GET.is_match(&first) {
        handler_get(&mut stream, payment_processor, &first).await?;
    } else {
        handler_404(&mut stream).await?;
    };
    stream.flush().await?;
    Ok(())
}
async fn handler_get(stream: &mut tokio::net::TcpStream, payment_processor: &mut PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
    if INDEX.is_match(first) {
        handler_get_index(stream).await?;
    } else if CHARGES.is_match(first) {
        handler_get_charges(stream, payment_processor).await?;
    } else if CHARGE.is_match(first) {
        handler_get_charge(stream, payment_processor, first).await?;
    } else if CHARGE_NEW.is_match(first) {
        handler_get_charge_new(stream, payment_processor, first).await?;
    } else {
        handler_404(stream).await?;
    };
    Ok(())
}
async fn handler_get_index(stream: &mut tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    stream
        .write_all(
            format!(
                "\
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *

{} {}
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
async fn handler_get_charges(stream: &mut tokio::net::TcpStream, payment_processor: &PaymentProcessor) -> Result<(), Box<dyn Error>> {
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
async fn handler_get_charge(stream: &mut tokio::net::TcpStream, payment_processor: &PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
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
async fn handler_get_charge_new(stream: &mut tokio::net::TcpStream, payment_processor: &mut PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
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
async fn handler_404(stream: &mut tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    stream.write_all("HTTP/1.1 404 NOT FOUND".as_bytes()).await?;
    Ok(())
}
