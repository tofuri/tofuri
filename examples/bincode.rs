use bincode::Options;
use pea::{address, constants::DECIMAL_PRECISION, transaction::Transaction, util};
fn main() {
    let options = bincode::DefaultOptions::new().with_varint_encoding();
    let input = vec![transaction()];
    let serialized = options.serialize(&input).unwrap();
    println!("{}", serialized.len());
    println!("{:x?}", serialized);
    println!(
        "{:x?}",
        options
            .deserialize::<Vec<Transaction>>(&serialized)
            .unwrap()
    );
}
fn transaction() -> Transaction {
    let keypair = util::keygen();
    let mut transaction = Transaction::new(
        address::decode(
            "0xbd8685eb128064f3969078db51b4fa94ea7af71844f70bea1f2e86c36186675db9ff2b09",
        )
        .unwrap(),
        69 * DECIMAL_PRECISION,
        1337,
    );
    transaction.sign(&keypair);
    transaction
}
