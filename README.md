# Pea Core integration/staging tree

It is recommended that you read through [The Technical Paper](https://github.com/peacash/paper/blob/main/README.md).

## Installation

```bash
git clone https://github.com/peacash/peacash.git
cd peacash
```

### Dependencies

#### Rustup - The toolchain installer

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Arch-based systems

```bash
sudo pacman -Sy
sudo pacman -S git cmake clang
```

#### Debian-based systems

```bash
sudo apt update
sudo apt install git cmake clang libssl-dev pkg-config
```

## Contribute

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.
