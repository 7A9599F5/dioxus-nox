//! # dx-dnd: Drag-and-Drop Component Library for Dioxus
//!
//! A composable, reactive drag-and-drop system for Dioxus.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use dioxus_nox_dnd::{DragId, DragType, DragContextProvider, Draggable, DropZone};
//!
//! fn app() -> Element {
//!     rsx! {
//!         DragContextProvider {
//!             Draggable { id: DragId::new("item-1"), drag_type: DragType::new("task"),
//!                 div { "Drag me!" }
//!             }
//!             DropZone { id: DragId::new("zone-1"),
//!                 div { "Drop here!" }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Import Patterns
//!
//! Prefer selective imports for clarity:
//! ```rust,ignore
//! use dioxus_nox_dnd::{DragId, SortableContext, SortableItem, ReorderEvent, FUNCTIONAL_STYLES};
//! use dioxus_nox_dnd::grouped::{GroupedItem, partition_grouped_items};
//! use dioxus_nox_dnd::styles::GROUPED_THEME_STYLES;
//! ```
//!
//! Or use the prelude for quick prototyping:
//! ```rust,ignore
//! use dioxus_nox_dnd::prelude::*;
//! ```
//!
//! ## Architecture
//!
//! The library is built in layers:
//!
//! - **Types Layer**: Core types like [`DragId`], [`DragType`], [`Position`], [`Rect`]
//! - **Context Layer**: [`DragContext`] for global state management
//! - **Collision Layer**: Pluggable collision detection via [`CollisionStrategy`]
//! - **Primitive Layer**: [`Draggable`], [`DropZone`], [`DragOverlay`] components
//! - **Pattern Layer**: Higher-level patterns like [`SortableContext`]
//!
//! ## Module Overview
//!
//! - [`types`]: Core types for drag-and-drop operations
//! - [`context`]: Global state management and provider component
//! - [`collision`]: Collision detection strategies
//! - [`primitives`]: Low-level drag-and-drop components
//! - [`patterns`]: High-level patterns (sortable lists)
//! - [`sortable`]: Short alias for sortable pattern components
//! - [`grouped`]: Short alias for grouped list helpers
//! - [`styles`]: CSS style constants

pub mod collision;
pub mod context;
pub mod patterns;
pub mod primitives;
mod sortable_projection;
pub mod types;
pub mod utils;

/// Short alias: `dioxus_nox_dnd::sortable::SortableContext` etc.
pub mod sortable {
    pub use crate::patterns::sortable::*;
}

/// Short alias: `dioxus_nox_dnd::grouped::GroupedItem` etc.
pub mod grouped {
    pub use crate::patterns::grouped::*;
}

/// CSS style constants: `dioxus_nox_dnd::styles::FUNCTIONAL_STYLES` etc.
pub mod styles {
    pub use crate::utils::{
        FEEDBACK_STYLES, FUNCTIONAL_STYLES, GROUPED_FEEDBACK_STYLES, GROUPED_FUNCTIONAL_STYLES,
    };
    #[cfg(feature = "styles")]
    pub use crate::utils::{GROUPED_THEME_STYLES, THEME_STYLES};
}

/// Prelude module — imports all public items.
///
/// For new code, prefer selective imports:
/// ```rust,ignore
/// use dioxus_nox_dnd::{DragId, SortableContext, SortableItem, ReorderEvent};
/// use dioxus_nox_dnd::grouped::{GroupedItem, partition_grouped_items};
/// use dioxus_nox_dnd::styles::GROUPED_THEME_STYLES;
/// ```
///
/// For quick prototyping, the prelude still works:
/// ```rust,ignore
/// use dioxus_nox_dnd::prelude::*;
/// ```
pub mod prelude {
    // Types
    pub use crate::types::{
        AnnouncementEvent, DragData, DragId, DragType, DropEvent, DropLocation, MergeEvent,
        MoveEvent, Orientation, Position, Rect, ReorderEvent,
    };

    // Context
    pub use crate::context::{
        ActiveDrag, DragContext, DragContextProvider, DragState, DropZoneState,
    };

    // Collision
    pub use crate::collision::CollisionStrategy;

    // Primitives
    pub use crate::primitives::{
        DragOverlay, DragOverlayRenderProps, Draggable, DraggableRenderProps, DropZone,
        DropZoneRenderProps,
    };

    // Patterns - Sortable
    pub use crate::patterns::sortable::{
        DropIndicator, IndicatorPosition, SortableContext, SortableGroup, SortableItem,
        SortableItemState,
    };

    // Patterns - Grouped (flat lists)
    pub use crate::patterns::grouped::{
        active_group_header, default_group_id, find_flat_insert_position, group_id_from_container,
        grouped_merge, grouped_merge_default, grouped_merge_with, grouped_move,
        grouped_move_default, grouped_position, grouped_reorder, grouped_reorder_default,
        grouped_style_info, partition_grouped_items, ActiveGroupHeader, GroupedItem, GroupedList,
        GroupedPosition, GroupedStyleInfo, TopLevelEntry, CONTAINER_SUFFIX,
        DEFAULT_MIN_GROUP_MEMBERS,
    };

    // Utils — CSS
    pub use crate::utils::{
        find_contiguous_block, FEEDBACK_STYLES, FUNCTIONAL_STYLES, GROUPED_FEEDBACK_STYLES,
        GROUPED_FUNCTIONAL_STYLES,
    };
    #[cfg(feature = "styles")]
    pub use crate::utils::{GROUPED_THEME_STYLES, THEME_STYLES};
}

// Selective root re-exports — the most common items at crate root
// Core identity types
pub use types::{DragId, DragType};

// Core event types
pub use types::{MergeEvent, MoveEvent, ReorderEvent};

// Core components
pub use context::{ActiveDrag, DragContext, DragContextProvider};
pub use primitives::{DragOverlay, Draggable, DropZone};

// Sortable pattern components
pub use patterns::sortable::{DropIndicator, SortableContext, SortableGroup, SortableItem};

// Collision strategy
pub use collision::CollisionStrategy;

// CSS styles (most common)
#[cfg(feature = "styles")]
pub use utils::THEME_STYLES;
pub use utils::{FEEDBACK_STYLES, FUNCTIONAL_STYLES};
