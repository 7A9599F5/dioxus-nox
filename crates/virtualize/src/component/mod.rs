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
/// subtree. The result is derived from a `Memo<LayoutSnapshot>`, so multiple
/// callers share the same cached computation with zero lock contention.
pub fn use_visible_range() -> (usize, usize) {
    let ctx = use_context::<VirtualListContext>();
    // layout is a Memo — .read() is a shared reference, no write lock.
    ctx.layout.read().visible_range()
}
