use pea_time::Time;
#[tokio::main]
async fn main() {
    let mut time = Time::new();
    println!("Success {}", time.sync().await);
    println!("Time difference {}", time.diff);
    println!("World time (micros) {}", time.timestamp_micros());
    println!("World time (secs) {}", time.timestamp_secs());
}
