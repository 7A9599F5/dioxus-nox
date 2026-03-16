//! Core types for dioxus-nox-cycle.

use dioxus::prelude::*;

/// Cycle state returned by [`use_cycle`](crate::use_cycle).
#[derive(Clone)]
pub struct CycleState<T: Clone> {
    /// Current value.
    pub current: Signal<T>,
    /// Current index in the items list.
    pub index: Signal<usize>,
    /// Advance to next value (wraps around).
    pub next: Callback<()>,
    /// Go to previous value (wraps around).
    pub previous: Callback<()>,
    /// Jump to specific index (clamped to valid range).
    pub set_index: Callback<usize>,
}
