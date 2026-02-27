use dioxus::prelude::*;

use crate::PreviewPosition;

// ── Root ─────────────────────────────────────────────────────────────────────

/// Outer wrapper for a preview area.
///
/// Applies `data-preview-position` and `data-preview-loading` attributes so
/// consumers can target them with CSS selectors. Ships **zero visual styles**.
///
/// ```text
/// [data-preview-position="right"]  { /* position the pane */ }
/// [data-preview-loading="true"]    { /* show a spinner / skeleton */ }
/// ```
#[component]
pub fn Root(
    /// Extra CSS classes applied to the root element.
    #[props(default)]
    class: Option<String>,
    /// Positional hint written as `data-preview-position`.
    #[props(default)]
    position: PreviewPosition,
    /// When `true`, sets `data-preview-loading="true"` to signal that the
    /// debounce window is active and a new preview is about to appear.
    #[props(default)]
    loading: bool,
    children: Element,
) -> Element {
    let pos = position.as_data_attr();
    rsx! {
        div {
            class: class,
            "data-preview-position": pos,
            "data-preview-loading": if loading { Some("true") } else { None },
            {children}
        }
    }
}

// ── Container ────────────────────────────────────────────────────────────────

/// Inner pane that wraps the actual preview content.
///
/// Marks itself with `data-preview="true"` for CSS targeting.
/// Ships **zero visual styles**.
#[component]
pub fn Container(
    /// Extra CSS classes applied to the container element.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: class,
            "data-preview": "true",
            {children}
        }
    }
}
