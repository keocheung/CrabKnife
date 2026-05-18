# CrabKnife

CrabKnife is a small desktop developer toolbox written in Rust with `egui`,
runs natively and on web.

## Features

- Regex pattern testing with Rust's `regex` crate.
- Match and capture group inspection.
- Highlighted matches and capture groups in the test text editor.
- Hex to string decoding.
- Base64 encoding and decoding.
- Hash generation and comparison for common algorithms.
- Radix conversion between signed decimal, hex, octal, and binary with byte order control.
- QR code generation with adjustable error correction and PNG export.
- Separate UI and editor font settings.
- System font selection through `fontdb`.

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
