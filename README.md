# Pea Core integration/staging tree

It is recommended that you read through [The Technical Paper](https://github.com/peacash/paper/blob/main/README.md).

## Installation

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
git clone https://github.com/peacash/peacash.git
```

## Install using [Cargo](https://doc.rust-lang.org/cargo/)

```bash
cargo install pea
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
