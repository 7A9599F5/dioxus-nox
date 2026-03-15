use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::use_hook;

pub(crate) static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// SSR-safe instance ID hook.
///
/// Returns a stable `u32` ID for the calling component instance. The ID is
/// allocated once on first render and returned unchanged on subsequent renders,
/// making it safe to use in both client-side and SSR contexts.
///
/// Internally delegates to `use_hook` so the allocation is stable across
/// re-renders (same component instance always gets the same ID).
///
/// This is the preferred way to obtain an instance ID in components.
/// The raw `next_instance_id()` function can be passed to `use_hook` directly
/// (as is done in `use_command_context`), but `use_instance_id()` is a
/// convenience wrapper for that pattern.
#[allow(dead_code)]
pub(crate) fn use_instance_id() -> u32 {
    use_hook(next_instance_id)
}

pub(crate) fn make_item_dom_id(instance_id: u32, item_id: &str) -> String {
    format!("cmdk-item-{instance_id}-{item_id}")
}

pub(crate) fn make_listbox_id(instance_id: u32) -> String {
    format!("cmdk-list-{instance_id}")
}

pub(crate) fn make_input_id(instance_id: u32) -> String {
    format!("cmdk-input-{instance_id}")
}

/// High-resolution timestamp in milliseconds via `performance.now()`.
/// Returns 0.0 if `window` or `performance` is unavailable.
#[cfg(target_arch = "wasm32")]
pub(crate) fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Stub for non-wasm targets. Returns 0.0.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) fn now_ms() -> f64 {
    0.0
}

/// Check if the user prefers reduced motion via the CSS media query.
#[cfg(target_arch = "wasm32")]
pub(crate) fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|mq| mq.matches())
}

/// Stub for non-wasm targets. Always returns `false`.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) fn prefers_reduced_motion() -> bool {
    false
}

/// Returns `window.innerHeight` in logical pixels.
///
/// # Platform support
/// - **wasm32**: `web_sys::Window::inner_height()` (uses the already-enabled `"Window"`
///   feature). Confirmed: no Dioxus 0.7 native viewport-height API exists as of 2026-02.
///   Source: dioxus-html MountedData API exposes only element rects, not viewport size.
/// - **non-wasm** (Desktop / Mobile): returns `0.0` sentinel — disables auto-flip in
///   `placement::compute_side`; `CommandList` always uses `preferred_side` on native targets.
#[cfg(target_arch = "wasm32")]
pub(crate) fn get_viewport_height() -> f64 {
    web_sys::window()
        .and_then(|w| w.inner_height().ok())
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

/// Stub for non-wasm targets. Returns `0.0` sentinel (disables auto-flip).
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) fn get_viewport_height() -> f64 {
    0.0
}

pub(crate) fn scroll_item_into_view(instance_id: u32, item_id: &str) {
    let dom_id = make_item_dom_id(instance_id, item_id);
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window()
            && let Some(document) = window.document()
            && let Some(el) = document.get_element_by_id(&dom_id)
        {
            let opts = web_sys::ScrollIntoViewOptions::new();
            opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
            el.scroll_into_view_with_scroll_into_view_options(&opts);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let js =
            format!(r#"document.getElementById("{dom_id}")?.scrollIntoView({{block:"nearest"}})"#);
        dioxus::prelude::document::eval(&js);
    }
}

// ---------------------------------------------------------------------------
// P-004: Background `inert` on palette open
// ---------------------------------------------------------------------------

/// Mark (or unmark) all `<body>` children that do NOT contain the palette
/// root element as `inert`, preventing keyboard navigation and screen reader
/// access to the background while the palette is open.
///
/// # Platform support
/// - **wasm32**: Uses `web_sys` to walk `document.body.children` and set/remove
///   the `inert` attribute.
/// - **non-wasm** (desktop / mobile): intentional no-op — Dioxus native event scope
///   naturally bounds propagation to the component tree.
///
/// # Parameters
/// - `palette_root_id`: the `id` attribute of the palette's container element.
/// - `inert`: `true` to mark siblings as inert, `false` to remove the attribute.
pub(crate) fn set_siblings_inert(palette_root_id: &str, inert: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        set_siblings_inert_wasm(palette_root_id, inert);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Intentional no-op: Dioxus native event propagation is bounded by the
        // component tree; background inert is not needed on desktop/mobile.
        let _ = palette_root_id;
        let _ = inert;
    }
}

#[cfg(target_arch = "wasm32")]
fn set_siblings_inert_wasm(palette_root_id: &str, inert: bool) {
    // REVIEW(web_sys): no Dioxus 0.7 native equivalent for setting `inert` on DOM siblings.
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else { return };

    // Cast body to Element to access .children() (HtmlBodyElement doesn't have it directly)
    let body_el: &web_sys::Element = body.unchecked_ref();

    // Find the palette root element so we can skip its ancestor chain.
    let palette_el = document.get_element_by_id(palette_root_id);

    let children = body_el.children();
    let len = children.length();
    for i in 0..len {
        let Some(child) = children.item(i) else {
            continue;
        };

        // Skip the element that contains (or IS) the palette root.
        let skip = palette_el
            .as_ref()
            .is_some_and(|pe| child.contains(Some(pe)));
        if skip {
            continue;
        }

        if inert {
            let _ = child.set_attribute("inert", "");
        } else {
            let _ = child.remove_attribute("inert");
        }
    }
}

// ---------------------------------------------------------------------------
// P-022: Focus trap helpers
// ---------------------------------------------------------------------------

/// Return the focusable element DOM ids within a container, in DOM order.
///
/// On wasm32: queries `querySelectorAll` for interactive elements within the
/// container identified by `container_id`.
/// On non-wasm: returns `None` (focus trap is handled by Dioxus tab guards).
///
/// Focusable selector: `button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])`
#[cfg(target_arch = "wasm32")]
pub(crate) fn get_focusable_elements_in_container(
    container_id: &str,
) -> Option<Vec<web_sys::HtmlElement>> {
    // REVIEW(web_sys): MountedData refs may not capture dynamically-added focusable
    // children (e.g. close buttons rendered by users). querySelectorAll is the
    // reliable cross-element approach on wasm32.
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let document = window.document()?;
    let container = document.get_element_by_id(container_id)?;
    let selector = "button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), \
         textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";
    let node_list = container.query_selector_all(selector).ok()?;
    let len = node_list.length();
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        if let Some(node) = node_list.item(i)
            && let Ok(el) = node.dyn_into::<web_sys::HtmlElement>()
        {
            result.push(el);
        }
    }
    Some(result)
}

/// Stub for non-wasm: returns `None` (no-op, tab guards handle focus cycling on native).
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) fn get_focusable_elements_in_container(_container_id: &str) -> Option<Vec<()>> {
    // Intentional no-op: Dioxus native focus management is handled by platform.
    None
}
