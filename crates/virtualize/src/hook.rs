//! Dioxus hook for virtual list with infinite-scroll support.
//!
//! Requires the `hooks` feature flag.

use crate::VirtualViewport;
use dioxus::prelude::*;

/// Return type for [`use_virtual_list`].
pub struct UseVirtualList {
    /// The current virtual viewport state (visible range, spacer heights, etc.).
    pub viewport: VirtualViewport,
    /// Current scroll position in pixels.
    pub scroll_top: u32,
}

/// Dioxus hook that wraps [`VirtualViewport`] with scroll tracking and
/// infinite-scroll support.
///
/// Tracks scroll position from the returned `onscroll` handler, recalculates
/// the viewport, and fires `on_end_reached` (with the next page number) when
/// the user scrolls near the end of the list. The callback is debounced per
/// page boundary so each page is requested at most once.
///
/// # Arguments
/// - `item_count` – total items currently loaded
/// - `item_height` – fixed height per item in px
/// - `viewport_height` – scroll container height in px
/// - `on_end_reached` – called with the next page number when near end
///
/// # Returns
/// `(UseVirtualList, EventHandler<Event<ScrollData>>)` – viewport state and an
/// `onscroll` handler to attach to the scrollable container.
pub fn use_virtual_list(
    item_count: usize,
    item_height: u32,
    viewport_height: u32,
    on_end_reached: EventHandler<usize>,
) -> (UseVirtualList, EventHandler<Event<ScrollData>>) {
    let mut scroll_top = use_signal(|| 0u32);
    let mut last_page_requested = use_signal(|| 0usize);

    let threshold = 5usize;
    let page_size = if viewport_height > 0 && item_height > 0 {
        (viewport_height / item_height) as usize
    } else {
        20
    };

    let viewport = VirtualViewport {
        item_count,
        item_height,
        viewport_height,
        scroll_top: *scroll_top.read(),
        overscan: 5,
    };

    // Check near-end and fire callback if new page boundary
    if viewport.is_near_end(threshold) && item_count > 0 {
        let current_page = if page_size > 0 {
            item_count / page_size
        } else {
            1
        };
        let next_page = current_page + 1;
        if next_page > *last_page_requested.read() {
            last_page_requested.set(next_page);
            on_end_reached.call(next_page);
        }
    }

    let onscroll = EventHandler::new(move |evt: Event<ScrollData>| {
        let pos = evt.scroll_top();
        scroll_top.set(pos.max(0.0) as u32);
    });

    let result = UseVirtualList {
        viewport,
        scroll_top: *scroll_top.read(),
    };

    (result, onscroll)
}
