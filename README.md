# Tofuri Core integration/staging tree

[![Rust](https://github.com/tofuri/tofuri/actions/workflows/rust.yml/badge.svg)](https://github.com/tofuri/tofuri/actions/workflows/rust.yml)

## Installation

```bash
git clone https://github.com/tofuri/tofuri
```

### Running Validator

```bash
cargo run --bin tofuri
```

### Dependencies

* [Rust](https://rustup.rs)
* [LLVM](https://github.com/llvm/llvm-project/releases)
* [CMake](https://github.com/Kitware/CMake/releases)
* [Protobuf](https://github.com/protocolbuffers/protobuf/releases)

#### Debian-based Linux Distributions

```bash
apt install git cmake clang libssl-dev pkg-config protobuf-compiler
```
