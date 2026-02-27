//! Shared sortable projection helpers used by both collision detection and
//! UI displacement rendering.

/// Compute displacement offset for an item during drag.
///
/// This is the canonical sortable projection model. Collision detection uses
/// it to project effective hit-test rects, and sortable rendering uses it to
/// project visual displacement.
///
/// # Arguments
/// * `my_idx` - Index of the item being checked (filtered list index)
/// * `source_idx` - Index of the dragged item in this container (filtered)
/// * `target_idx` - Index where the item would drop in this container (filtered)
/// * `target_is_partial` - Whether the target is an IntoItem (merge) state
/// * `item_size` - Size of THIS item along the drag axis
/// * `dragged_size` - Size of the DRAGGED item along the drag axis
///
/// # Returns
/// The offset to apply to the item's position:
/// - negative = shift up/left
/// - positive = shift down/right
pub(crate) fn compute_displacement_offset(
    my_idx: usize,
    source_idx: Option<usize>,
    target_idx: Option<usize>,
    target_is_partial: bool,
    item_size: f64,
    dragged_size: f64,
) -> f64 {
    match (source_idx, target_idx) {
        // Reorder: source and target both in this container
        (Some(src), Some(tgt)) => {
            if target_is_partial {
                // IntoItem (merge): target squeezes 50%, items between
                // source and target get full displacement.
                if my_idx == tgt {
                    // Target squeezes toward source direction
                    if src < my_idx {
                        -item_size * 0.5
                    } else {
                        item_size * 0.5
                    }
                } else if src <= my_idx && my_idx < tgt {
                    -dragged_size // Shift up/left (same as Before/After)
                } else if tgt < my_idx && my_idx < src {
                    dragged_size // Shift down/right (same as Before/After)
                } else {
                    0.0
                }
            } else if src <= my_idx && my_idx < tgt {
                -dragged_size // Shift up/left to fill source gap
            } else if tgt <= my_idx && my_idx < src {
                dragged_size // Shift down/right to make room at target
            } else {
                0.0
            }
        }
        // Drag out: source here, target elsewhere/none
        (Some(src), None) => {
            if my_idx >= src {
                -dragged_size // Shift up/left to fill gap
            } else {
                0.0
            }
        }
        // Drag in: source elsewhere, target here
        (None, Some(tgt)) => {
            if target_is_partial {
                // Cross-container IntoItem: target squeezes down,
                // no other items need displacement (no source gap).
                if my_idx == tgt {
                    item_size * 0.5
                } else {
                    0.0
                }
            } else if my_idx >= tgt {
                dragged_size // Shift down/right to make room
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Convert a full-list index to the corresponding filtered-list index where
/// the source slot is removed.
pub(crate) fn to_filtered_index(index: usize, source_idx: usize) -> usize {
    if index > source_idx {
        index - 1
    } else {
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_list_to_filtered_projection_consistency() {
        let src_full = 1usize;
        let dragged_size = 80.0;
        let item_size = 80.0;

        // Simulate full-list indices [item0, dragged, item2, item3].
        // AtIndex target uses filtered-list index (drop before item3 => 2).
        let tgt_filtered = 2usize;

        // item2 (full index 2) should shift up by dragged size.
        let my_full = 2usize;
        let my_filtered = to_filtered_index(my_full, src_full);
        let shared_offset = compute_displacement_offset(
            my_filtered,
            Some(src_full),
            Some(tgt_filtered),
            false,
            item_size,
            dragged_size,
        );
        assert_eq!(shared_offset, -80.0);

        // item0 (full index 0) should stay put.
        let my_full = 0usize;
        let my_filtered = to_filtered_index(my_full, src_full);
        let shared_offset = compute_displacement_offset(
            my_filtered,
            Some(src_full),
            Some(tgt_filtered),
            false,
            item_size,
            dragged_size,
        );
        assert_eq!(shared_offset, 0.0);
    }

    #[test]
    fn test_projection_stays_stable_across_repeated_cycles() {
        // Repeated drag cycles should keep returning the same displacement.
        for _ in 0..100 {
            let offset = compute_displacement_offset(1, Some(0), Some(2), false, 80.0, 80.0);
            assert_eq!(offset, -80.0);

            let offset = compute_displacement_offset(0, Some(2), Some(0), false, 80.0, 80.0);
            assert_eq!(offset, 80.0);
        }
    }
}
