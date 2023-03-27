use tofuri_block::BlockA;
use tofuri_key::Key;
use tofuri_stake::StakeA;
use tofuri_transaction::TransactionA;
fn main() {
    let key = Key::from_slice(&[0xcd; 32]).unwrap();
    let output_address = [1; 20];
    let amount = 1_000_000_000_000_000_000;
    let fee = 1_000_000_000_000_000;
    let timestamp = 0;
    let transaction_a = TransactionA::sign(output_address, amount, fee, timestamp, &key).unwrap();
    let deposit = true;
    let stake_a = StakeA::sign(deposit, amount, fee, timestamp, &key).unwrap();
    let previous_hash = [0; 32];
    let previous_beta = [0; 32];
    let transactions = vec![transaction_a];
    let stakes = vec![stake_a];
    let block_a = BlockA::sign(
        previous_hash,
        timestamp,
        transactions,
        stakes,
        &key,
        &previous_beta,
    )
    .unwrap();
    println!("{:#?}", block_a);
    let block_b = block_a.b();
    println!("{:#?}", block_b);
    let block_c = block_b.c();
    println!("{:#?}", block_c);
}
