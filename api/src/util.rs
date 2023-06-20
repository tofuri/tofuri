use decimal::Decimal;
use decimal::FromStr;
use std::num::ParseIntError;
use tofuri_address::public;
use tofuri_block::Block;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use vint::Vint;
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    Address(tofuri_address::public::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(core::array::TryFromSliceError),
}
pub fn block(block_b: &Block) -> Result<crate::Block, tofuri_key::Error> {
    Ok(crate::Block {
        hash: hex::encode(block_b.hash()),
        previous_hash: hex::encode(block_b.previous_hash),
        timestamp: block_b.timestamp,
        beta: hex::encode(block_b.beta()?),
        pi: hex::encode(block_b.pi),
        forger_address: public::encode(&block_b.input_address()?),
        signature: hex::encode(block_b.signature),
        transactions: block_b
            .transactions
            .iter()
            .map(|x| hex::encode(x.hash()))
            .collect(),
        stakes: block_b
            .stakes
            .iter()
            .map(|x| hex::encode(x.hash()))
            .collect(),
    })
}
pub fn transaction(transaction: &Transaction) -> Result<crate::Transaction, tofuri_key::Error> {
    Ok(crate::Transaction {
        input_address: public::encode(&transaction.input_address()?),
        output_address: public::encode(&transaction.output_address),
        amount: u128::from(transaction.amount).decimal::<18>(),
        fee: u128::from(transaction.fee).decimal::<18>(),
        timestamp: transaction.timestamp,
        hash: hex::encode(transaction.hash()),
        signature: hex::encode(transaction.signature),
    })
}
pub fn stake(stake: &Stake) -> Result<crate::Stake, tofuri_key::Error> {
    Ok(crate::Stake {
        amount: u128::from(stake.amount).decimal::<18>(),
        fee: u128::from(stake.fee).decimal::<18>(),
        deposit: stake.deposit,
        timestamp: stake.timestamp,
        signature: hex::encode(stake.signature),
        input_address: public::encode(&stake.input_address()?),
        hash: hex::encode(stake.hash()),
    })
}
pub fn transaction_b(transaction: &crate::Transaction) -> Result<Transaction, Error> {
    let transaction = Transaction {
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
    };
    Ok(transaction)
}
pub fn stake_b(stake: &crate::Stake) -> Result<Stake, Error> {
    let stake = Stake {
        amount: Vint::from(u128::from_str::<18>(&stake.amount).map_err(Error::ParseIntError)?),
        fee: Vint::from(u128::from_str::<18>(&stake.fee).map_err(Error::ParseIntError)?),
        deposit: stake.deposit,
        timestamp: stake.timestamp,
        signature: hex::decode(&stake.signature)
            .map_err(Error::Hex)?
            .as_slice()
            .try_into()
            .map_err(Error::TryFromSliceError)?,
    };
    Ok(stake)
}
