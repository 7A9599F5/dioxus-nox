//! Collision detection strategies
//!
//! This module provides collision detection algorithms for drag operations.
//! Different strategies are suitable for different UI patterns
//! (simple drop zones, sortable lists, etc.)
//!
//! The public API is the `CollisionStrategy` enum which implements `detect()`
//! directly. The individual detector modules contain the algorithm implementations.

mod closest;
mod pointer;
mod sortable;

use std::collections::HashMap;

use crate::context::DropZoneState;
use crate::types::{DragData, DragId, DropLocation, Position};

// ============================================================================
// CollisionStrategy Enum
// ============================================================================

/// Available collision detection strategies
///
/// Use this enum to select which collision detection algorithm to use.
/// The default is `Pointer` which is suitable for simple drop zones.
///
/// Call `detect()` directly on the strategy to run collision detection.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum CollisionStrategy {
    /// Simple pointer containment - is pointer inside zone rect?
    ///
    /// Best for: Simple drop zones, trash bins, panels
    /// Returns: `DropLocation::IntoContainer`
    #[default]
    Pointer,

    /// Closest center point - which zone's center is nearest to pointer?
    ///
    /// Best for: Grids, scattered items, when zones don't tile perfectly
    /// Returns: `DropLocation::IntoContainer`
    ClosestCenter,

    /// Sortable list detection - determines insertion index based on position
    ///
    /// Best for: Reorderable lists (vertical or horizontal)
    /// Returns: `DropLocation::AtIndex`
    Sortable,

    /// Sortable with merge zones (direction-aware split)
    ///
    /// Best for: Lists where items can be merged/grouped (e.g., workout supersets)
    /// Returns: `DropLocation::AtIndex` or `DropLocation::IntoItem`
    SortableWithMerge,
}

impl CollisionStrategy {
    /// Detect which drop location (if any) the pointer is currently over
    ///
    /// # Arguments
    /// * `pointer` - Current pointer position
    /// * `dragged` - Data about the item being dragged
    /// * `zones` - All registered drop zones and their state
    /// * `current_target` - The previous drop location (used for displacement-aware detection)
    /// * `gap_displacement` - Whether items displace to create gaps (affects zone splitting)
    /// * `delta` - Drag delta from start position (used for direction-aware zone splitting)
    ///
    /// # Returns
    /// The drop location if a valid target is detected, None otherwise
    pub fn detect(
        &self,
        pointer: Position,
        dragged: &DragData,
        zones: &HashMap<DragId, DropZoneState>,
        current_target: Option<&DropLocation>,
        gap_displacement: bool,
        delta: Position,
    ) -> Option<DropLocation> {
        match self {
            CollisionStrategy::Pointer => pointer::detect_pointer(pointer, dragged, zones),
            CollisionStrategy::ClosestCenter => {
                closest::detect_closest_center(pointer, dragged, zones)
            }
            CollisionStrategy::Sortable => sortable::detect_sortable(
                pointer,
                dragged,
                zones,
                current_target,
                false,
                gap_displacement,
                delta,
            ),
            CollisionStrategy::SortableWithMerge => sortable::detect_sortable(
                pointer,
                dragged,
                zones,
                current_target,
                true,
                gap_displacement,
                delta,
            ),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_strategy_default() {
        let strategy = CollisionStrategy::default();
        assert_eq!(strategy, CollisionStrategy::Pointer);
    }
}
