use clap::Parser;
use colored::*;
use libp2p::Multiaddr;
use log::info;
use pea::{blockchain::Blockchain, p2p};
use pea_address as address;
use pea_db as db;
use pea_logger as logger;
use pea_wallet::Wallet;
use std::error::Error;
use tempdir::TempDir;
use tokio::net::TcpListener;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Multiaddr to listen on
    #[clap(short, long, value_parser, default_value = "/ip4/0.0.0.0/tcp/9333")]
    pub host: String,
    /// Multiaddr to dial
    #[clap(short, long, value_parser, default_value = "")]
    pub peer: String,
    /// TCP socket address to bind to
    #[clap(long, value_parser, default_value = ":::9332")]
    pub bind_http_api: String,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = false)]
    pub tempkey: bool,
    /// Ticks per second
    #[clap(long, value_parser, default_value = "5")]
    pub tps: f64,
    /// Trust fork after blocks
    #[clap(long, value_parser, default_value = "128")]
    pub trust: usize,
    /// Pending blocks limit
    #[clap(long, value_parser, default_value = "10")]
    pub pending: usize,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    logger::init(args.debug);
    info!("{} {}", "Version".cyan(), env!("CARGO_PKG_VERSION").yellow());
    info!("{} {}", "Commit".cyan(), env!("GIT_HASH").yellow());
    info!("{} {}", "Repository".cyan(), env!("CARGO_PKG_REPOSITORY").yellow());
    info!("{} {}", "--debug".cyan(), args.debug.to_string().magenta());
    info!("{} {}", "--host".cyan(), args.host.magenta());
    info!("{} {}", "--peer".cyan(), args.peer.magenta());
    info!("{} {}", "--bind-http-api".cyan(), args.bind_http_api.magenta());
    info!("{} {}", "--tempdb".cyan(), args.tempdb.to_string().magenta());
    info!("{} {}", "--tempkey".cyan(), args.tempkey.to_string().magenta());
    info!("{} {}", "--trust".cyan(), args.trust.to_string().magenta());
    info!("{} {}", "--pending".cyan(), args.pending.to_string().magenta());
    info!("{} {}", "--tps".cyan(), args.tps.to_string().magenta());
    info!("{} {}", "--wallet".cyan(), args.wallet.magenta());
    info!("{} {}", "--passphrase".cyan(), "*".repeat(args.passphrase.len()).magenta());
    let tempdir = TempDir::new("pea")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./storage/pea",
    };
    let db = db::open(path);
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import(&args.wallet, &args.passphrase)?,
    };
    info!("{} {}", "PubKey".cyan(), address::public::encode(&wallet.key.public_key_bytes()).green());
    let mut blockchain = Blockchain::new(db, wallet.key, args.trust, args.pending);
    let peers = db::peer::get_all(&blockchain.db);
    info!("{} {}", "Peers".cyan(), format!("{:?}", peers).yellow());
    blockchain.load();
    let mut swarm = p2p::swarm(blockchain, args.tps).await?;
    swarm.listen_on(args.host.parse()?)?;
    swarm.dial(args.peer.parse::<Multiaddr>()?)?;
    for peer in peers {
        swarm.dial(peer.parse::<Multiaddr>()?)?;
    }
    let tcp_listener_http_api = if args.bind_http_api != "" { Some(TcpListener::bind(args.bind_http_api).await?) } else { None };
    p2p::listen(&mut swarm, tcp_listener_http_api).await?;
    Ok(())
}
