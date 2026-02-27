# dioxus-nox-virtualize

Virtual list viewport math for Dioxus. Used by `dioxus-nox-cmdk` via the `"virtualize"` feature.
See workspace `CLAUDE.md` for shared conventions.

## Crate Purpose

Provides pure Rust viewport/offset math for virtual scrolling. Zero Dioxus runtime dependency —
calculations are plain Rust functions that cmdk wires into its signal graph.

## Public API Surface

Pure math functions for virtual list rendering (item offset computation, visible range calculation).
No components, no signals, no web-sys calls.

## CI Commands

```bash
cargo test -p dioxus-nox-virtualize
cargo clippy -p dioxus-nox-virtualize -- -D warnings
```
