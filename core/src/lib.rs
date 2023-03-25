pub type AmountBytes = [u8; AMOUNT_BYTES];
pub type Hash = [u8; 32];
pub type Checksum = [u8; 4];
pub type MerkleRoot = [u8; 32];
pub type Beta = [u8; 32];
pub type Pi = [u8; 81];
pub type AddressBytes = [u8; 20];
pub type PublicKeyBytes = [u8; 33];
pub type SecretKeyBytes = [u8; 32];
pub type SignatureBytes = [u8; 64];
pub const PREFIX_ADDRESS: &str = "0x";
pub const PREFIX_SECRET_KEY: &str = "SECRETx";
pub const BLOCK_SIZE_LIMIT: usize = 57797;
pub const MAX_TRANSMIT_SIZE: usize = 1_000_000;
pub const PROTOCOL_VERSION: &str = "tofuri/1.0.0";
pub const PROTOCOL_NAME: &str = "/sync/1";
pub const DECIMAL_PLACES: usize = 18;
pub const COIN: u128 = 10_u128.pow(DECIMAL_PLACES as u32);
pub const BLOCK_TIME: u32 = 60;
pub const ELAPSED: u32 = 90;
pub const EXTENSION: &str = "tofuri";
pub const AMOUNT_BYTES: usize = 4;
pub const GENESIS_BLOCK_BETA: Beta = [0; 32];
pub const GENESIS_BLOCK_PREVIOUS_HASH: Beta = [0; 32];
pub const RECOVERY_ID: i32 = 0;
pub const TEMP_DB: bool = false;
pub const TEMP_DB_DEV: bool = true;
pub const TEMP_KEY: bool = false;
pub const TEMP_KEY_DEV: bool = true;
pub const RPC: &str = ":::9332";
pub const RPC_DEV: &str = ":::9334";
pub const HOST: &str = "/ip4/0.0.0.0/tcp/9333";
pub const HOST_DEV: &str = "/ip4/0.0.0.0/tcp/9335";
pub const API: &str = "0.0.0.0:80";
pub const API_DEV: &str = "0.0.0.0:3000";
pub const HTTP_API: &str = "http://localhost:80";
pub const HTTP_API_DEV: &str = "http://localhost:3000";
pub const PAY_API: &str = "0.0.0.0:4000";
pub const PAY_API_DEV: &str = "0.0.0.0:5000";
pub const P2P_RATELIMIT_REQUEST_TIMEOUT: u32 = 3600;
pub const P2P_RATELIMIT_RESPONSE_TIMEOUT: u32 = 3600;
pub const P2P_RATELIMIT_REQUEST: usize = 60 + 1;
pub const P2P_RATELIMIT_RESPONSE: usize = 60 + 1;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_BLOCK: usize = 1 + 1;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_TRANSACTION: usize = 60 * 100;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_STAKE: usize = 60 * 100;
pub const P2P_RATELIMIT_GOSSIPSUB_MESSAGE_PEERS: usize = 1 + 1;
