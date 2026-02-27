//! # dioxus-gestures
//!
//! Touch gesture primitives (swipe-left reveal, long-press) for Dioxus WASM apps.
//!
//! See SPEC.md for the full design specification.
//! Implementation: run the BUILD_PROMPT.md prompt in a fresh Claude Code session.
//!
//! ## Planned API
//! - `use_swipe_gesture(config: SwipeConfig) -> SwipeHandle`
//! - `use_long_press(duration_ms: u32, on_press: EventHandler<()>) -> LongPressHandle`
//! - Pure math: `gesture_angle_degrees`, `is_horizontal_gesture`, `next_swipe_phase`
