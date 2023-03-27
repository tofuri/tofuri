use tofuri_key::Key;
use tofuri_stake::StakeA;
pub fn main() {
    let key = Key::from_slice(&[0xcd; 32]).unwrap();
    let deposit = true;
    let amount = 1_000_000_000_000_000_000;
    let fee = 1_000_000_000_000_000;
    let timestamp = 0;
    let stake_a = StakeA::sign(deposit, amount, fee, timestamp, &key).unwrap();
    println!("{stake_a:#?}");
    let stake_b = stake_a.b();
    println!("{stake_b:#?}");
}
