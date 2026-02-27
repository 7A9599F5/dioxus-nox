# dioxus-nox-gestures

Touch gesture primitives (swipe-left reveal, long-press) for Dioxus WASM apps.
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Crate Purpose

Provides `use_swipe_gesture` and `use_long_press` hooks. Standalone — zero dependency on dioxus-cmdk.

## Public API Surface

- `use_swipe_gesture(config: SwipeConfig) -> SwipeHandle`
- `use_long_press(duration_ms: u32, on_press: EventHandler<()>) -> LongPressHandle`
- `gesture_angle_degrees(dx: f64, dy: f64) -> f64` — pure math
- `is_horizontal_gesture(dx: f64, dy: f64, tolerance_degrees: u32) -> bool` — pure math
- `next_swipe_phase(current: SwipePhase, delta_x: f64, threshold_px: u32, is_horizontal: bool) -> SwipePhase` — pure math
- Types: `SwipeConfig`, `SwipeHandle`, `SwipePhase`, `GestureState`, `LongPressHandle`

## Module Structure

- `lib.rs` — re-exports only
- `config.rs` — SwipeConfig
- `swipe.rs` — GestureState, SwipePhase, SwipeHandle, use_swipe_gesture, pure math functions
- `long_press.rs` — LongPressHandle, use_long_press
- `tests.rs` — pure unit tests (angle, state machine, jitter logic)

## Gesture-Specific Data Attributes

- `data-gesture-phase="idle"` / `"tracking"` / `"revealed"` / `"dismissed"`
- `data-swiped="true"` / `"false"`
- `data-pressing="true"` / `"false"`

## Crate-Specific Conventions

- `Rc<RefCell<GestureState>>` for hot-path state (non-reactive, updated on every pointermove)
- Reactive signals (`is_swiped`, `offset_px`, `phase`) update only at gesture phase boundaries
- Inline `translateX` style is `FUNCTIONAL` (runtime-computed) — keep

## CI Commands

```bash
cargo test
cargo clippy -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo check --features desktop --no-default-features
cargo check --features mobile --no-default-features
```
