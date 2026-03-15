//! # dioxus-nox-gestures
//!
//! Touch gesture primitives (swipe-left reveal, long-press) for Dioxus WASM apps.
//!
//! ## Layers
//!
//! | Layer | Items | Dioxus dep? |
//! |-------|-------|-------------|
//! | **Math** | [`distance`], [`gesture_angle_degrees`], [`is_horizontal_gesture`], [`velocity`], [`next_swipe_phase`] | No |
//! | **Hooks** | [`use_swipe_gesture`], [`use_long_press`] | Yes |
//! | **Components** | [`swipe_actions::Root`], [`swipe_actions::Content`], [`swipe_actions::Actions`] | Yes |
//!
//! Use the math layer directly for custom gesture detection, or reach for the
//! hooks and compound components for ready-made swipe-to-reveal and long-press.

mod math;
mod types;
mod swipe;
mod long_press;
mod components;

#[cfg(test)]
mod tests;

// Pure math — no Dioxus dependency
pub use math::{
    distance, gesture_angle_degrees, is_horizontal_gesture, next_swipe_phase, velocity,
    SwipeDecision,
};

// Types
pub use types::{
    LongPressHandle, LongPressPhase, SwipeConfig, SwipeHandle, SwipePhase,
};

// Hooks
pub use long_press::use_long_press;
pub use swipe::use_swipe_gesture;

/// Compound components for swipe-to-reveal actions.
///
/// Wrap your content in [`Root`](swipe_actions::Root) with
/// [`Content`](swipe_actions::Content) and [`Actions`](swipe_actions::Actions)
/// children. The root wires pointer events and provides swipe state via context.
pub mod swipe_actions {
    pub use super::components::{Actions, Content, Root};
}
