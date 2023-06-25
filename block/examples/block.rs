use block::Block;
use key::Key;
use stake::Stake;
use transaction::Transaction;
fn main() {
    let key = Key::from_slice(&[0xcd; 32]).unwrap();
    let output_address = [1; 20];
    let amount = 1_000_000_000_000_000_000;
    let fee = 1_000_000_000_000_000;
    let timestamp = 0;
    let transaction = Transaction::sign(output_address, amount, fee, timestamp, &key).unwrap();
    let deposit = true;
    let stake = Stake::sign(deposit, amount, fee, timestamp, &key).unwrap();
    let previous_hash = [0; 32];
    let previous_beta = [0; 32];
    let transactions = vec![transaction];
    let stakes = vec![stake];
    let block = Block::sign(
        previous_hash,
        timestamp,
        transactions,
        stakes,
        &key,
        &previous_beta,
    )
    .unwrap();
    println!("{:#?}", block);
}
