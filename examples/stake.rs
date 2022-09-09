use pea::{constants::DECIMAL_PRECISION, stake::Stake, util};
fn main() {
    let keypair = util::keygen();
    let mut stake = Stake::new(
        true, // false -> withdraw
        69 * DECIMAL_PRECISION,
        1337,
    );
    stake.sign(&keypair);
    println!("{:?}", stake);
}
