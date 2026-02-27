//! Closest center collision detection
//!
//! Detects collisions based on proximity to element centers.
//! Finds the drop zone whose center is closest to the pointer position.

use std::collections::HashMap;

use crate::context::DropZoneState;
use crate::types::{DragData, DragId, DropLocation, Position};

/// Detect collision using closest center distance
///
/// Finds the drop zone whose center point is closest to the current
/// pointer position. Skips the item currently being dragged.
pub(crate) fn detect_closest_center(
    pointer: Position,
    dragged: &DragData,
    zones: &HashMap<DragId, DropZoneState>,
) -> Option<DropLocation> {
    let mut closest: Option<(&DragId, f64)> = None;

    for (id, zone) in zones {
        // Skip the item being dragged
        if id == &dragged.id {
            continue;
        }

        let center = zone.rect.center();
        let distance = pointer.distance_to(center);

        if closest.is_none() || distance < closest.unwrap().1 {
            closest = Some((id, distance));
        }
    }

    closest.map(|(id, _)| DropLocation::IntoContainer {
        container_id: id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;

    #[test]
    fn test_closest_center_detection() {
        let mut zones = HashMap::new();

        // Two zones side by side
        let zone1 = DropZoneState::new(
            "zone1",
            "container",
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![],
        );
        let zone2 = DropZoneState::new(
            "zone2",
            "container",
            Rect::new(200.0, 0.0, 100.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("zone1"), zone1);
        zones.insert(DragId::new("zone2"), zone2);

        let dragged = DragData::new("item1", "task");

        // Pointer closer to zone1's center (50, 50)
        let result = detect_closest_center(Position::new(40.0, 40.0), &dragged, &zones);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoContainer {
                container_id: DragId::new("zone1")
            }
        );

        // Pointer closer to zone2's center (250, 50)
        let result = detect_closest_center(Position::new(260.0, 40.0), &dragged, &zones);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoContainer {
                container_id: DragId::new("zone2")
            }
        );
    }

    #[test]
    fn test_closest_center_skips_dragged_item() {
        let mut zones = HashMap::new();

        let zone1 = DropZoneState::new(
            "item1",
            "container",
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![],
        );
        let zone2 = DropZoneState::new(
            "zone2",
            "container",
            Rect::new(200.0, 0.0, 100.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), zone1);
        zones.insert(DragId::new("zone2"), zone2);

        // Dragging item1 - should not detect item1 as target
        let dragged = DragData::new("item1", "task");

        let result = detect_closest_center(Position::new(40.0, 40.0), &dragged, &zones);
        assert!(result.is_some());
        // Should return zone2 since item1 is being dragged
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoContainer {
                container_id: DragId::new("zone2")
            }
        );
    }
}
