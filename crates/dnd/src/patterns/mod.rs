//! High-level pattern components
//!
//! This module provides pre-built patterns for common drag-and-drop use cases.
//! These patterns build on top of the primitives to provide higher-level
//! abstractions with sensible defaults.
//!
//! ## Available Patterns
//!
//! ### Sortable Lists
//!
//! The [`sortable`] module provides components for reorderable lists:
//!
//! - [`SortableContext`](sortable::SortableContext) - Context provider for sortable operations
//! - [`SortableItem`](sortable::SortableItem) - Individual sortable item wrapper
//! - [`SortableGroup`](sortable::SortableGroup) - Multi-container sortable support
//! - [`DropIndicator`](sortable::DropIndicator) - Visual indicator for drop position
//!
//! ### Grouped Lists (Flat Data)
//!
//! The [`grouped`] module provides convenience helpers for grouped flat lists:
//!
//! - [`GroupedList`](grouped::GroupedList) - Apply reorder/merge/cleanup logic
//! - [`GroupedItem`](grouped::GroupedItem) - Trait for items that participate in grouping
//! - [`grouped_reorder`](grouped::grouped_reorder) - Reorder + cleanup for signal data
//! - [`grouped_merge`](grouped::grouped_merge) - Merge + cleanup for signal data
//! - [`grouped_reorder_default`](grouped::grouped_reorder_default) - Default reorder helper
//! - [`grouped_merge_default`](grouped::grouped_merge_default) - Default merge helper
//! - [`grouped_merge`](grouped::grouped_merge) - Default merge with UUID v7 IDs
//! - [`grouped_move`](grouped::grouped_move) - Cross-container move with group support
//! - [`partition_grouped_items`](grouped::partition_grouped_items) - Partition flat list into groups/standalone
//! - [`find_flat_insert_position`](grouped::find_flat_insert_position) - Group-aware flat index lookup
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::prelude::*;
//!
//! rsx! {
//!     DragContextProvider {
//!         SortableContext {
//!             id: DragId::new("list"),
//!             on_reorder: move |event: ReorderEvent| { /* handle reorder */ },
//!             for (i, item) in items.iter().enumerate() {
//!                 SortableItem { index: i, id: DragId::new(&item.id),
//!                     div { "{item.title}" }
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```

pub mod grouped;
pub mod sortable;
