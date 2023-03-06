use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tofuri::interval;
use tofuri::rpc;
use tofuri::swarm;
use tofuri::Node;
use tofuri::CARGO_PKG_NAME;
use tofuri::CARGO_PKG_REPOSITORY;
use tofuri::CARGO_PKG_VERSION;
use tofuri_address::address;
use tofuri_blockchain::Blockchain;
use tofuri_core::*;
use tofuri_key::Key;
use tofuri_p2p::multiaddr;
use tofuri_p2p::P2p;
use tofuri_tree::Branch;
use tokio::net::TcpListener;
use tokio::time::interval_at;
use tracing::info;
use tracing::metadata::LevelFilter;
use tracing::warn;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy())
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let mut args = tofuri::Args::parse();
    println!("{}", tofuri_util::build(CARGO_PKG_NAME, CARGO_PKG_VERSION, CARGO_PKG_REPOSITORY));
    if args.dev {
        if args.tempdb == TEMP_DB {
            args.tempdb = DEV_TEMP_DB;
        }
        if args.tempkey == TEMP_KEY {
            args.tempkey = DEV_TEMP_KEY;
        }
        if args.api_internal == API_INTERNAL {
            args.api_internal = DEV_API_INTERNAL.to_string();
        }
        if args.host == HOST {
            args.host = DEV_HOST.to_string();
        }
    }
    println!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    println!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    println!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    println!("{} {}", "--mint".cyan(), args.mint.to_string().magenta());
    println!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    println!("{} {}", "--time-delta".cyan(), args.time_delta.to_string().magenta());
    println!("{} {}", "--max-established".cyan(), format!("{:?}", args.max_established).magenta());
    println!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    println!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    println!("{} {}", "--peer".cyan(), args.peer.magenta());
    println!("{} {}", "--bind-api".cyan(), args.api_internal.magenta());
    println!("{} {}", "--host".cyan(), args.host.magenta());
    println!("{} {}", "--dev".cyan(), args.dev.to_string().magenta());
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let key = match args.tempkey {
        true => Key::generate(),
        false => tofuri_wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!(address = address::encode(&key.address_bytes()));
    let tempdir = TempDir::new("tofuri-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./tofuri-db",
    };
    let db = tofuri_db::open(path);
    let mut known = HashSet::new();
    if let Some(multiaddr) = multiaddr::ip_port(&args.peer.parse::<Multiaddr>().unwrap()) {
        known.insert(multiaddr);
    }
    let peers = tofuri_db::peer::get_all(&db);
    for peer in peers {
        if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
    }
    let p2p = P2p::new(args.max_established, args.timeout, known).await.unwrap();
    let blockchain = Blockchain::default();
    let mut node = Node::new(db, key, args, p2p, blockchain);
    node.blockchain.load(&node.db, node.args.trust);
    info!(
        height = node
            .blockchain
            .tree
            .main()
            .unwrap_or(&Branch {
                height: 0,
                hash: [0; 32],
                timestamp: 0
            })
            .height,
    );
    info!(last_seen = node.blockchain.last_seen());
    let multiaddr: Multiaddr = node.args.host.parse().unwrap();
    info!(multiaddr = multiaddr.to_string(), "P2P");
    node.p2p.swarm.listen_on(multiaddr).unwrap();
    let listener = TcpListener::bind(&node.args.api_internal).await.unwrap();
    info!(local_addr = listener.local_addr().unwrap().to_string(), "RPC");
    let start = tofuri_util::interval_at_start();
    let mut interval_a = interval_at(start, Duration::from_secs(1));
    let mut interval_b = interval_at(start, Duration::from_millis(200));
    let mut interval_c = interval_at(start, Duration::from_secs(10));
    let mut interval_d = interval_at(start, Duration::from_secs(60));
    let mut interval_e = interval_at(start, Duration::from_secs(5));
    let mut interval_f = interval_at(start, Duration::from_secs(1));
    loop {
        node.ticks += 1;
        tokio::select! {
            _ = interval_a.tick() => interval::grow(&mut node),
            _ = interval_b.tick() => interval::sync_request(&mut node),
            _ = interval_c.tick() => interval::share(&mut node),
            _ = interval_d.tick() => interval::dial_known(&mut node),
            _ = interval_e.tick() => interval::dial_unknown(&mut node),
            _ = interval_f.tick() => interval::clear(&mut node),
            event = node.p2p.swarm.select_next_some() => swarm::event(&mut node, event),
            res = listener.accept() => rpc::accept(&mut node, res).await
        }
    }
}
