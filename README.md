# Tofuri Core integration/staging tree

It is recommended that you read through [The Technical Paper](https://github.com/tofuri/paper/blob/main/README.md).

## Installation

```bash
git clone https://github.com/tofuri/tofuri.git
cd tofuri
cargo run --bin tofuri
```

### Dependencies

* [LLVM](https://github.com/llvm/llvm-project/releases)
* [CMake](https://github.com/Kitware/CMake/releases)
* [Protocol Buffers](https://github.com/protocolbuffers/protobuf/releases)

#### [Rustup.rs](https://rustup.rs/) - The Rust toolchain installer

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Arch-based Linux Distributions

```bash
sudo pacman -Sy
sudo pacman -S git cmake clang protobuf
```

#### Debian-based Linux Distributions

```bash
sudo apt update
sudo apt install git cmake clang libssl-dev pkg-config protobuf-compiler
```

### Configuration

Synchronize system clock.

```bash
timedatectl set-ntp true
```

Allow port `9333` in firewall.

```bash
sudo ufw allow 9333
```

## Contribute

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
