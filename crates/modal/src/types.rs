//! Core types for dioxus-nox-modal.

use dioxus::prelude::*;

/// Context shared between modal compound components via Dioxus context API.
#[derive(Clone)]
pub struct ModalContext {
    /// Whether the modal is currently open.
    pub open: bool,
    /// Close handler — called on ESC, backdrop click, etc.
    pub on_close: EventHandler<()>,
    /// Whether to close on backdrop click.
    pub close_on_backdrop: bool,
    /// Auto-generated unique ID for this modal instance.
    pub instance_id: u32,
}

/// Handle returned by [`use_modal`](crate::use_modal).
#[derive(Clone)]
pub struct ModalHandle {
    /// Whether the modal is open (reactive signal).
    pub open: Signal<bool>,
    /// Open the modal.
    pub show: Callback<()>,
    /// Close the modal.
    pub close: Callback<()>,
    /// Toggle the modal open/closed.
    pub toggle: Callback<()>,
}
