use clap::Parser;
use pea::{cli::ValidatorArgs, db, p2p, print, blockchain::Blockchain, wallet::Wallet};
use std::error::Error;
use tempdir::TempDir;
use tokio::net::TcpListener;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = ValidatorArgs::parse();
    print::env_logger_init(args.debug);
    print::build();
    print::validator_args(&args);
    let tempdir = TempDir::new("rocksdb")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./db",
    };
    let db = db::open(path);
    let known = Blockchain::get_known(&args)?;
    print::known_peers(&known);
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import(&args.wallet, &args.passphrase)?,
    };
    let blockchain = Blockchain::new(wallet.keypair, db, known);
    print::blockchain(&blockchain);
    let mut swarm = p2p::swarm(blockchain).await?;
    swarm.listen_on(args.multiaddr.parse()?)?;
    let listener = TcpListener::bind(args.http).await?;
    print::http(&listener)?;
    Blockchain::listen(&mut swarm, listener).await?;
    Ok(())
}
