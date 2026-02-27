//! Pure placement math for floating `CommandList` positioning.
//! No Dioxus, web-sys, or async deps — testable on any host platform.
use crate::types::Side;

/// Decide which side to open on given measured viewport space.
///
/// Auto-flip guard: only flips when *both* values are positive (> 0.0).
/// When `vp_height == 0.0` (non-wasm sentinel), `space_below` is negative,
/// the guard prevents flip, and `preferred` is returned unchanged.
pub fn compute_side(preferred: Side, space_above: f64, space_below: f64) -> Side {
    let can_flip = space_above > 0.0 && space_below > 0.0;
    if !can_flip {
        return preferred;
    }
    match preferred {
        Side::Bottom => {
            if space_below < space_above {
                Side::Top
            } else {
                Side::Bottom
            }
        }
        Side::Top => {
            if space_above < space_below {
                Side::Bottom
            } else {
                Side::Top
            }
        }
    }
}

/// Build the `position:fixed` inline style string.
///
/// - `Side::Bottom`: `position:fixed;top:{max_y + offset}px;left:{min_x}px;width:{width}px;`
/// - `Side::Top`:    `position:fixed;bottom:{vp_height - min_y + offset}px;left:{min_x}px;width:{width}px;`
///
/// Inline styles are permitted here because all values are runtime-computed
/// coordinates that cannot be expressed as CSS classes.
pub fn compute_float_style(
    side: Side,
    min_x: f64,
    min_y: f64,
    max_y: f64,
    width: f64,
    offset: f64,
    vp_height: f64,
) -> String {
    match side {
        Side::Bottom => format!(
            "position:fixed;top:{top}px;left:{left}px;width:{width}px;",
            top = max_y + offset,
            left = min_x,
        ),
        Side::Top => format!(
            "position:fixed;bottom:{bottom}px;left:{left}px;width:{width}px;",
            bottom = vp_height - min_y + offset,
            left = min_x,
        ),
    }
}
