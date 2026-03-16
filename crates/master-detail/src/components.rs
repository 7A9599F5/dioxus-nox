//! Master-detail compound components.

use dioxus::prelude::*;

use crate::types::MasterDetailContext;

/// Headless master-detail root component.
///
/// Provides [`MasterDetailContext`] to children. Emits `data-detail="open|closed"`.
#[component]
pub fn Root(
    /// Master and detail panel children.
    children: Element,
    /// Whether the detail panel is open.
    detail_open: bool,
    /// Called when detail panel should close (e.g., backdrop click).
    on_detail_close: EventHandler<()>,
) -> Element {
    use_context_provider(|| MasterDetailContext {
        detail_open,
        on_detail_close: on_detail_close.clone(),
    });

    let data_detail = if detail_open { "open" } else { "closed" };

    rsx! {
        div {
            "data-detail": data_detail,
            "data-master-detail": "",
            {children}
        }
    }
}

/// Headless master/list panel.
///
/// Always visible. Full width on mobile, flex child on desktop.
#[component]
pub fn Master(
    /// List content.
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "region",
            aria_label: "List",
            "data-master-panel": "",
            {children}
        }
    }
}

/// Headless detail panel.
///
/// Behavior varies by consumer CSS:
/// - Mobile: full overlay
/// - Tablet: side panel
/// - Desktop: inline sidebar
#[component]
pub fn Detail(
    /// Detail content.
    children: Element,
) -> Element {
    let ctx: MasterDetailContext = use_context();
    let data_detail = if ctx.detail_open { "open" } else { "closed" };

    rsx! {
        div {
            role: "region",
            aria_label: "Detail",
            aria_hidden: if !ctx.detail_open { "true" },
            "data-detail": data_detail,
            "data-detail-panel": "",
            {children}
        }
    }
}

/// Headless backdrop for mobile/tablet overlay.
///
/// Only rendered when detail is open. Clicking closes the detail panel.
#[component]
pub fn Backdrop() -> Element {
    let ctx: MasterDetailContext = use_context();

    if !ctx.detail_open {
        return rsx! {};
    }

    rsx! {
        div {
            aria_hidden: "true",
            "data-detail-backdrop": "",
            onclick: move |_| {
                ctx.on_detail_close.call(());
            },
        }
    }
}
