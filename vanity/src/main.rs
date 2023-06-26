use address;
use address::public;
use address::secret;
use clap::Parser;
use key::Key;
use std::io::BufRead;
use std::io::BufReader;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Threads
    #[clap(long, value_parser, default_value = "1")]
    pub threads: usize,
}
fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let (layer, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let args = Args::parse();
    let best = Arc::new(Mutex::new([0xff; 20]));
    let attempts = Arc::new(AtomicUsize::new(0));
    let handles = (0..args.threads)
        .map(|_| {
            let best = best.clone();
            let attempts = attempts.clone();
            std::thread::spawn(move || generate(&best, &attempts))
        })
        .collect::<Vec<_>>();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let attempts_per_second = attempts.load(Ordering::Relaxed);
        debug!(attempts_per_second);
        attempts.store(0, Ordering::Relaxed);
    });
    io_reload_filter(reload_handle);
    for handle in handles {
        handle.join().unwrap();
    }
}
fn generate(best: &Arc<Mutex<[u8; 20]>>, attempts: &AtomicUsize) {
    loop {
        let key = Key::generate();
        let address_bytes = key.address_bytes();
        let mut locked_best = best.lock().unwrap();
        if address_bytes.cmp(&locked_best) == std::cmp::Ordering::Less {
            *locked_best = address_bytes;
            let address = public::encode(&address_bytes);
            let secret = secret::encode(&key.secret_key_bytes());
            let zeroes = address.chars().skip(2).take_while(|c| *c == '0').count();
            info!(zeroes, address, secret);
        }
        attempts.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn io_reload_filter(reload_handle: reload::Handle<EnvFilter, Registry>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(std::io::stdin());
        let mut line = String::new();
        loop {
            _ = reader.read_line(&mut line);
            let filter = EnvFilter::new(line.trim());
            info!(?filter, "Reload");
            if let Err(e) = reload_handle.modify(|x| *x = filter) {
                error!(?e)
            }
            line.clear();
        }
    });
}
