# Pea

## Running

```bash
cargo run --bin pea-node --http-api=:::8080
cargo run --bin pea-wallet --http-api=:::8080
```

## Build

```bash
git clone https://github.com/peacash/pea
cd pea
rustup default nightly
cargo build
```

### Dependencies

#### Arch

```bash
sudo pacman -Sy
sudo pacman -S cmake clang
```

#### Debian

```bash
sudo apt update
sudo apt install cmake clang libssl-dev pkg-config
```

## Contribute

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
