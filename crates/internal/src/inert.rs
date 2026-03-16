//! Background inert management for overlay components.
//!
//! When an overlay (modal, drawer, command palette) is open, all sibling
//! elements of the overlay's root should be marked as `inert` to prevent
//! keyboard navigation and screen reader access to the background.

/// Mark (or unmark) all `<body>` children that do NOT contain the given
/// root element as `inert`.
///
/// # Platform support
/// - **wasm32**: Uses `web_sys` to walk `document.body.children` and set/remove
///   the `inert` attribute.
/// - **non-wasm** (desktop / mobile): intentional no-op — Dioxus native event scope
///   naturally bounds propagation to the component tree.
///
/// # Parameters
/// - `root_id`: the `id` attribute of the overlay's container element.
/// - `inert`: `true` to mark siblings as inert, `false` to remove the attribute.
pub fn set_siblings_inert(root_id: &str, inert: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        set_siblings_inert_wasm(root_id, inert);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = root_id;
        let _ = inert;
    }
}

#[cfg(target_arch = "wasm32")]
fn set_siblings_inert_wasm(root_id: &str, inert: bool) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else { return };

    let body_el: &web_sys::Element = body.unchecked_ref();
    let root_el = document.get_element_by_id(root_id);

    let children = body_el.children();
    let len = children.length();
    for i in 0..len {
        let Some(child) = children.item(i) else {
            continue;
        };

        // Skip the element that contains (or IS) the overlay root.
        let skip = root_el.as_ref().is_some_and(|re| child.contains(Some(re)));
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
