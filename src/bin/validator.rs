use axiom::{cli::ValidatorArgs, db, p2p, print, validator::Validator, wallet::Wallet};
use clap::Parser;
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
    let known = Validator::get_known(&args)?;
    print::known_peers(&known);
    let wallet = match args.tempkey {
        true => Wallet::new(),
        false => Wallet::import(&args.wallet, &args.passphrase)?,
    };
    let validator = Validator::new(wallet, db, known)?;
    print::validator(&validator);
    print::blockchain(&validator.blockchain);
    let mut swarm = p2p::swarm(validator).await?;
    swarm.listen_on(args.multiaddr.parse()?)?;
    let listener = TcpListener::bind(args.http).await?;
    print::http(&listener)?;
    Validator::listen(&mut swarm, listener).await?;
    Ok(())
}
