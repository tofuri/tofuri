# Pea

A cryptocurrency made in rust.

## [API Documentation](https://docs.rs/pea)

## [Examples](https://github.com/peacash/pea/tree/main/examples)

`examples/api_get_height.rs`

```rust
use pea::api::get;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let height = get::height("http://localhost:8080").await?;
    println!("{}", height);
    Ok(())
}
```

`examples/api_get_block.rs`

```rust
use pea::api::get;
use std::error::Error;
const API: &str = "http://localhost:8080";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let height = get::height(API).await?;
    let hash = get::hash(API, &height).await?;
    let block = get::block(API, &hash).await?;
    println!("{:?}", block);
    Ok(())
}
```
