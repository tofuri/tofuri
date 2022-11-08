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
}
pub async fn next(listener: &tokio::net::TcpListener) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
    Ok(listener.accept().await?.0)
}
pub async fn handler(mut stream: tokio::net::TcpStream, payment_processor: &PaymentProcessor) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    let first = buffer.lines().next().ok_or("http request first line")??;
    info!(
        "{} {} {}",
        "HTTP API".cyan(),
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
async fn handler_get(stream: &mut tokio::net::TcpStream, payment_processor: &PaymentProcessor, first: &str) -> Result<(), Box<dyn Error>> {
    if INDEX.is_match(first) {
        handler_get_index(stream).await?;
    } else if CHARGES.is_match(first) {
        handler_get_json_charges(stream, payment_processor).await?;
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
async fn handler_get_json_charges(stream: &mut tokio::net::TcpStream, payment_processor: &PaymentProcessor) -> Result<(), Box<dyn Error>> {
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
async fn handler_404(stream: &mut tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    stream.write_all("HTTP/1.1 404 NOT FOUND".as_bytes()).await?;
    Ok(())
}
