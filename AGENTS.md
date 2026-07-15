# AGENTS.md

## Project Shape
- Single Rust binary crate (`Cargo.toml`, edition 2024); the real entrypoint is `src/main.rs`.
- `src/main.rs` owns app state, input handling, UI rendering, and config constants; `src/history.rs` owns zsh history loading/parsing; `src/search.rs` owns result ordering/search behavior.
- The app is a terminal UI built with `ratatui`, `crossterm`, and `tui-input`.
- `target/` is generated build output and ignored; do not inspect or edit it unless debugging build artifacts specifically.

## Commands
- Run locally with `cargo run`; this is the intended manual test path for now.
- Fast compile check: `cargo check`.
- Run tests: `cargo test`.
- Format before finishing Rust edits: `cargo fmt`.

## Current TUI Behavior To Preserve
- The app renders with `Viewport::Inline(...)` rather than as a full-screen alternate-screen UI.
- Preserve the existing input-mode state machine unless explicitly asked to change it: starts in `Editing`; `Esc` switches to `Normal`; `e` switches back to `Editing`; `q` exits from `Normal`.
- In `Editing`, pass normal key events through `tui_input` so cursor movement and text editing keep working.

## Current Product Direction
- MVP focus is zsh history search UI only; do not implement shell execution/insertion behavior for Enter or Tab yet unless explicitly requested.
- Zsh history should come from `$HISTFILE`, falling back to `~/.zsh_history`.
- Keep behavior toggles as constants near the top of the Rust code when adding history/search behavior, e.g. duplicate handling or metadata search.
- Multi-line history commands should display as compact previews ending with `...` when truncated.
