use clap::Parser;
use colored::*;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use log::info;
use log::warn;
use pea::api_internal;
use pea::interval;
use pea::swarm;
use pea::Node;
use pea_address::address;
use pea_blockchain::blockchain::Blockchain;
use pea_core::*;
use pea_db as db;
use pea_key::Key;
use pea_p2p::multiaddr;
use pea_p2p::P2p;
use pea_util;
use pea_wallet::wallet;
use std::collections::HashSet;
use std::time::Duration;
use tempdir::TempDir;
use tokio::net::TcpListener;
#[tokio::main]
async fn main() {
    let mut args = pea::Args::parse();
    pea_logger::init(args.debug);
    info!(
        "{} = {{ version = \"{}\" }}",
        env!("CARGO_PKG_NAME").yellow(),
        env!("CARGO_PKG_VERSION").magenta()
    );
    info!("{}/tree/{}", env!("CARGO_PKG_REPOSITORY").yellow(), env!("GIT_HASH").magenta());
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
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--mint".cyan(), args.mint.to_string().magenta());
    info!("{} {}", "--time-api".cyan(), args.time_api.to_string().magenta());
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--ban-offline".cyan(), args.ban_offline.to_string().magenta());
    info!("{} {}", "--time-delta".cyan(), args.time_delta.to_string().magenta());
    info!("{} {}", "--max-established".cyan(), format!("{:?}", args.max_established).magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--bind-api".cyan(), args.api_internal.magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    info!("{} {}", "--dev".cyan(), args.dev.to_string().magenta());
    if args.dev {
        warn!("{}", "DEVELOPMENT MODE IS ACTIVATED!".yellow());
    }
    let key = match args.tempkey {
        true => Key::generate(),
        false => wallet::load(&args.wallet, &args.passphrase).unwrap().3,
    };
    info!("Address {}", address::encode(&key.address_bytes()).green());
    let tempdir = TempDir::new("peacash-db").unwrap();
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./peacash-db",
    };
    let db = db::open(path);
    let mut known = HashSet::new();
    if let Some(multiaddr) = multiaddr::ip_port(&args.peer.parse::<Multiaddr>().unwrap()) {
        known.insert(multiaddr);
    }
    let peers = db::peer::get_all(&db);
    for peer in peers {
        if let Some(multiaddr) = multiaddr::ip_port(&peer.parse::<Multiaddr>().unwrap()) {
            known.insert(multiaddr);
        }
    }
    let p2p = P2p::new(args.max_established, args.timeout, known, args.ban_offline).await.unwrap();
    let blockchain = Blockchain::new();
    let mut node = Node {
        key,
        p2p,
        blockchain,
        db,
        args,
        ticks: 0,
        lag: 0.0,
    };
    node.blockchain.load(&node.db, node.args.trust);
    info!(
        "Blockchain height is {}",
        if let Some(main) = node.blockchain.tree.main() {
            main.1.to_string().yellow()
        } else {
            "0".red()
        }
    );
    info!("Latest block seen {}", node.blockchain.last_seen().yellow());
    let multiaddr: Multiaddr = node.args.host.parse().unwrap();
    node.p2p.swarm.listen_on(multiaddr.clone()).unwrap();
    info!("Swarm is listening on {}", multiaddr.to_string().magenta());
    let listener = TcpListener::bind(&node.args.api_internal).await.unwrap();
    info!(
        "API is listening on {}{}",
        "http://".cyan(),
        listener.local_addr().unwrap().to_string().magenta()
    );
    let mut interval = tokio::time::interval(Duration::from_micros(pea_util::micros_per_tick(node.args.tps)));
    loop {
        tokio::select! {
            biased;
            instant = interval.tick() => interval::tick(&mut node, instant),
            event = node.p2p.swarm.select_next_some() => swarm::event(&mut node, event),
            res = listener.accept() => api_internal::accept(&mut node, res).await,
        }
    }
}
