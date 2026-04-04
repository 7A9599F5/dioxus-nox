//! Compound component for virtual lists with variable-height items.
//!
//! Follows the Radix Primitives pattern: `Root`, `Viewport`, `Item`.
//!
//! ```text
//! virtual_list::Root {
//!     item_count: items.len(),
//!     estimate_item_height: 48,
//!     virtual_list::Viewport {
//!         for i in start..end {
//!             virtual_list::Item { index: i, key: "{i}",
//!                 p { "{items[i]}" }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! Use [`use_visible_range`] inside a `Viewport` to get the current
//! `[start, end)` range of item indices to render.

mod types;
mod root;
mod viewport_part;
mod item;

pub use types::VirtualListContext;
pub use root::Root;
pub use viewport_part::Viewport;
pub use item::Item;

use dioxus::prelude::*;

/// Convenience hook that returns the visible `[start, end)` range of item
/// indices from the nearest [`VirtualListContext`].
///
/// Must be called inside a `virtual_list::Root` / `virtual_list::Viewport`
/// subtree. Subscribes to scroll position and measurement changes so the
/// component re-renders when the visible range changes.
pub fn use_visible_range() -> (usize, usize) {
    let mut ctx = use_context::<VirtualListContext>();
    // Subscribe to scroll and measurement changes.
    let _ = (ctx.scroll_top)();
    let _ = (ctx.measure_gen)();
    ctx.viewport.write().visible_range()
}
