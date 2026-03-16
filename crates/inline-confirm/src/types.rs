//! Core types for dioxus-nox-inline-confirm.

use dioxus::prelude::*;

/// Confirmation state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmState {
    /// Default state — show the action trigger.
    #[default]
    Idle,
    /// Confirmation requested — show confirm/cancel.
    Confirming,
}

/// Handle returned by [`use_inline_confirm`](crate::use_inline_confirm).
#[derive(Clone)]
pub struct InlineConfirmHandle {
    /// Current state.
    pub state: Signal<ConfirmState>,
    /// Request confirmation (Idle → Confirming).
    pub request: Callback<()>,
    /// Confirm the action (Confirming → Idle).
    pub confirm: Callback<()>,
    /// Cancel (Confirming → Idle).
    pub cancel: Callback<()>,
}

/// Context shared between inline-confirm compound components.
#[derive(Clone)]
pub struct InlineConfirmContext {
    /// Current state.
    pub state: ConfirmState,
    /// Confirm handler.
    pub on_confirm: EventHandler<()>,
    /// Cancel handler.
    pub on_cancel: EventHandler<()>,
}
