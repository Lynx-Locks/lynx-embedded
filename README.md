# Lynx Embedded

Lynx Lock Embedded Software.

## Hardware

Board: [ESP32-C3-DevKitM-1](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/hw-reference/esp32c3/user-guide-devkitm-1.html)

### Board properties

* ESP32-C3FN4 RISC-V MCU
* Addressable RGB LED, driven by GPIO8
* Wi-Fi and Bluetooth combo module with PCB antenna

### External Sensors and Actuators

* PN532 NFC/RFID reader

## Config

To use Wi-Fi, create a file named `cfg.toml` in the project directory, then add your
Wi-Fi SSID and password to it in the same format as [cfg.example.toml](./cfg.example.toml).

## Usage: Docker (Linux/WSL)

This method only works in WSL if you have done the [additional setup](./WSL_README.md) for it.

### Run the main binary

Flash the program:

```bash
./esp-cargo espflash flash --release
```

Run and monitor the program:

```bash
./esp-cargo run
```

### Run an example

Flash the program:

```bash
./esp-cargo espflash flash --release --example led
```

Run and monitor the program:

```bash
./esp-cargo run --example led
```

## Usage: Local Environment

**IMPORTANT**: If you want to use WSL, you will need to do some [additional setup](./WSL_README.md).

### Prerequisites

Follow the std development requirements
in [The Rust on ESP Book](https://esp-rs.github.io/book/installation/index.html).

To summarize:

1. Install [rust](https://www.rust-lang.org/tools/install).

2. Install the [nightly](https://rust-lang.github.io/rustup/concepts/channels.html#working-with-nightly-rust)
   toolchain with the `rust-src` [component](https://rust-lang.github.io/rustup/concepts/components.html):
   ```bash
   rustup toolchain install nightly --component rust-src
   ```

3. Add the cross compilation target:
   ```bash
   rustup target add riscv32imc-unknown-none-elf
   ```

4. Install [LLVM](https://llvm.org/) compiler infrastructure,
   [python](https://www.python.org/downloads/) (with pip and venv),
   and [git](https://git-scm.com/downloads).

5. Install [ldproxy](https://github.com/esp-rs/embuild/tree/master/ldproxy) binary crate:
   ```bash
   cargo install ldproxy
   ```

6. Install [espflash](https://github.com/esp-rs/espflash):
   ```bash
   # Subcommand for cargo
   cargo install cargo-espflash
   # Standalone command version
   cargo install espflash
   ```

### Run the main binary

Flash the program:

```bash
cargo espflash flash --release
```

Run and monitor the program:

```bash
cargo run
```

### Run an example

Flash the program:

```bash
cargo espflash flash --release --example led
```

Run and monitor the program:

```bash
cargo run --example led
```

## Troubleshooting

- The build script for `esp-idf-sys` creates a directory named `.embuild`.
  This directory cannot be reused between systems (e.g. local system and Docker),
  so it must be manually removed before building on a different system.
