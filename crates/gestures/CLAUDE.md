# dioxus-nox-gestures — Touch gesture primitives (swipe, long-press)

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Provides `use_swipe_gesture` and `use_long_press` hooks for Dioxus WASM apps. Standalone — zero dependency on dioxus-cmdk. Hot-path state via `Rc<RefCell<GestureState>>`; reactive signals update only at gesture phase boundaries.

## Module Structure
- `lib.rs` — re-exports only
- `config.rs` — `SwipeConfig`
- `swipe.rs` — `GestureState`, `SwipePhase`, `SwipeHandle`, `use_swipe_gesture`, pure math functions
- `long_press.rs` — `LongPressHandle`, `use_long_press`
- `tests.rs` — pure unit tests (angle, state machine, jitter logic)

## Key Design Decisions
1. `Rc<RefCell<GestureState>>` for hot-path (every pointermove); reactive signals only at phase boundaries
2. Inline `translateX` style is `FUNCTIONAL` (runtime-computed) — always keep
3. Platform features: `web`/`desktop`/`mobile` feature flags for platform selection

## Further Reading
Detailed context in `.context/` — read on demand:
- `api.md` — full API surface, SwipeConfig fields, data attributes, phase state machine

## CI
```bash
cargo check -p dioxus-nox-gestures
cargo test -p dioxus-nox-gestures
cargo clippy -p dioxus-nox-gestures --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-gestures --features desktop --no-default-features
cargo check -p dioxus-nox-gestures --features mobile --no-default-features
```
