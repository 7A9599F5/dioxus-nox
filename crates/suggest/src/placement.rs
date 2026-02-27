/// Compute a CSS `position:fixed` style string for floating list positioning.
///
/// Places the list **below** the anchor element, left-aligned to its edge.
/// `anchor_bottom` is the Y coordinate of the anchor's bottom edge (from
/// `ClientRect::max_y()`).
///
/// No auto-flip in v0.1 — the list always opens downward.
/// `viewport_height` is reserved for future flip logic (v0.2).
///
/// # FUNCTIONAL inline style
///
/// All values are runtime-computed coordinates; they cannot be expressed as
/// static CSS classes. This is the sole exception to the zero-inline-styles rule.
#[allow(unused_variables)]
pub fn compute_float_style(
    anchor_left: f64,
    anchor_bottom: f64,
    anchor_width: f64,
    side_offset: f64,
    viewport_height: f64,
) -> String {
    format!(
        "position:fixed;top:{top}px;left:{left}px;min-width:{min_width}px;",
        top = anchor_bottom + side_offset,
        left = anchor_left,
        min_width = anchor_width,
    )
}
