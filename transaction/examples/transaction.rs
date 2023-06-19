use tofuri_key::Key;
use tofuri_transaction::Transaction;
pub fn main() {
    let key = Key::from_slice(&[0xcd; 32]).unwrap();
    let amount = 1_000_000_000_000_000_000;
    let fee = 1_000_000_000_000_000;
    let timestamp = 0;
    let transaction = Transaction::sign([0x00; 20], amount, fee, timestamp, &key).unwrap();
    println!("{transaction:#?}");
}
