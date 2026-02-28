# dioxus-nox-cmdk — Headless command palette primitive

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Headless command palette for Dioxus 0.7 (Rust WASM). All public API re-exported from `lib.rs`. `ctx.is_open` is set by `CommandDialog`/`CommandSheet`; standalone `CommandList` manages it via `use_hook`/`use_drop`. Optional virtual scrolling via `"virtualize"` feature.

## Module Structure
- `lib.rs` — all public API re-exports; update when adding items
- `types.rs` — data types
- `hook.rs` — hooks and handles
- `components.rs` — components
- `context.rs` — `CommandContext`
- `scoring.rs` — pure scoring functions (testable without Dioxus runtime)
- `navigation.rs` — pure navigation functions
- `tests.rs` — all unit tests (no component rendering tests)

## Key Design Decisions
1. Self-registering pattern: `use_hook` on mount, `use_drop` on unmount, `use_effect` for prop sync
2. Non-reactive hot-path state: `Rc<RefCell<T>>` (e.g., DragState in CommandSheet)
3. Reactive derived state: `Memo<T>` with O(1) HashSet caching for visibility checks

## Further Reading
Detailed context in `.context/` — read on demand:
- `architecture.md` — component tree, context/hook/types relationships, self-registering pattern
- `scoring.md` — nucleo-matcher integration, CommandScore, post-scoring
- `gotchas.md` — Rc<RefCell> hot-path patterns, virtualize feature integration

## CI
```bash
cargo check -p dioxus-nox-cmdk
cargo test -p dioxus-nox-cmdk
cargo clippy -p dioxus-nox-cmdk --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-cmdk --features desktop --no-default-features
cargo check -p dioxus-nox-cmdk --features mobile --no-default-features
```
