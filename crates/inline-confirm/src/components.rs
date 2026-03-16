//! Inline confirm compound components.

use dioxus::prelude::*;

use crate::types::{ConfirmState, InlineConfirmContext};

/// Headless inline confirm root.
///
/// Provides context and emits `data-state="idle|confirming"`.
#[component]
pub fn Root(
    /// Children (Trigger + Action components).
    children: Element,
    /// Current state (controlled).
    state: ConfirmState,
    /// Handler when user confirms.
    on_confirm: EventHandler<()>,
    /// Handler when user cancels (or auto-cancel fires).
    on_cancel: EventHandler<()>,
) -> Element {
    use_context_provider(|| InlineConfirmContext {
        state,
        on_confirm: on_confirm.clone(),
        on_cancel: on_cancel.clone(),
    });

    let data_state = match state {
        ConfirmState::Idle => "idle",
        ConfirmState::Confirming => "confirming",
    };

    rsx! {
        div {
            "data-state": data_state,
            "data-inline-confirm": "",
            {children}
        }
    }
}

/// Slot shown when state is Idle.
///
/// Contains the initial action trigger (e.g., a "Delete" button).
#[component]
pub fn Trigger(
    /// Trigger content.
    children: Element,
) -> Element {
    let ctx: InlineConfirmContext = use_context();

    if ctx.state != ConfirmState::Idle {
        return rsx! {};
    }

    rsx! {
        div {
            "data-inline-confirm-trigger": "",
            {children}
        }
    }
}

/// Slot shown when state is Confirming.
///
/// Contains confirm/cancel buttons.
#[component]
pub fn Action(
    /// Confirmation action content.
    children: Element,
) -> Element {
    let ctx: InlineConfirmContext = use_context();

    if ctx.state != ConfirmState::Confirming {
        return rsx! {};
    }

    rsx! {
        div {
            "data-inline-confirm-action": "",
            {children}
        }
    }
}
