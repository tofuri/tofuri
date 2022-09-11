# pea

## Usage

### Build Transaction

Filename: `examples/transaction.rs`

```rust
use pea::{address, constants::DECIMAL_PRECISION, transaction::Transaction, util};
fn main() {
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
    println!("{:?}", transaction);
}
```

### Build Stake

Filename: `examples/stake.rs`

```rust
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
```
