use pea::{
    address,
    constants::DECIMAL_PRECISION,
    transaction::{Input, Output, Transaction},
    util,
};
fn main() {
    let keypair = util::keygen();
    let address = address::decode(
        "0xbd8685eb128064f3969078db51b4fa94ea7af71844f70bea1f2e86c36186675db9ff2b09",
    );
    let transaction = Transaction::new(
        vec![Output::new(address, 69 * DECIMAL_PRECISION)],
        &[kepair],
    );
    println!("{:?}", transaction);
}
