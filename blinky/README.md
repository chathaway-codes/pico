# Blinky

A game of memorization.

## Setup

Make sure Rust is up-to-date, and install the `thumbv6m-none-eabi` target

```bash
rustup self update
rustup update stable
rustup target add thumbv6m-none-eabi
```

Install elf2uf2-rs, which convert the elf output to uf2.

```bash
cargo install elf2uf2-rs --locked
```

To deploy to your pico, hold the BOOTSEL button and plug it into a USB port. Mount it as a thumb drive, and run:

```
cargo run --release
```