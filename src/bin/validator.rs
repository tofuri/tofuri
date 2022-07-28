use axiom::{
    db, p2p,
    util::{print, ValidatorArgs},
    validator::Validator,
    wallet::Wallet,
};
use clap::Parser;
use std::error::Error;
use tempdir::TempDir;
use tokio::net::TcpListener;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    print::env_logger_init();
    print::build();
    let args = ValidatorArgs::parse();
    print::validator_args(&args);
    let tempdir = TempDir::new("rocksdb")?;
    let path: &str = match args.tempdb {
        true => tempdir.path().to_str().unwrap(),
        false => "./db",
    };
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import()?,
    };
    let db = db::open(path);
    let known = Validator::get_known(&args)?;
    print::known_peers(&known);
    let validator = Validator::new(wallet, db, known)?;
    print::validator(&validator);
    print::blockchain(&validator.blockchain);
    let mut swarm = p2p::swarm(validator).await?;
    swarm.listen_on(args.multiaddr.parse()?)?;
    let listener = TcpListener::bind(args.http).await?;
    print::http(&listener)?;
    print::listen();
    Validator::listen(&mut swarm, listener).await?;
    Ok(())
}
