use pea_api::get;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let height = get::height("http://localhost:8080").await?;
    println!("{}", height);
    Ok(())
}
