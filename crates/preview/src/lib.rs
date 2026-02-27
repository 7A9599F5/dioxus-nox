//! # dioxus-nox-preview
//!
//! Debounced preview hook and LRU cache for navigable Dioxus lists.
//!
//! Prevents preview flicker during rapid arrow-key navigation by:
//! 1. Debouncing the active item ID via [`use_debounced_active`].
//! 2. Caching rendered preview closures in an LRU cache via [`use_preview_cache`].
//!
//! Standalone — zero dependency on `dioxus-nox-cmdk`.
//!
//! ## Quick start
//!
//! ```ignore
//! use dioxus_nox_preview::{use_debounced_active, use_preview_cache, PreviewPosition, preview};
//!
//! // In your component:
//! let active_id: ReadSignal<Option<String>> = /* … */;
//! let debounced = use_debounced_active(active_id, 120);
//! let cache = use_preview_cache(10);
//!
//! rsx! {
//!     preview::Root {
//!         position: PreviewPosition::Right,
//!         loading: debounced.read().is_none(),
//!         preview::Container {
//!             // Render from cache or fall back to loading state
//!         }
//!     }
//! }
//! ```

mod cache;
mod components;
mod debounce;
mod position;

#[cfg(test)]
mod tests;

pub use cache::{PreviewCacheHandle, use_preview_cache};
pub use debounce::use_debounced_active;
pub use position::PreviewPosition;

/// Thin compound components that apply `data-*` attributes.
///
/// Zero visual styles are shipped — all styling is left to the consumer.
pub mod preview {
    pub use super::components::{Container, Root};
}
