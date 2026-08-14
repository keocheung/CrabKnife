# CrabKnife

CrabKnife is a small, GPU-accelerated desktop developer toolbox written in Rust
with [`gpui`](https://gpui.rs) and `gpui-component`.

## Features

- Regex pattern testing with Rust's `regex` crate.
- Hex to string decoding.
- Base64 encoding and decoding.
- SHA-256 and MD5 hash generation.
- Decimal-to-hexadecimal, octal, and binary radix conversion.
- Floating-point bit inspection.
- Secure password generation.
- QR code validation and sizing.

## Run

```bash
cargo run
```

## Check

```bash
cargo fmt --check
cargo check
cargo test
```
