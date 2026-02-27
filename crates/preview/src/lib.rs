//! # dioxus-preview
//!
//! Debounced preview hook and LRU cache for navigable Dioxus lists.
//!
//! See SPEC.md for the full design specification.
//! Implementation: run the BUILD_PROMPT.md prompt in a fresh Claude Code session.
//!
//! ## Planned API
//! - `use_debounced_active(active_id: ReadOnlySignal<Option<String>>, debounce_ms: u32) -> ReadOnlySignal<Option<String>>`
//! - `use_preview_cache(capacity: usize) -> PreviewCacheHandle`
//! - `PreviewPosition` enum (None, Right, Bottom)
