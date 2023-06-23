pub mod internal;
pub mod router;
use axum::Router;
use axum::Server;
use decimal::Decimal;
use decimal::FromStr;
use hex;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::net::IpAddr;
use std::num::ParseIntError;
use tofuri_address::public;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::info;
use vint::Vint;
pub async fn serve(router: Router, api: String) {
    let addr = api.parse().unwrap();
    info!(?addr, "API listening");
    let make_service = router.into_make_service();
    Server::bind(&addr).serve(make_service).await.unwrap();
}
#[derive(Clone)]
pub struct S {
    pub tx: mpsc::Sender<Request>,
}
impl S {
    pub async fn call<T: DeserializeOwned>(&self, call: Call) -> T {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        let _ = self
            .tx
            .send(Request {
                call,
                oneshot_sender,
            })
            .await;
        let vec = oneshot_receiver.await.unwrap();
        bincode::deserialize(&vec).unwrap()
    }
}
pub struct Request {
    pub call: Call,
    pub oneshot_sender: oneshot::Sender<Vec<u8>>,
}
pub enum Call {
    Balance([u8; 20]),
    BalancePendingMin([u8; 20]),
    BalancePendingMax([u8; 20]),
    Staked([u8; 20]),
    StakedPendingMin([u8; 20]),
    StakedPendingMax([u8; 20]),
    Height,
    HeightByHash([u8; 32]),
    BlockLatest,
    HashByHeight(usize),
    BlockByHash([u8; 32]),
    TransactionByHash([u8; 32]),
    StakeByHash([u8; 32]),
    Peers,
    Peer(IpAddr),
    Transaction(Transaction),
    Stake(Stake),
    Address,
    Ticks,
    TreeSize,
    Sync,
    RandomQueue,
    UnstableHashes,
    UnstableLatestHashes,
    UnstableStakers,
    StableHashes,
    StableLatestHashes,
    StableStakers,
}
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    Address(tofuri_address::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(core::array::TryFromSliceError),
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub cargo_pkg_name: &'static str,
    pub cargo_pkg_version: &'static str,
    pub cargo_pkg_repository: &'static str,
    pub git_hash: &'static str,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockHex {
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u32,
    pub beta: String,
    pub pi: String,
    pub forger_address: String,
    pub signature: String,
    pub transactions: Vec<String>,
    pub stakes: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransactionHex {
    pub input_address: String,
    pub output_address: String,
    pub amount: String,
    pub fee: String,
    pub timestamp: u32,
    pub hash: String,
    pub signature: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StakeHex {
    pub amount: String,
    pub fee: String,
    pub deposit: bool,
    pub timestamp: u32,
    pub signature: String,
    pub input_address: String,
    pub hash: String,
}
impl TryFrom<tofuri_block::Block> for BlockHex {
    type Error = tofuri_key::Error;
    fn try_from(block: tofuri_block::Block) -> Result<Self, Self::Error> {
        Ok(BlockHex {
            hash: hex::encode(block.hash()),
            previous_hash: hex::encode(block.previous_hash),
            timestamp: block.timestamp,
            beta: hex::encode(block.beta()?),
            pi: hex::encode(block.pi),
            forger_address: public::encode(&block.input_address()?),
            signature: hex::encode(block.signature),
            transactions: block
                .transactions
                .iter()
                .map(|x| hex::encode(x.hash()))
                .collect(),
            stakes: block.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
        })
    }
}
impl TryFrom<tofuri_transaction::Transaction> for TransactionHex {
    type Error = tofuri_key::Error;
    fn try_from(transaction: tofuri_transaction::Transaction) -> Result<Self, Self::Error> {
        Ok(TransactionHex {
            input_address: public::encode(&transaction.input_address()?),
            output_address: public::encode(&transaction.output_address),
            amount: u128::from(transaction.amount).decimal::<18>(),
            fee: u128::from(transaction.fee).decimal::<18>(),
            timestamp: transaction.timestamp,
            hash: hex::encode(transaction.hash()),
            signature: hex::encode(transaction.signature),
        })
    }
}
impl TryFrom<tofuri_stake::Stake> for StakeHex {
    type Error = tofuri_key::Error;
    fn try_from(stake: tofuri_stake::Stake) -> Result<Self, Self::Error> {
        Ok(StakeHex {
            amount: u128::from(stake.amount).decimal::<18>(),
            fee: u128::from(stake.fee).decimal::<18>(),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: hex::encode(stake.signature),
            input_address: public::encode(&stake.input_address()?),
            hash: hex::encode(stake.hash()),
        })
    }
}
impl TryFrom<TransactionHex> for tofuri_transaction::Transaction {
    type Error = Error;
    fn try_from(transaction: TransactionHex) -> Result<Self, Self::Error> {
        Ok(tofuri_transaction::Transaction {
            output_address: public::decode(&transaction.output_address).map_err(Error::Address)?,
            amount: Vint::from(
                u128::from_str::<18>(&transaction.amount).map_err(Error::ParseIntError)?,
            ),
            fee: Vint::from(u128::from_str::<18>(&transaction.fee).map_err(Error::ParseIntError)?),
            timestamp: transaction.timestamp,
            signature: hex::decode(&transaction.signature)
                .map_err(Error::Hex)?
                .as_slice()
                .try_into()
                .map_err(Error::TryFromSliceError)?,
        })
    }
}
impl TryFrom<StakeHex> for tofuri_stake::Stake {
    type Error = Error;
    fn try_from(stake: StakeHex) -> Result<Self, Self::Error> {
        Ok(tofuri_stake::Stake {
            amount: Vint::from(u128::from_str::<18>(&stake.amount).map_err(Error::ParseIntError)?),
            fee: Vint::from(u128::from_str::<18>(&stake.fee).map_err(Error::ParseIntError)?),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: hex::decode(&stake.signature)
                .map_err(Error::Hex)?
                .as_slice()
                .try_into()
                .map_err(Error::TryFromSliceError)?,
        })
    }
}
