use pea::{address, constants::DECIMAL_PRECISION, transaction::Transaction, util};
fn main() {
    let keypair = util::keygen();
    let mut transaction = Transaction::new(
        address::public::decode(
            "0xbd8685eb128064f3969078db51b4fa94ea7af71844f70bea1f2e86c36186675db9ff2b09",
        )
        .unwrap(),
        69 * DECIMAL_PRECISION,
        1337,
    );
    transaction.sign(&keypair);
    println!("{:?}", transaction);
}
