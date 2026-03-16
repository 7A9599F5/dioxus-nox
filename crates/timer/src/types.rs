//! Core types for dioxus-nox-timer.

use dioxus::prelude::*;

/// Timer state machine.
///
/// Represents the lifecycle of a countdown timer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// No timer active.
    #[default]
    Idle,
    /// Counting down (or counting up for stopwatch).
    Running,
    /// Paused mid-count.
    Paused,
    /// Countdown reached zero.
    Complete,
}

/// Controls returned by [`use_countdown`](crate::use_countdown).
#[derive(Clone)]
pub struct CountdownControls {
    /// Start countdown with given duration in seconds.
    pub start: Callback<i64>,
    /// Pause a running countdown.
    pub pause: Callback<()>,
    /// Resume a paused countdown.
    pub resume: Callback<()>,
    /// Cancel immediately, return to Idle.
    pub skip: Callback<()>,
    /// Adjust remaining time by delta seconds (positive adds, negative subtracts).
    /// Remaining time is clamped to >= 0.
    pub adjust: Callback<i64>,
    /// Dismiss after completion (Complete → Idle).
    pub dismiss: Callback<()>,
}
