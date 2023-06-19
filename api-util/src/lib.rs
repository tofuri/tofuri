use std::num::ParseIntError;
use tofuri_address::address;
use tofuri_block::Block;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use vint::Vint;
#[derive(Debug)]
pub enum Error {
    Hex(hex::FromHexError),
    Address(tofuri_address::Error),
    ParseIntError(ParseIntError),
    TryFromSliceError(core::array::TryFromSliceError),
}
pub fn block(block_b: &Block) -> Result<tofuri_api_core::Block, tofuri_block::Error> {
    Ok(tofuri_api_core::Block {
        hash: hex::encode(block_b.hash()),
        previous_hash: hex::encode(block_b.previous_hash),
        timestamp: block_b.timestamp,
        beta: hex::encode(block_b.beta()?),
        pi: hex::encode(block_b.pi),
        forger_address: address::encode(&block_b.input_address()?),
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
pub fn transaction(
    transaction_a: &Transaction,
) -> Result<tofuri_api_core::Transaction, tofuri_transaction::Error> {
    Ok(tofuri_api_core::Transaction {
        input_address: address::encode(&transaction_a.input_address()?),
        output_address: address::encode(&transaction_a.output_address),
        amount: parseint::to_string::<18>(transaction_a.amount.into()),
        fee: parseint::to_string::<18>(transaction_a.fee.into()),
        timestamp: transaction_a.timestamp,
        hash: hex::encode(transaction_a.hash()),
        signature: hex::encode(transaction_a.signature),
    })
}
pub fn stake(stake: &Stake) -> Result<tofuri_api_core::Stake, tofuri_stake::Error> {
    Ok(tofuri_api_core::Stake {
        amount: parseint::to_string::<18>(stake.amount.into()),
        fee: parseint::to_string::<18>(stake.fee.into()),
        deposit: stake.deposit,
        timestamp: stake.timestamp,
        signature: hex::encode(stake.signature),
        input_address: address::encode(&stake.input_address()?),
        hash: hex::encode(stake.hash()),
    })
}
pub fn transaction_b(transaction: &tofuri_api_core::Transaction) -> Result<Transaction, Error> {
    let transaction_b = Transaction {
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
pub fn stake_b(stake: &tofuri_api_core::Stake) -> Result<Stake, Error> {
    let stake = Stake {
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
