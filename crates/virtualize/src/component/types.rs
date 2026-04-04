//! Shared context type for the `virtual_list` compound component.

use dioxus::prelude::*;

use crate::VariableViewport;

/// Shared state for the `virtual_list` compound component tree.
///
/// Provided by [`super::Root`], consumed by [`super::Viewport`] and [`super::Item`].
/// All fields are `Signal` so the struct is `Copy`.
#[derive(Clone, Copy)]
pub struct VirtualListContext {
    /// The variable viewport state.
    pub(crate) viewport: Signal<VariableViewport>,
    /// Current scroll position in pixels (separate signal for fine-grained reactivity).
    pub(crate) scroll_top: Signal<u32>,
    /// Scroll container height in pixels.
    pub(crate) container_height: Signal<u32>,
    /// Generation counter — incremented after each measurement batch.
    pub(crate) measure_gen: Signal<u64>,
    /// Accumulated scroll correction delta (pixels).
    pub(crate) scroll_correction: Signal<i32>,
    /// Total item count (kept in sync with the viewport).
    pub(crate) item_count: Signal<usize>,
    /// Callback for infinite scroll (called with next page number).
    pub(crate) on_end_reached: Option<EventHandler<usize>>,
    /// How many items from the end to trigger `on_end_reached`.
    pub(crate) end_threshold: usize,
    /// Tracks the last page requested (prevents duplicate calls).
    pub(crate) last_page_requested: Signal<usize>,
    /// Estimated items per page (for page number calculation).
    pub(crate) page_size: usize,
}
