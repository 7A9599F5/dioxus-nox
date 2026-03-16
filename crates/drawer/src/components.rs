//! Drawer compound components: Root, Overlay, Content.

use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

use crate::types::{DrawerContext, DrawerSide};

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Headless drawer root component.
///
/// Provides [`DrawerContext`] to children. Renders nothing when `open = false`.
/// Handles ESC key press, scroll lock, and background inert.
#[component]
pub fn Root(
    /// Drawer content (Overlay + Content components).
    children: Element,
    /// Whether the drawer is open (controlled).
    open: bool,
    /// Close handler.
    on_close: EventHandler<()>,
    /// Which edge the drawer slides from.
    #[props(default = DrawerSide::Right)]
    side: DrawerSide,
    /// Close on Escape key.
    #[props(default = true)]
    close_on_escape: bool,
    /// Close on overlay click.
    #[props(default = true)]
    close_on_overlay: bool,
    /// Trap focus within the drawer.
    #[props(default = true)]
    trap_focus: bool,
    /// Lock body scroll when open.
    #[props(default = true)]
    lock_scroll: bool,
) -> Element {
    let instance_id = use_hook(next_instance_id);
    let root_id = format!("nox-drawer-{instance_id}");
    let root_id_clone = root_id.clone();

    use_context_provider(|| DrawerContext {
        open,
        on_close,
        close_on_overlay,
        side,
        instance_id,
    });

    // Scroll lock effect.
    use_effect(move || {
        if open && lock_scroll {
            dioxus_nox_internal::lock_body_scroll();
        } else {
            dioxus_nox_internal::unlock_body_scroll();
        }
    });

    // Background inert effect.
    let root_id_for_inert = root_id.clone();
    use_effect(move || {
        dioxus_nox_internal::set_siblings_inert(&root_id_for_inert, open);
    });

    // Cleanup on unmount.
    let root_id_for_cleanup = root_id.clone();
    use_drop(move || {
        dioxus_nox_internal::unlock_body_scroll();
        dioxus_nox_internal::set_siblings_inert(&root_id_for_cleanup, false);
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
                if trap_focus && (evt.key() == Key::Tab) {
                    evt.prevent_default();
                    let forward = !evt.modifiers().shift();
                    dioxus_nox_internal::cycle_focus(&root_id, forward);
                }
            },
            {children}
        }
    }
}

/// Headless drawer overlay/backdrop.
///
/// Clicking triggers the drawer's close handler if `close_on_overlay` is enabled.
#[component]
pub fn Overlay(
    /// Optional children.
    children: Option<Element>,
) -> Element {
    let ctx: DrawerContext = use_context();

    rsx! {
        div {
            "data-drawer-overlay": "",
            aria_hidden: "true",
            onclick: move |_| {
                if ctx.close_on_overlay {
                    ctx.on_close.call(());
                }
            },
            {children}
        }
    }
}

/// Headless drawer content panel.
///
/// Emits `role="dialog"`, `aria-modal="true"`, `data-side`, and `data-state`.
/// Stops click propagation to prevent overlay close.
#[component]
pub fn Content(
    /// Drawer content.
    children: Element,
) -> Element {
    let ctx: DrawerContext = use_context();
    let label_id = format!("nox-drawer-label-{}", ctx.instance_id);
    let data_state = if ctx.open { "open" } else { "closed" };

    rsx! {
        div {
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: "{label_id}",
            "data-state": data_state,
            "data-side": ctx.side.as_str(),
            "data-drawer-content": "",
            tabindex: "-1",
            onclick: move |evt: MouseEvent| {
                evt.stop_propagation();
            },
            {children}
        }
    }
}
