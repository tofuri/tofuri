use clap::Parser;
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct ValidatorArgs {
    /// Log path to source file
    #[clap(short, long, value_parser, default_value_t = false)]
    pub debug: bool,
    /// Multiaddr to a validator in the network
    #[clap(short, long, value_parser, default_value = "/ip4/0.0.0.0/tcp/0")]
    pub multiaddr: String,
    /// Store blockchain in a temporary database
    #[clap(long, value_parser, default_value_t = false)]
    pub tempdb: bool,
    /// Multiaddr to a validator in the network
    #[clap(long, value_parser, default_value = ":::8080")]
    pub http: String,
    /// Use temporary random keypair
    #[clap(long, value_parser, default_value_t = false)]
    pub tempkey: bool,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct WalletArgs {
    /// Multiaddr to a validator in the network
    #[clap(long, value_parser, default_value = "http://localhost:8080")]
    pub api: String,
    /// Wallet filename
    #[clap(long, value_parser, default_value = "")]
    pub wallet: String,
    /// Passphrase to wallet
    #[clap(long, value_parser, default_value = "")]
    pub passphrase: String,
}
