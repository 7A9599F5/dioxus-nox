# dioxus-nox-cmdk

Headless command palette library for Dioxus 0.7 (Rust WASM).
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Architecture

- Core library: `src/`
- All public API re-exported from `lib.rs` — update re-exports when adding items
- `types.rs` → data types, `hook.rs` → hooks/handles, `components.rs` → components, `context.rs` → CommandContext
- `scoring.rs`, `navigation.rs` → pure functions (testable without Dioxus runtime)
- Tests in `tests.rs` — all unit tests, no component rendering tests
- Examples in `examples/command_palette*/` (workspace members, each a separate crate)
- `ctx.is_open` is only set by `CommandDialog`/`CommandSheet`; standalone `CommandList` manages it via `use_hook`/`use_drop`; consumers detect close via `CommandRoot.on_close`
- `virtualize` feature enables virtual scrolling via `dioxus-nox-virtualize`

## Adding a New Feature Checklist

- New component: `components.rs` → `lib.rs` re-export → `tests.rs` → `README.md` table
- New hook: `hook.rs` → `lib.rs` re-export → `tests.rs` → `README.md` hooks section
- New type: `types.rs` → `lib.rs` re-export (if public) → `tests.rs`
- New web-sys API: add feature string to `Cargo.toml` `[target.wasm32]` web-sys features list

## Crate-Specific Conventions

- Self-registering pattern: `use_hook` on mount, `use_drop` on unmount, `use_effect` for prop sync
- Non-reactive hot-path state: `Rc<RefCell<T>>` (e.g., DragState in CommandSheet)
- Reactive derived state: `Memo<T>` with O(1) HashSet caching for visibility checks

## CI Commands

```bash
cargo test -p dioxus-nox-cmdk
cargo clippy -p dioxus-nox-cmdk -- -D warnings
cargo clippy -p dioxus-nox-cmdk --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-cmdk --features desktop --no-default-features
cargo check -p dioxus-nox-cmdk --features mobile --no-default-features
```
