use dioxus::prelude::*;

use crate::PreviewPosition;

// ── Root ─────────────────────────────────────────────────────────────────────

/// Outer wrapper for a preview area.
///
/// Applies `data-preview-position` and `data-preview-loading` attributes so
/// consumers can target them with CSS selectors. Ships **zero visual styles**.
///
/// ```text
/// [data-preview-position="right"]   { /* position the pane */ }
/// [data-preview-loading="true"]     { /* show a spinner / skeleton */ }
/// [data-preview-loading="false"]    { /* hide the spinner */ }
/// ```
///
/// `data-preview-loading` is always present on the element (either `"true"` or
/// `"false"`), so both attribute selectors are reliable.
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
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
            "data-preview-position": pos,
            "data-preview-loading": if loading { "true" } else { "false" },
            ..attributes,
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
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-preview": "true",
            ..attributes,
            {children}
        }
    }
}
