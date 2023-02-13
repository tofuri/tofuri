use pea_api_core::internal::Data;
use pea_api_core::internal::Request;
use std::error::Error;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
pub async fn request(addr: &str, data: Data, vec: Option<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    stream
        .write_all(&bincode::serialize(&Request {
            data,
            vec: vec.unwrap_or(vec![]),
        })?)
        .await?;
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer).await?;
    Ok(buffer.to_vec())
}
