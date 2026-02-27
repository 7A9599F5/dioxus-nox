//! Sortable list pattern
//!
//! Provides components for creating reorderable lists with drag-and-drop.
//! Supports both single-list reordering and multi-list item movement.
//!
//! ## Features
//!
//! - Vertical and horizontal list orientation
//! - Visual drop indicators
//! - Smooth animations with CSS transitions
//! - Multi-container support via [`SortableGroup`]
//!
//! ## Architecture
//!
//! The sortable system uses a layered architecture:
//!
//! - **`SortableContext`**: The core building block for sortable lists. When
//!   standalone, creates its own `DragContextProvider`. When inside a group,
//!   uses `DropZone` to register with the group's provider.
//!
//! - **`SortableGroup`**: Creates a shared `DragContextProvider` for multiple
//!   containers, enabling cross-container drag-and-drop. Also provides shared
//!   `on_reorder` and `on_move` handlers via context.
//!
//! ## Single List Example
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::{DragId, ReorderEvent, SortableContext, SortableItem};
//!
//! #[component]
//! fn TaskList(items: Vec<Task>) -> Element {
//!     let mut items = use_signal(|| items);
//!     let item_ids: Vec<DragId> = items.read()
//!         .iter()
//!         .map(|t| DragId::new(&t.id))
//!         .collect();
//!
//!     rsx! {
//!         SortableContext {
//!             id: DragId::new("tasks"),
//!             items: item_ids,
//!             on_reorder: move |event: ReorderEvent| {
//!                 let mut list = items.write();
//!                 let item = list.remove(event.from_index);
//!                 list.insert(event.to_index, item);
//!             },
//!             for task in items.read().iter() {
//!                 SortableItem {
//!                     key: "{task.id}",
//!                     id: DragId::new(&task.id),
//!                     div { class: "task-card", "{task.title}" }
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Multi-Container Example (Kanban-style)
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::{DragId, DragOverlay, MoveEvent, ReorderEvent, SortableContext, SortableGroup};
//!
//! #[component]
//! fn KanbanBoard() -> Element {
//!     rsx! {
//!         SortableGroup {
//!             on_reorder: move |e: ReorderEvent| {
//!                 // Handle same-container reorders
//!                 // Note: Use e.item_id to find the item, not e.from_index
//!             },
//!             on_move: move |e: MoveEvent| {
//!                 // Handle cross-container moves
//!             },
//!
//!             // Handlers inherited from group - no on_reorder needed!
//!             SortableContext { id: DragId::new("todo"), items: todo_ids }
//!             SortableContext { id: DragId::new("in-progress"), items: doing_ids }
//!             SortableContext { id: DragId::new("done"), items: done_ids }
//!
//!             DragOverlay {
//!                 // Overlay content - must be inside SortableGroup
//!             }
//!         }
//!     }
//! }
//! ```

pub mod context;
pub mod group;
pub mod indicator;
pub mod item;

pub use context::SortableContext;
pub use group::{SortableGroup, SortableGroupContext};
pub use indicator::DropIndicator;
pub use item::{IndicatorPosition, SortableItem, SortableItemProps, SortableItemState};
