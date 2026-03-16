//! Focus trap helpers for overlay components.
//!
//! On wasm32, queries the DOM for focusable elements within a container
//! and cycles focus forward/backward. On non-wasm targets, these are
//! intentional no-ops — Dioxus native platforms handle focus naturally.

/// The CSS selector for interactive/focusable elements.
pub const FOCUSABLE_SELECTOR: &str = "button:not([disabled]), [href], input:not([disabled]), \
     select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/// Return the focusable HTML elements within a container, in DOM order.
///
/// On wasm32: queries `querySelectorAll` for interactive elements within the
/// container identified by `container_id`.
/// On non-wasm: returns `None` (focus trap is handled by Dioxus tab guards).
#[cfg(target_arch = "wasm32")]
pub fn get_focusable_elements_in_container(
    container_id: &str,
) -> Option<Vec<web_sys::HtmlElement>> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let document = window.document()?;
    let container = document.get_element_by_id(container_id)?;
    let node_list = container.query_selector_all(FOCUSABLE_SELECTOR).ok()?;
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
pub fn get_focusable_elements_in_container(_container_id: &str) -> Option<Vec<()>> {
    None
}

/// Cycle focus within a container element.
///
/// - `forward = true`: move focus to the next focusable element, wrapping from last to first.
/// - `forward = false`: move focus to the previous focusable element, wrapping from first to last.
///
/// On non-wasm targets this is a no-op.
#[cfg(target_arch = "wasm32")]
pub fn cycle_focus(container_id: &str, forward: bool) {
    let Some(focusables) = get_focusable_elements_in_container(container_id) else {
        return;
    };
    if focusables.is_empty() {
        return;
    }

    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());

    let current_index = active.and_then(|active_el| {
        use wasm_bindgen::JsCast;
        let active_html: &web_sys::HtmlElement = active_el.unchecked_ref();
        focusables
            .iter()
            .position(|el| std::ptr::eq(el as *const _, active_html as *const _))
    });

    // If we can't find the active element in our focusables, find by equality check
    let current_index = current_index.or_else(|| {
        active.and_then(|active_el| {
            focusables.iter().position(|el| {
                let el_ref: &web_sys::Element = el.as_ref();
                *el_ref == active_el
            })
        })
    });

    let next_index = match current_index {
        Some(idx) => {
            if forward {
                if idx + 1 >= focusables.len() { 0 } else { idx + 1 }
            } else if idx == 0 {
                focusables.len() - 1
            } else {
                idx - 1
            }
        }
        None => {
            // No current focus in container — focus first or last element
            if forward { 0 } else { focusables.len() - 1 }
        }
    };

    let _ = focusables[next_index].focus();
}

/// Stub for non-wasm targets.
#[cfg(not(target_arch = "wasm32"))]
pub fn cycle_focus(_container_id: &str, _forward: bool) {
    // Intentional no-op: Dioxus native focus management is handled by platform.
}
