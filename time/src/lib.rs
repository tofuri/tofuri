use chrono::DateTime;
use serde::Deserialize;
use std::{
    error::Error,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Debug)]
pub struct Time {
    pub diff: i64,
    pub requests: usize,
}
impl Time {
    pub fn new(requests: usize) -> Time {
        Time { diff: 0, requests }
    }
    pub async fn sync(&mut self) -> bool {
        if let Ok(a) = avg(self.requests).await {
            self.diff = a;
            true
        } else {
            self.diff = 0;
            false
        }
    }
    pub fn timestamp_micros(&self) -> u64 {
        (timestamp() - self.diff) as u64
    }
    pub fn timestamp_secs(&self) -> u32 {
        (self.timestamp_micros() / 1_000_000) as u32
    }
}
fn timestamp() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros() as i64
}
#[derive(Debug, Deserialize)]
struct Data {
    utc_datetime: String,
}
async fn get() -> Result<i64, Box<dyn Error>> {
    let start = Instant::now();
    let data = reqwest::get("https://worldtimeapi.org/api/timezone/CET").await?.json::<Data>().await?;
    let duration = start.elapsed().as_micros() as u64;
    let rfc3339 = DateTime::parse_from_rfc3339(&data.utc_datetime)?;
    let world = rfc3339.timestamp_micros() as i64 + (duration / 2) as i64;
    let local = timestamp();
    Ok(local - world as i64)
}
async fn avg(requests: usize) -> Result<i64, Box<dyn Error>> {
    let mut sum = 0;
    for _ in 0..requests {
        sum += get().await?;
    }
    Ok(sum / requests as i64)
}
