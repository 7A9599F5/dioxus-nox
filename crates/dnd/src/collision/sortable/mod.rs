//! Sortable-specific collision detection
//!
//! Optimized collision detection for sortable lists.
//! Determines insertion index based on pointer location
//! relative to item positions.

use std::collections::HashMap;

use crate::context::DropZoneState;
use crate::sortable_projection::compute_displacement_offset;
use crate::types::{DragData, DragId, DropLocation, Orientation, Position};

/// Minimum pixel size for before/after zones. Ensures small items (< 100px)
/// still have usable interaction zones. Items >= 100px are unaffected.
const MIN_ZONE_PX: f64 = 15.0;

/// Overshoot tolerance in pixels. When the pointer exits all container rects,
/// we expand container bounds by this amount to catch near-miss drags at
/// list edges. Industry standard is ~40px (WCAG recommends 44px targets).
const OVERSHOOT_PX: f64 = 40.0;

/// Direction of drag relative to source position
#[derive(Clone, Copy, Debug, PartialEq)]
enum DragDirection {
    /// Dragging toward higher indices (down in vertical, right in horizontal)
    Down,
    /// Dragging toward lower indices (up in vertical, left in horizontal)
    Up,
}

/// Sortable collision detector
///
/// Specialized detector for sortable lists that determines whether
/// the drop should occur before or after each item based on the
/// pointer position relative to item centers.
///
/// When `enable_merge` is true, uses a 30/40/30 split to detect
/// Before/IntoItem/After zones, enabling merge functionality.
///
/// Best for: Reorderable lists (vertical or horizontal)
#[derive(Clone, Debug)]
struct SortableCollisionDetector {
    /// Whether merge (IntoItem) zones are enabled (30/40/30 split vs 50/50)
    enable_merge: bool,
    /// Whether items displace to create gaps (true) or stay in place (false).
    /// When false (indicator mode), zone splits are symmetric and displacement-gap
    /// IntoItem detection is skipped.
    gap_displacement: bool,
    /// Drag delta from start position. Used to determine drag direction
    /// (positive Y = dragging down, negative Y = dragging up) for
    /// direction-aware zone splitting.
    delta: Position,
}

impl Default for SortableCollisionDetector {
    fn default() -> Self {
        Self {
            enable_merge: false,
            gap_displacement: true,
            delta: Position::default(),
        }
    }
}

impl SortableCollisionDetector {
    #[cfg(test)]
    fn vertical() -> Self {
        Self {
            enable_merge: false,
            gap_displacement: true,
            delta: Position::default(),
        }
    }

    #[cfg(test)]
    fn horizontal() -> Self {
        Self {
            enable_merge: false,
            gap_displacement: true,
            delta: Position::default(),
        }
    }

    #[cfg(test)]
    fn with_merge() -> Self {
        Self {
            enable_merge: true,
            gap_displacement: true,
            delta: Position::default(),
        }
    }

    #[cfg(test)]
    fn indicator_mode(enable_merge: bool) -> Self {
        Self {
            enable_merge,
            gap_displacement: false,
            delta: Position::default(),
        }
    }

    #[cfg(test)]
    fn with_delta(mut self, delta: Position) -> Self {
        self.delta = delta;
        self
    }
}

/// Detect sortable collision with displacement awareness
///
/// This is the public entry point used by `CollisionStrategy::Sortable`
/// and `CollisionStrategy::SortableWithMerge`.
///
/// When `enable_merge` is true, uses a 30/40/30 zone split (Before/IntoItem/After).
/// When false, uses a 50/50 split (Before/After only).
/// When `gap_displacement` is false (indicator mode), uses symmetric 15/70/15 zones
/// for merge and skips displacement-gap IntoItem detection.
pub(crate) fn detect_sortable(
    pointer: Position,
    dragged: &DragData,
    zones: &HashMap<DragId, DropZoneState>,
    current_target: Option<&DropLocation>,
    enable_merge: bool,
    gap_displacement: bool,
    delta: Position,
) -> Option<DropLocation> {
    let detector = SortableCollisionDetector {
        enable_merge,
        gap_displacement,
        delta,
    };
    detector.detect(pointer, dragged, zones, current_target)
}

impl SortableCollisionDetector {
    fn detect(
        &self,
        pointer: Position,
        dragged: &DragData,
        zones: &HashMap<DragId, DropZoneState>,
        current_target: Option<&DropLocation>,
    ) -> Option<DropLocation> {
        let mut item_zones: Vec<(&DragId, &DropZoneState)> = Vec::new();
        let mut container_zones: Vec<(&DragId, &DropZoneState)> = Vec::new();

        // Helper to check for cycles (dropping a parent into its own child)
        let is_descendant = |child_zone: &DropZoneState| -> bool {
            let mut current = &child_zone.container_id;
            // Limit depth to prevent infinite loops in graph traversals
            for _ in 0..50 {
                if current == &dragged.id {
                    return true;
                }
                if let Some(parent_zone) = zones.get(current) {
                    current = &parent_zone.container_id;
                } else {
                    break;
                }
            }
            false
        };

        for (id, zone) in zones.iter() {
            if id == &dragged.id {
                continue;
            }

            // Strict Hierarchy Check: Skip zones that are descendants of the dragged item
            if is_descendant(zone) {
                continue;
            }

            if id == &zone.container_id {
                container_zones.push((id, zone));
            } else {
                item_zones.push((id, zone));
            }
        }

        // Filter out zones whose container doesn't accept the dragged item's type.
        // This prevents collision detection AND displacement for items inside
        // non-accepting containers (e.g., group headers can't collide with group children).
        item_zones.retain(|(_, zone)| {
            if let Some(container_zone) = zones.get(&zone.container_id) {
                container_zone.accepts_data(dragged)
            } else {
                true // Container not found — allow (backward compat)
            }
        });

        // Collect rejected container zones BEFORE filtering — used to suppress
        // parent container fallthrough when the pointer is inside a non-accepting
        // nested container's rect.
        let rejected_containers: Vec<&DropZoneState> = container_zones
            .iter()
            .filter(|(_, zone)| !zone.accepts_data(dragged))
            .map(|(_, zone)| *zone)
            .collect();

        container_zones.retain(|(_, zone)| zone.accepts_data(dragged));

        // Build set of container IDs that have nested containers (inner_container_id set).
        // Items inside these containers should NOT get IntoItem zones — only Before/After.
        let nested_container_ids: std::collections::HashSet<&DragId> = item_zones
            .iter()
            .filter_map(|(_, zone)| zone.inner_container_id.as_ref())
            .collect();

        let area = |zone: &DropZoneState| zone.rect.width * zone.rect.height;
        // Resolve orientation per-zone from DropZoneState. This supports mixed
        // orientations in a SortableGroup (e.g., horizontal pills inside a
        // vertical kanban column).
        let pointer_pos_for = |orientation: Orientation| -> f64 {
            match orientation {
                Orientation::Vertical => pointer.y,
                Orientation::Horizontal => pointer.x,
            }
        };
        let axis_start = |zone: &DropZoneState| match zone.orientation {
            Orientation::Vertical => zone.rect.y,
            Orientation::Horizontal => zone.rect.x,
        };
        let axis_size = |zone: &DropZoneState| match zone.orientation {
            Orientation::Vertical => zone.rect.height,
            Orientation::Horizontal => zone.rect.width,
        };

        // Sort items by their original (non-displaced) positions to establish baseline order
        let sorted_items_for_container = |container_id: &DragId| {
            let mut items: Vec<(&DragId, &DropZoneState)> = item_zones
                .iter()
                .copied()
                .filter(|(_, zone)| zone.container_id == *container_id)
                .collect();

            items.sort_by(|(id_a, a), (id_b, b)| {
                axis_start(a)
                    .partial_cmp(&axis_start(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| id_a.0.cmp(&id_b.0))
            });

            items
        };

        // Compute source container and index of the dragged item
        let source_container = zones.get(&dragged.id).map(|z| &z.container_id);
        // Compute effective axis position accounting for visual displacement
        // This mirrors the displacement_transform logic in SortableItem
        let effective_axis_start = |zone: &DropZoneState| -> f64 {
            let base = axis_start(zone);

            // Indicator mode: items don't displace visually, use base positions.
            // Mirrors compute_displacement() in item.rs which returns no_displacement()
            // when !gap_displacement. Prevents phantom displacement from causing
            // boundary oscillation at nested container edges.
            if !self.gap_displacement {
                return base;
            }

            // No current target → items stay at base positions (no displacement)
            // This distinguishes "no target yet" from "target in another container"
            if current_target.is_none() {
                return base;
            }

            // IntoItem stability: when a zone IS the IntoItem target, use its
            // base position for collision detection. IntoItem has no displacement
            // so this is an early-return optimization that also prevents any
            // potential feedback loops.
            if let Some(DropLocation::IntoItem { item_id, .. }) = current_target
                && *item_id == zone.id
            {
                return base;
            }

            let size = axis_size(zone);

            // Get sorted items to find indices
            let sorted = sorted_items_for_container(&zone.container_id);
            let sorted_ids: Vec<DragId> = sorted.iter().map(|(id, _)| (*id).clone()).collect();

            // Find this zone's index in the sorted list
            let my_idx = match sorted.iter().position(|(id, _)| *id == &zone.id) {
                Some(idx) => idx,
                None => return base, // Not in sorted items, use base position
            };

            // Compute source index (dragged item's position in THIS container).
            // When the dragged item is inside a nested child container (e.g., a group
            // header), resolve the group's item zone as the effective source. This
            // ensures parent-level items displace by the group's height, not the header's.
            let (source_idx, effective_dragged_size) =
                if Some(&zone.container_id) == source_container {
                    let ds = zones.get(&dragged.id).map(&axis_size).unwrap_or(size);
                    // The dragged item is excluded from sorted_items (filtered at zone
                    // collection). Compute its position by comparing its base rect to
                    // sorted items — count how many come before it.
                    let idx = zones.get(&dragged.id).map(|dz| {
                        let dragged_pos = axis_start(dz);
                        sorted
                            .iter()
                            .filter(|(_, z)| axis_start(z) < dragged_pos)
                            .count()
                    });
                    (idx, ds)
                } else {
                    // Check if dragged item is inside a nested child of this container
                    let nested_source = source_container.and_then(|src_cid| {
                        zones.values().find(|z| {
                            z.inner_container_id.as_ref() == Some(src_cid)
                                && z.container_id == zone.container_id
                        })
                    });
                    if let Some(parent_zone) = nested_source {
                        let idx = sorted.iter().position(|(id, _)| *id == &parent_zone.id);
                        (idx, axis_size(parent_zone))
                    } else {
                        let ds = zones.get(&dragged.id).map(&axis_size).unwrap_or(size);
                        (None, ds)
                    }
                };

            // Compute target index from current_target (if targeting THIS container)
            let (target_idx, is_partial) = current_target
                .filter(|target| target.container_id() == zone.container_id)
                .map(|target| match target {
                    DropLocation::IntoItem { item_id, .. } => {
                        let idx = sorted_ids
                            .iter()
                            .position(|id| id == item_id)
                            .unwrap_or(sorted_ids.len());
                        (Some(idx), true)
                    }
                    _ => (Some(target.resolve_drop_index(&sorted_ids)), false),
                })
                .unwrap_or((None, false));

            let offset = compute_displacement_offset(
                my_idx,
                source_idx,
                target_idx,
                is_partial,
                size,
                effective_dragged_size,
            );
            base + offset
        };

        // Create an effective rect for a zone that accounts for displacement
        let effective_contains = |zone: &DropZoneState, point: Position| -> bool {
            let eff_start = effective_axis_start(zone);
            let size = axis_size(zone);

            match zone.orientation {
                Orientation::Vertical => {
                    point.x >= zone.rect.x
                        && point.x <= zone.rect.x + zone.rect.width
                        && point.y >= eff_start
                        && point.y <= eff_start + size
                }
                Orientation::Horizontal => {
                    point.y >= zone.rect.y
                        && point.y <= zone.rect.y + zone.rect.height
                        && point.x >= eff_start
                        && point.x <= eff_start + size
                }
            }
        };

        // Filter nested container zones whose parent item zone has displaced away.
        // A nested container (e.g., group-1-container) shares the same rect as its
        // parent item zone (group-1). When the parent displaces in the grandparent
        // container, the inner container's base rect becomes stale — the pointer may
        // be inside the base rect but outside the effective (displaced) rect. Without
        // this filter, the inner container incorrectly wins smallest-area matching.
        container_zones.retain(|(cid, _)| {
            item_zones
                .iter()
                .find(|(_, z)| z.inner_container_id.as_ref() == Some(cid))
                .map(|(_, parent_zone)| effective_contains(parent_zone, pointer))
                .unwrap_or(true) // Not a nested container, keep
        });

        // Pre-compute overshoot: pointer is outside all container rects (horizontal drift).
        // Used below for 1D axis fallback on item matching.
        let is_overshoot = !container_zones
            .iter()
            .any(|(_, zone)| zone.rect.contains(pointer));

        // Find all item zones under the pointer (using effective positions)
        let mut items_under_pointer: Vec<(&DragId, &DropZoneState)> = item_zones
            .iter()
            .copied()
            .filter(|(_, zone)| effective_contains(zone, pointer))
            .collect();

        // Filter children of nested containers whose parent zone has displaced
        // away from the pointer. When a parent item zone (e.g., a group) shifts
        // in its parent container, children visually move with it. But children's
        // base rects don't change, so effective_contains may still return true for
        // their original positions. Check the parent's effective rect to confirm
        // the pointer is still within the displaced group boundary.
        items_under_pointer.retain(|(_, zone)| {
            item_zones
                .iter()
                .find(|(_, z)| z.inner_container_id.as_ref() == Some(&zone.container_id))
                .map(|(_, parent_zone)| effective_contains(parent_zone, pointer))
                .unwrap_or(true) // Not in a nested container, keep
        });

        // Regular items (no nested container) — full collision detection
        let mut item_matches: Vec<(&DragId, &DropZoneState)> = items_under_pointer
            .iter()
            .copied()
            .filter(|(_, zone)| zone.inner_container_id.is_none())
            .collect();

        // During side-overshoot, item rects don't contain the pointer (horizontal drift).
        // Fall back to 1D axis matching — find items whose primary-axis range contains
        // pointer_pos, scoped to the closest overshoot container. This allows IntoItem
        // zones to work even when the pointer drifts sideways during a merge attempt.
        if item_matches.is_empty()
            && is_overshoot
            && let Some((cid, _)) = container_zones
                .iter()
                .copied()
                .filter(|(_, zone)| zone.rect.expanded(OVERSHOOT_PX).contains(pointer))
                .min_by(|(_, a), (_, b)| {
                    let da = pointer.distance_to(a.rect.center());
                    let db = pointer.distance_to(b.rect.center());
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
        {
            item_matches = item_zones
                .iter()
                .copied()
                .filter(|(_, zone)| {
                    zone.inner_container_id.is_none() && zone.container_id == *cid && {
                        let start = effective_axis_start(zone);
                        let size = axis_size(zone);
                        let pp = pointer_pos_for(zone.orientation);
                        pp >= start && pp <= start + size
                    }
                })
                .collect();
        }

        // Reorder stability: when dragging within the same container without
        // merge, prefer base-rect item hits over displaced-rect hits.
        //
        // This breaks reverse-drag feedback loops where the currently selected
        // target displaces an item away from the pointer, causing collision to
        // stop seeing that item and bounce back to a gap target.
        if !self.enable_merge
            && let Some(src_cid) = source_container
        {
            let base_matches: Vec<(&DragId, &DropZoneState)> = item_zones
                .iter()
                .copied()
                .filter(|(_, zone)| {
                    zone.inner_container_id.is_none()
                        && zone.container_id == *src_cid
                        && zone.rect.contains(pointer)
                })
                .collect();
            if !base_matches.is_empty() {
                item_matches = base_matches;
            }
        }

        // Nested container item zones — used for edge detection at group boundaries
        let nested_matches: Vec<(&DragId, &DropZoneState)> = items_under_pointer
            .iter()
            .copied()
            .filter(|(_, zone)| zone.inner_container_id.is_some())
            .collect();

        // Check nested container edge zones FIRST (before regular item matching).
        // Edge zones (top/bottom 12%, clamped 15-30px) return Before/After in the
        // parent container. However, if a child item inside the nested container
        // is under the pointer, prefer the child (fall through) to avoid oscillation
        // between edge zone and child item detection at container boundaries.
        if !nested_matches.is_empty() {
            let mut sorted_nested = nested_matches;
            sorted_nested.sort_by(|(_, a), (_, b)| {
                area(a)
                    .partial_cmp(&area(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let (nested_id, nested_zone) = sorted_nested[0];
            let nested_container_id = nested_zone.container_id.clone();
            let nested_start = effective_axis_start(nested_zone);
            let nested_size = axis_size(nested_zone);
            let edge_size = (nested_size * 0.12).clamp(15.0, 30.0);

            // Helper: check if any child item inside this nested container is
            // under the pointer. Uses BOTH displaced and original positions to
            // prevent oscillation from displacement-shifted effective rects.
            let child_under_pointer = |inner_cid: &Option<DragId>| -> bool {
                inner_cid.as_ref().is_some_and(|icid| {
                    item_zones.iter().any(|(_, z)| {
                        z.container_id == *icid
                            && z.inner_container_id.is_none()
                            && (effective_contains(z, pointer) || z.rect.contains(pointer))
                    })
                })
            };

            let nested_pp = pointer_pos_for(nested_zone.orientation);
            if nested_pp < nested_start + edge_size {
                if !child_under_pointer(&nested_zone.inner_container_id) {
                    // Before the nested group in parent → AtIndex at the nested item's position
                    let sorted_items = sorted_items_for_container(&nested_container_id);
                    let nested_pos = sorted_items
                        .iter()
                        .position(|(id, _)| *id == nested_id)
                        .unwrap_or(0);
                    return Some(DropLocation::AtIndex {
                        container_id: nested_container_id,
                        index: nested_pos,
                    });
                }
                // Child item under pointer — fall through to item matching
            } else if nested_pp > nested_start + nested_size - edge_size {
                // Bottom edge zone always wins — no child_under_pointer check.
                // Unlike the top edge (where the first child legitimately occupies
                // the same space), the bottom edge is the only way to drop BELOW
                // a group. Children at the bottom overlap the edge zone due to
                // container padding, but the user intent at the bottom edge is
                // clearly "escape the group." No oscillation risk: the edge zone
                // targets the parent container, so children's positions don't change.
                let sorted_items = sorted_items_for_container(&nested_container_id);
                let nested_pos = sorted_items
                    .iter()
                    .position(|(id, _)| *id == nested_id)
                    .unwrap_or(0);
                return Some(DropLocation::AtIndex {
                    container_id: nested_container_id,
                    index: nested_pos + 1,
                });
            }
            // Middle zone or child override: fall through to regular item matching
        }

        if !item_matches.is_empty() {
            item_matches.sort_by(|(_, a), (_, b)| {
                area(a)
                    .partial_cmp(&area(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let (item_id, zone) = item_matches[0];
            let container_id = zone.container_id.clone();

            let merge_allowed = self.enable_merge
                && !nested_container_ids.contains(&container_id)
                && zone.accepts_data(dragged);

            // For plain reorder (merge disabled), use base geometry for
            // before/after threshold splitting. Using displaced geometry here can
            // create a feedback loop on reverse drags: target changes displace the
            // item, which moves the split boundary, which flips the target back.
            //
            // Merge mode keeps using displaced geometry because the asymmetric
            // zone split intentionally compensates for displacement-gap geometry.
            let start = if merge_allowed {
                effective_axis_start(zone)
            } else {
                axis_start(zone)
            };
            let size = axis_size(zone);
            let end = start + size;

            // Zone split depends on merge mode, displacement mode, and drag direction:
            // - Merge disabled (or suppressed by group): 50/50 (Before / After)
            // - Indicator mode + merge: symmetric 15/70/15 (no gap asymmetry to compensate)
            // - Gap mode + merge: direction-aware split to compensate for
            //   displacement gap asymmetry (gap is always on the source side)
            let (before_end, after_start) = if !merge_allowed {
                (start + size * 0.5, start + size * 0.5)
            } else if !self.gap_displacement {
                // Indicator mode: symmetric 15/70/15 — no displacement gaps
                // means no directional asymmetry to compensate for
                let clamp_zones = |before_end: f64, after_start: f64| -> (f64, f64) {
                    let before_size = (before_end - start).max(MIN_ZONE_PX).min(size * 0.40);
                    let after_size = (end - after_start).max(MIN_ZONE_PX).min(size * 0.40);
                    (start + before_size, end - after_size)
                };
                clamp_zones(start + size * 0.15, end - size * 0.15)
            } else {
                // Gap displacement mode: direction-aware split
                // Determine drag direction from the drag delta (distance from start).
                // This is grab-position-insensitive — the same delta produces the same
                // direction regardless of where the user grabbed the item. A 1px dead
                // zone prevents micro-oscillation at drag start.
                let delta_axis = match zone.orientation {
                    Orientation::Vertical => self.delta.y,
                    Orientation::Horizontal => self.delta.x,
                };
                let drag_direction = if delta_axis > 1.0 {
                    Some(DragDirection::Down)
                } else if delta_axis < -1.0 {
                    Some(DragDirection::Up)
                } else {
                    None // Dead zone near start
                };

                // Clamp zone boundaries so Before/After zones are at least MIN_ZONE_PX
                let clamp_zones = |before_end: f64, after_start: f64| -> (f64, f64) {
                    let before_size = (before_end - start).max(MIN_ZONE_PX).min(size * 0.40);
                    let after_size = (end - after_start).max(MIN_ZONE_PX).min(size * 0.40);
                    (start + before_size, end - after_size)
                };

                match drag_direction {
                    Some(DragDirection::Down) => {
                        // Dragging DOWN: gap is above target (Before zone side)
                        // Shrink Before zone, expand IntoItem toward top
                        clamp_zones(start + size * 0.15, end - size * 0.30)
                    }
                    Some(DragDirection::Up) => {
                        // Dragging UP: gap is below target (After zone side)
                        // Shrink After zone, expand IntoItem toward bottom
                        clamp_zones(start + size * 0.30, end - size * 0.15)
                    }
                    None => {
                        // Cross-container or unknown: symmetric split
                        clamp_zones(start + size * 0.25, end - size * 0.25)
                    }
                }
            };

            let item_pp = pointer_pos_for(zone.orientation);
            if item_pp < before_end {
                // Before zone: insert at this item's position in the filtered list
                let sorted_items = sorted_items_for_container(&container_id);
                let item_pos = sorted_items
                    .iter()
                    .position(|(id, _)| *id == item_id)
                    .unwrap_or(0);
                return Some(DropLocation::AtIndex {
                    container_id,
                    index: item_pos,
                });
            }

            if item_pp >= after_start {
                // After zone: insert after this item (position + 1)
                let sorted_items = sorted_items_for_container(&container_id);
                let item_pos = sorted_items
                    .iter()
                    .position(|(id, _)| *id == item_id)
                    .unwrap_or(0);
                return Some(DropLocation::AtIndex {
                    container_id,
                    index: item_pos + 1,
                });
            }

            // Middle zone (only reachable in merge mode with 30/40/30 split)
            return Some(DropLocation::IntoItem {
                container_id,
                item_id: item_id.clone(),
            });
        }

        // No item zone matched. Try container zones (covers gaps between items).

        // If the pointer is inside a rejected container (one that doesn't accept
        // the dragged type), suppress fallthrough to parent containers. This prevents
        // the parent container's gap logic from producing drop indicators when the
        // pointer is over a nested container that correctly rejected the drag type.
        let pointer_in_rejected = rejected_containers.iter().any(|z| z.rect.contains(pointer));
        if pointer_in_rejected {
            return None;
        }

        // Phase 1: exact containment (existing behavior)
        let mut container_matches: Vec<(&DragId, &DropZoneState)> = container_zones
            .iter()
            .copied()
            .filter(|(_, zone)| zone.rect.contains(pointer))
            .collect();

        let is_overshoot = container_matches.is_empty();

        // Phase 2: expanded rect fallback for overshoot tolerance
        if is_overshoot {
            container_matches = container_zones
                .iter()
                .copied()
                .filter(|(_, zone)| zone.rect.expanded(OVERSHOOT_PX).contains(pointer))
                .collect();
        }

        if container_matches.is_empty() {
            return None;
        }

        if is_overshoot {
            // During overshoot, closest-center gives "I drifted away from this container"
            container_matches.sort_by(|(_, a), (_, b)| {
                let dist_a = pointer.distance_to(a.rect.center());
                let dist_b = pointer.distance_to(b.rect.center());
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            // In-bounds: smallest-area picks innermost nested container
            container_matches.sort_by(|(_, a), (_, b)| {
                area(a)
                    .partial_cmp(&area(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        let (container_id, _) = container_matches[0];
        let container_id = container_id.clone();

        let sorted_items = sorted_items_for_container(&container_id);
        if sorted_items.is_empty() {
            return Some(DropLocation::IntoContainer { container_id });
        }

        for (i, (item_id, zone)) in sorted_items.iter().enumerate() {
            // Use effective (displaced) position for gap detection
            let start = effective_axis_start(zone);
            let gap_pp = pointer_pos_for(zone.orientation);
            if gap_pp < start {
                // Pointer is in the gap before this item.
                // When merge is enabled, use the bottom half of the gap as
                // an IntoItem zone for the next item, making merge accessible
                // through the displacement gap.
                //
                // Three guards prevent oscillation and invalid merge targets:
                // 1. gap_is_natural: Only allow gap IntoItem when the gap exists
                //    at base positions (pointer < base). Displacement-created gaps
                //    (pointer >= base) cause feedback loops: Before shifts items →
                //    gap opens → IntoItem triggers → items unshift → Before again.
                // 2. prev_displaced: If the previous item is displaced from its
                //    base position, this gap was created by displacement (e.g.,
                //    downward drag shifting items up). Treat as insertion gap, not
                //    merge target. Catches displacement gaps that gap_is_natural
                //    misses when the current item itself is undisplaced.
                // 3. item_allows_merge: Items with inner_container_id are group
                //    containers — merging into them makes no sense (enter instead).
                let base = axis_start(zone);
                let gap_is_natural = gap_pp < base;
                let prev_displaced = if i > 0 {
                    let (_, prev_zone) = &sorted_items[i - 1];
                    (effective_axis_start(prev_zone) - axis_start(prev_zone)).abs() > 0.5
                } else {
                    false
                };
                let item_allows_merge =
                    zone.inner_container_id.is_none() && zone.accepts_data(dragged);
                if self.enable_merge
                    && self.gap_displacement
                    && gap_is_natural
                    && !prev_displaced
                    && item_allows_merge
                    && !nested_container_ids.contains(&container_id)
                    && !is_overshoot
                {
                    // Find the end of the previous item (or container start)
                    let prev_end = if i > 0 {
                        let (_, prev_zone) = &sorted_items[i - 1];
                        effective_axis_start(prev_zone) + axis_size(prev_zone)
                    } else {
                        // No previous item — gap starts at container top
                        0.0
                    };
                    let gap_midpoint = (prev_end + start) / 2.0;
                    if gap_pp >= gap_midpoint {
                        // Bottom half of gap → merge with next item
                        return Some(DropLocation::IntoItem {
                            container_id: container_id.clone(),
                            item_id: (*item_id).clone(),
                        });
                    }
                }
                return Some(DropLocation::AtIndex {
                    container_id: container_id.clone(),
                    index: i,
                });
            }
        }

        // After all items
        Some(DropLocation::AtIndex {
            container_id,
            index: sorted_items.len(),
        })
    }
}

#[cfg(test)]
mod tests;
