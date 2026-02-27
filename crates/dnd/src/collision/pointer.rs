//! Pointer-based collision detection
//!
//! Detects collisions based on the current pointer position.
//! Simple containment check - is the pointer inside a zone's bounding rect?

use std::collections::HashMap;

use crate::context::DropZoneState;
use crate::types::{DragData, DragId, DropLocation, Position};

/// Detect collision using simple pointer containment
///
/// Checks if the pointer position is contained within any registered
/// drop zone's bounding rectangle.
pub(crate) fn detect_pointer(
    pointer: Position,
    _dragged: &DragData,
    zones: &HashMap<DragId, DropZoneState>,
) -> Option<DropLocation> {
    for (id, zone) in zones {
        if zone.rect.contains(pointer) {
            return Some(DropLocation::IntoContainer {
                container_id: id.clone(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;

    #[test]
    fn test_pointer_collision_detection() {
        let mut zones = HashMap::new();

        let zone = DropZoneState::new(
            "zone1",
            "container1",
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("zone1"), zone);

        let dragged = DragData::new("item1", "task");

        // Pointer inside zone
        let result = detect_pointer(Position::new(50.0, 50.0), &dragged, &zones);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoContainer {
                container_id: DragId::new("zone1")
            }
        );

        // Pointer outside zone
        let result = detect_pointer(Position::new(150.0, 150.0), &dragged, &zones);
        assert!(result.is_none());
    }
}
