//! Body scroll lock for overlay components.
//!
//! When an overlay is open, the body element's `overflow` should be set to
//! `hidden` to prevent background scrolling. This module provides platform-aware
//! lock/unlock functions.

/// Lock body scrolling by setting `overflow: hidden` on the `<body>` element.
///
/// On non-wasm targets, uses Dioxus `document::eval` to execute the same
/// operation via JavaScript.
pub fn lock_body_scroll() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.style().set_property("overflow", "hidden");
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        dioxus::prelude::document::eval("document.body.style.overflow = 'hidden'");
    }
}

/// Unlock body scrolling by removing the `overflow` property from `<body>`.
///
/// On non-wasm targets, uses Dioxus `document::eval` to execute the same
/// operation via JavaScript.
pub fn unlock_body_scroll() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.style().remove_property("overflow");
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        dioxus::prelude::document::eval("document.body.style.overflow = ''");
    }
}
