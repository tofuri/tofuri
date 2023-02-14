use pea_address::address;
use pea_api_core::Block;
use pea_api_core::Stake;
use pea_api_core::Transaction;
use pea_block::BlockA;
use pea_stake::StakeA;
use pea_transaction::TransactionA;
pub fn external_block(block_a: &BlockA) -> Block {
    Block {
        hash: hex::encode(block_a.hash),
        previous_hash: hex::encode(block_a.previous_hash),
        timestamp: block_a.timestamp,
        beta: hex::encode(block_a.beta),
        pi: hex::encode(block_a.pi),
        forger_address: address::encode(&block_a.input_address()),
        signature: hex::encode(block_a.signature),
        transactions: block_a.transactions.iter().map(|x| hex::encode(x.hash)).collect(),
        stakes: block_a.stakes.iter().map(|x| hex::encode(x.hash)).collect(),
    }
}
pub fn external_transaction(transaction_a: &TransactionA) -> Transaction {
    Transaction {
        input_address: address::encode(&transaction_a.input_address),
        output_address: address::encode(&transaction_a.output_address),
        amount: pea_int::to_string(transaction_a.amount),
        fee: pea_int::to_string(transaction_a.fee),
        timestamp: transaction_a.timestamp,
        hash: hex::encode(transaction_a.hash),
        signature: hex::encode(transaction_a.signature),
    }
}
pub fn external_stake(stake_a: &StakeA) -> Stake {
    Stake {
        amount: pea_int::to_string(stake_a.amount),
        fee: pea_int::to_string(stake_a.fee),
        deposit: stake_a.deposit,
        timestamp: stake_a.timestamp,
        signature: hex::encode(stake_a.signature),
        input_address: address::encode(&stake_a.input_address),
        hash: hex::encode(stake_a.hash),
    }
}
