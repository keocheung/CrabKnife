# RustKnife

RustKnife is a small desktop developer toolbox written in Rust with `egui`.
It currently implements the first tool: a Regex Tester inspired by DevToys, while
keeping the interface close to native egui styling.

## Features

- Regex pattern testing with Rust's `regex` crate.
- Match and capture group inspection.
- Highlighted matches and capture groups in the test text editor.
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

## Notes

The app is English-only for now. Future tools should be added as separate state
and UI sections under the existing egui application structure.
