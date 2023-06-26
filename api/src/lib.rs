use address::public;
use decimal::Decimal;
use decimal::FromStr;
use hex;
use hex::FromHexError;
use serde::Deserialize;
use serde::Serialize;
use std::array::TryFromSliceError;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::num::ParseIntError;
use vint::Vint;
#[derive(Debug)]
pub enum Error {
    FromHexError(FromHexError),
    Address(address::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(TryFromSliceError),
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub cargo_pkg_name: String,
    pub cargo_pkg_version: String,
    pub cargo_pkg_repository: String,
    pub git_hash: String,
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
impl TryFrom<block::Block> for BlockHex {
    type Error = key::Error;
    fn try_from(block: block::Block) -> Result<Self, Self::Error> {
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
impl TryFrom<transaction::Transaction> for TransactionHex {
    type Error = key::Error;
    fn try_from(transaction: transaction::Transaction) -> Result<Self, Self::Error> {
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
impl TryFrom<stake::Stake> for StakeHex {
    type Error = key::Error;
    fn try_from(stake: stake::Stake) -> Result<Self, Self::Error> {
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
impl TryFrom<TransactionHex> for transaction::Transaction {
    type Error = Error;
    fn try_from(transaction: TransactionHex) -> Result<Self, Self::Error> {
        Ok(transaction::Transaction {
            output_address: public::decode(&transaction.output_address).map_err(Error::Address)?,
            amount: Vint::from(
                u128::from_str::<18>(&transaction.amount).map_err(Error::ParseIntError)?,
            ),
            fee: Vint::from(u128::from_str::<18>(&transaction.fee).map_err(Error::ParseIntError)?),
            timestamp: transaction.timestamp,
            signature: hex::decode(&transaction.signature)
                .map_err(Error::FromHexError)?
                .as_slice()
                .try_into()
                .map_err(Error::TryFromSliceError)?,
        })
    }
}
impl TryFrom<StakeHex> for stake::Stake {
    type Error = Error;
    fn try_from(stake: StakeHex) -> Result<Self, Self::Error> {
        Ok(stake::Stake {
            amount: Vint::from(u128::from_str::<18>(&stake.amount).map_err(Error::ParseIntError)?),
            fee: Vint::from(u128::from_str::<18>(&stake.fee).map_err(Error::ParseIntError)?),
            deposit: stake.deposit,
            timestamp: stake.timestamp,
            signature: hex::decode(&stake.signature)
                .map_err(Error::FromHexError)?
                .as_slice()
                .try_into()
                .map_err(Error::TryFromSliceError)?,
        })
    }
}
