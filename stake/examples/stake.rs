use key::Key;
use stake::Stake;
pub fn main() {
    let key = Key::from_slice(&[0xcd; 32]).unwrap();
    let deposit = true;
    let amount = 1_000_000_000_000_000_000;
    let fee = 1_000_000_000_000_000;
    let timestamp = 0;
    let stake = Stake::sign(deposit, amount, fee, timestamp, &key).unwrap();
    println!("{stake:#?}");
}
