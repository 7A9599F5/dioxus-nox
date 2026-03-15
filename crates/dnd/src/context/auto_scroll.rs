//! Auto-scroll logic for viewport-edge proximity scrolling.
//!
//! When the pointer is near the top or bottom edge of the viewport during a
//! drag, auto-scroll kicks in to scroll the page. The velocity scales linearly
//! with distance into the edge zone.

/// Distance from viewport edge (px) where auto-scroll triggers.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const SCROLL_EDGE_PX: f64 = 60.0;

/// Maximum scroll speed per animation frame (px/frame).
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const SCROLL_MAX_PX: f64 = 15.0;

/// Pure computation of scroll velocity given pointer Y and viewport height.
///
/// Returns velocity in px/frame: negative = scroll up, positive = scroll down,
/// zero = no scroll. Velocity scales linearly with distance into the edge zone.
///
/// Called from the WASM `compute_scroll_velocity` and from unit tests.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn scroll_velocity_for(pointer_y: f64, viewport_height: f64) -> f64 {
    if pointer_y < SCROLL_EDGE_PX {
        // Near top — scroll up (negative)
        let distance = SCROLL_EDGE_PX - pointer_y;
        let ratio = (distance / SCROLL_EDGE_PX).min(1.0);
        -SCROLL_MAX_PX * ratio
    } else if pointer_y > viewport_height - SCROLL_EDGE_PX {
        // Near bottom — scroll down (positive)
        let distance = pointer_y - (viewport_height - SCROLL_EDGE_PX);
        let ratio = (distance / SCROLL_EDGE_PX).min(1.0);
        SCROLL_MAX_PX * ratio
    } else {
        0.0
    }
}

/// Compute viewport-edge auto-scroll velocity based on pointer Y position.
///
/// On WASM, reads `window.innerHeight` and delegates to [`scroll_velocity_for`].
#[cfg(target_arch = "wasm32")]
pub(super) fn compute_scroll_velocity(pointer_y: f64) -> f64 {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return 0.0,
    };
    let inner_height = match window.inner_height() {
        Ok(v) => v.as_f64().unwrap_or(0.0),
        Err(_) => return 0.0,
    };
    scroll_velocity_for(pointer_y, inner_height)
}

/// Non-wasm stub: always returns 0.0 (no auto-scroll outside browser).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn compute_scroll_velocity(_pointer_y: f64) -> f64 {
    0.0
}
