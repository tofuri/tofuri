# Tofuri Core integration/staging tree

[![Rust](https://github.com/tofuri/tofuri/actions/workflows/rust.yml/badge.svg)](https://github.com/tofuri/tofuri/actions/workflows/rust.yml)

## Installation

```bash
git clone https://github.com/tofuri/tofuri
```

### Running Validator

```bash
cargo run
```

### Dependencies

* [Rust](https://rustup.rs)
* [LLVM](https://github.com/llvm/llvm-project/releases)
* [CMake](https://github.com/Kitware/CMake/releases)
* [Protobuf](https://github.com/protocolbuffers/protobuf/releases)

#### Debian-based Linux Distributions

```bash
apt install cmake clang protobuf-compiler libssl-dev pkg-config
```

#### Arch-based Linux Distributions

```bash
pacman -S cmake clang protobuf
```
