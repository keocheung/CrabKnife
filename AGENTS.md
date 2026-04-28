# AGENTS.md

## Project

This is a Rust desktop app using `eframe`/`egui`. The current app surface is a
developer toolbox with a Regex Tester and a Settings page.

## Commands

- Format: `cargo fmt`
- Check: `cargo check`
- Test: `cargo test`
- Run locally: `cargo run`

## Guidelines

- Keep the UI English-only.
- Prefer egui native widgets and styling.
- Keep tools modular in state and rendering code.
- Do not introduce large abstractions until there is a second real tool.
- Preserve separate UI font and editor font behavior.
- Use `regex` for regular expression behavior; do not hand-roll parsing.
- Run `cargo fmt --check`, `cargo check`, and `cargo test` before finishing.
