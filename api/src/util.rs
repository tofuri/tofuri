use pea_address::address;
use pea_api_core::external::Block;
use pea_block::BlockA;
pub fn external_block(block_a: BlockA) -> Block {
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
