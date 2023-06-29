use address;
use address::public;
use address::secret;
use axum::extract::State;
use axum::routing::post;
use axum::Router;
use axum::Server;
use clap::Parser;
use key::Key;
use std::net::SocketAddr;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::debug;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Threads
    #[clap(long, value_parser, default_value = "1")]
    pub threads: usize,

    /// Control endpoint
    #[clap(long, env = "CONTROL", default_value = "127.0.0.1:2023")]
    pub control: String,
}
#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let (layer, handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let args = Args::parse();
    if let Ok(addr) = args.control.parse() {
        spawn(handle.clone(), &addr);
    }
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
pub fn spawn(handle: Handle<EnvFilter, Registry>, addr: &SocketAddr) {
    let builder = Server::bind(addr);
    let router = Router::new()
        .route("/", post(handler))
        .layer(TraceLayer::new_for_http())
        .with_state(handle);
    let make_service = router.into_make_service();
    tokio::spawn(async { builder.serve(make_service).await });
}
async fn handler(State(handle): State<Handle<EnvFilter, Registry>>, body: String) {
    handle.reload(body).unwrap();
}
