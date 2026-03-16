//! Modal compound components: Root, Overlay, Content.

use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

use crate::types::ModalContext;

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Headless modal root component.
///
/// Provides [`ModalContext`] to children. Renders nothing when `open = false`.
/// Handles ESC key press and scroll lock.
#[component]
pub fn Root(
    /// Modal content (Overlay + Content components).
    children: Element,
    /// Whether the modal is open (controlled).
    open: bool,
    /// Close handler.
    on_close: EventHandler<()>,
    /// Close on Escape key press.
    #[props(default = true)]
    close_on_escape: bool,
    /// Close on backdrop/overlay click.
    #[props(default = true)]
    close_on_backdrop: bool,
    /// Trap focus within the modal.
    #[props(default = true)]
    trap_focus: bool,
    /// Lock body scroll when open.
    #[props(default = true)]
    lock_scroll: bool,
) -> Element {
    let instance_id = use_hook(next_instance_id);
    let root_id = format!("nox-modal-{instance_id}");
    let root_id_clone = root_id.clone();

    // Provide context to children.
    use_context_provider(|| ModalContext {
        open,
        on_close,
        close_on_backdrop,
        instance_id,
    });

    // Scroll lock effect.
    use_effect(move || {
        if open && lock_scroll {
            dioxus_nox_core::lock_body_scroll();
        } else {
            dioxus_nox_core::unlock_body_scroll();
        }
    });

    // Background inert effect.
    let root_id_for_inert = root_id.clone();
    use_effect(move || {
        dioxus_nox_core::set_siblings_inert(&root_id_for_inert, open);
    });

    // Cleanup on unmount.
    let root_id_for_cleanup = root_id.clone();
    use_drop(move || {
        dioxus_nox_core::unlock_body_scroll();
        dioxus_nox_core::set_siblings_inert(&root_id_for_cleanup, false);
    });

    if !open {
        return rsx! {};
    }

    let data_state = if open { "open" } else { "closed" };

    rsx! {
        div {
            id: "{root_id_clone}",
            "data-state": data_state,
            tabindex: "-1",
            onkeydown: move |evt: KeyboardEvent| {
                if close_on_escape && evt.key() == Key::Escape {
                    on_close.call(());
                }
                // Focus trap: intercept Tab key.
                if trap_focus && (evt.key() == Key::Tab) {
                    evt.prevent_default();
                    let forward = !evt.modifiers().shift();
                    dioxus_nox_core::cycle_focus(&root_id, forward);
                }
            },
            {children}
        }
    }
}

/// Headless modal overlay/backdrop.
///
/// Emits `aria-hidden="true"`. Clicking this element triggers the modal's
/// close handler if `close_on_backdrop` is enabled.
#[component]
pub fn Overlay(
    /// Optional children (rare — overlays are usually empty).
    children: Option<Element>,
) -> Element {
    let ctx: ModalContext = use_context();

    rsx! {
        div {
            "data-modal-overlay": "",
            aria_hidden: "true",
            onclick: move |_| {
                if ctx.close_on_backdrop {
                    ctx.on_close.call(());
                }
            },
            {children}
        }
    }
}

/// Headless modal content container.
///
/// Emits `role="dialog"`, `aria-modal="true"`, and `aria-labelledby`.
/// Stops click propagation to prevent overlay click-close when clicking content.
#[component]
pub fn Content(
    /// Modal content.
    children: Element,
) -> Element {
    let ctx: ModalContext = use_context();
    let label_id = format!("nox-modal-label-{}", ctx.instance_id);
    let data_state = if ctx.open { "open" } else { "closed" };

    rsx! {
        div {
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: "{label_id}",
            "data-state": data_state,
            "data-modal-content": "",
            tabindex: "-1",
            onclick: move |evt: MouseEvent| {
                evt.stop_propagation();
            },
            {children}
        }
    }
}
