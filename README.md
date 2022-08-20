# Axiom

## Specifications

| Name | Value |
| :- | :-: |
| Forge reward | `100_000_000` |
| Decimal precision | `1e-8` |
| Network stack | `libp2p` |
| Toolchain | `Nightly Rust` |

| Name | Min value | Max value |
| :- | :-: | :-: |
| Block time | `10s` | `20s` |
| Stake amount | `100_000_000` | `3_200_000_000` |

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
