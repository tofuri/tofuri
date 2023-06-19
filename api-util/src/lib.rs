use std::num::ParseIntError;
use tofuri_address::address;
use tofuri_api_core::Block;
use tofuri_api_core::Stake;
use tofuri_api_core::Transaction;
use tofuri_block::BlockA;
use tofuri_transaction::TransactionA;
use tofuri_transaction::TransactionB;
use vint::Vint;
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    Address(tofuri_address::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(core::array::TryFromSliceError),
}
pub fn block(block_a: &BlockA) -> Block {
    Block {
        hash: hex::encode(block_a.hash),
        previous_hash: hex::encode(block_a.previous_hash),
        timestamp: block_a.timestamp,
        beta: hex::encode(block_a.beta),
        pi: hex::encode(block_a.pi),
        forger_address: address::encode(&block_a.input_address()),
        signature: hex::encode(block_a.signature),
        transactions: block_a
            .transactions
            .iter()
            .map(|x| hex::encode(x.hash))
            .collect(),
        stakes: block_a
            .stakes
            .iter()
            .map(|x| hex::encode(x.hash()))
            .collect(),
    }
}
pub fn transaction(transaction_a: &TransactionA) -> Transaction {
    Transaction {
        input_address: address::encode(&transaction_a.input_address),
        output_address: address::encode(&transaction_a.output_address),
        amount: parseint::to_string::<18>(transaction_a.amount),
        fee: parseint::to_string::<18>(transaction_a.fee),
        timestamp: transaction_a.timestamp,
        hash: hex::encode(transaction_a.hash),
        signature: hex::encode(transaction_a.signature),
    }
}
pub fn stake(stake: &tofuri_stake::Stake) -> Result<Stake, tofuri_stake::Error> {
    Ok(Stake {
        amount: parseint::to_string::<18>(stake.amount.into()),
        fee: parseint::to_string::<18>(stake.fee.into()),
        deposit: stake.deposit,
        timestamp: stake.timestamp,
        signature: hex::encode(stake.signature),
        input_address: address::encode(&stake.input_address()?),
        hash: hex::encode(stake.hash()),
    })
}
pub fn transaction_b(transaction: &Transaction) -> Result<TransactionB, Error> {
    let transaction_b = TransactionB {
        output_address: address::decode(&transaction.output_address).map_err(Error::Address)?,
        amount: Vint::from(
            parseint::from_str::<18>(&transaction.amount).map_err(Error::ParseIntError)?,
        ),
        fee: Vint::from(parseint::from_str::<18>(&transaction.fee).map_err(Error::ParseIntError)?),
        timestamp: transaction.timestamp,
        signature: hex::decode(&transaction.signature)
            .map_err(Error::Hex)?
            .as_slice()
            .try_into()
            .map_err(Error::TryFromSliceError)?,
    };
    Ok(transaction_b)
}
pub fn stake_b(stake: &Stake) -> Result<tofuri_stake::Stake, Error> {
    let stake = tofuri_stake::Stake {
        amount: Vint::from(parseint::from_str::<18>(&stake.amount).map_err(Error::ParseIntError)?),
        fee: Vint::from(parseint::from_str::<18>(&stake.fee).map_err(Error::ParseIntError)?),
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
