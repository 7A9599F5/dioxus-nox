//! Primitive drag-and-drop components
//!
//! This module contains the foundational building blocks for drag-and-drop
//! interactions. These components can be composed to build custom drag-and-drop
//! experiences or used directly for simple use cases.
//!
//! ## Components
//!
//! ### Draggable
//!
//! The [`Draggable`] component makes its children draggable:
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::prelude::*;
//!
//! rsx! {
//!     Draggable {
//!         id: DragId::new("item-1"),
//!         drag_type: DragType::new("task"),
//!         on_drag_start: move |_| { /* drag started */ },
//!         div { "Drag me!" }
//!     }
//! }
//! ```
//!
//! ### DropZone
//!
//! The [`DropZone`] component creates an area that can receive dropped items:
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::prelude::*;
//!
//! rsx! {
//!     DropZone {
//!         id: DragId::new("zone-1"),
//!         accepts: vec![DragType::new("task")],
//!         div { "Drop items here" }
//!     }
//! }
//! ```
//!
//! ### DragOverlay
//!
//! The [`DragOverlay`] component displays a visual representation of the
//! dragged item that follows the cursor:
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::prelude::*;
//!
//! rsx! {
//!     DragOverlay {
//!         div { "I follow the cursor during drag" }
//!     }
//! }
//! ```

pub mod draggable;
pub mod dropzone;
pub mod overlay;

pub use draggable::{Draggable, DraggableProps, DraggableRenderProps};
pub use dropzone::{DropZone, DropZoneRenderProps};
pub use overlay::{DragOverlay, DragOverlayRenderProps};
