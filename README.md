# Pea Core integration/staging tree

It is recommended that you read through [The Technical Paper](https://github.com/peacash/paper/blob/main/README.md).

## Installation

```bash
git clone https://github.com/peacash/peacash.git
cd peacash
cargo run --bin pea
```

### Dependencies

* [LLVM](https://github.com/llvm/llvm-project/releases)
* [CMake](https://github.com/Kitware/CMake/releases)

#### [Rustup.rs](https://rustup.rs/) - The Rust toolchain installer

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Arch-based Linux Distributions

```bash
sudo pacman -Sy
sudo pacman -S git cmake clang
```

#### Debian-based Linux Distributions

```bash
sudo apt update
sudo apt install git cmake clang libssl-dev pkg-config
```

### Configuration

Synchronize system clock.

```bash
timedatectl set-ntp true
```

Allow port `9333` in firewall.

```
sudo ufw allow 9333
```

Allow port `9332` in firewall (`API`).

```
sudo ufw allow 9332
```

## Contribute

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
