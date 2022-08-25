# Axiom

## Specifications

| Name | Value |
| :- | :-: |
| Network stack | `libp2p` |
| Toolchain | `Nightly Rust` |
| Decimal precision | `1e-9` |
| Forge reward | *2 <sup>x / **Decimal precision** / 100</sup> - 1* |

| Name | Min value | Max value |
| :- | :-: | :-: |
| Block time | `10s` | `20s` |
| Stake amount | `1e9` | `1e11` |
| Forge reward | `6955550` | `1e9` |

## Validator

### Running a validator

```powershell
cargo run --bin validator
```

#### Validator options

```powershell
cargo run --bin validator -- --help
```

## Wallet

### Running the wallet

```powershell
cargo run --bin wallet
```

#### Wallet options

```powershell
cargo run --bin wallet -- --help
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
