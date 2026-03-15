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
    fn with_merge(_orientation: Orientation) -> Self {
        Self {
            enable_merge: true,
            gap_displacement: true,
            delta: Position::default(),
        }
    }

    #[cfg(test)]
    fn indicator_mode(_orientation: Orientation, enable_merge: bool) -> Self {
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
mod tests {
    use super::*;
    use crate::DragType;
    use crate::types::Rect;

    #[test]
    fn test_sortable_vertical_before() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Item at y=0-100 (single item, index 0 in filtered list)
        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Pointer in top half (y=15) -> AtIndex 0 (before item1)
        let result = detector.detect(Position::new(50.0, 15.0), &dragged, &zones, None);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );
    }

    #[test]
    fn test_sortable_vertical_after() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Pointer in bottom half (y=85) -> AtIndex 1 (after item1)
        let result = detector.detect(Position::new(50.0, 85.0), &dragged, &zones, None);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_vertical_center_without_merge() {
        // Without merge: 50/50 split. Center (y=50) is at threshold → after zone (index 1).
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_horizontal() {
        let detector = SortableCollisionDetector::horizontal();
        let mut zones = HashMap::new();

        let mut zone =
            DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zone.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Pointer in left half (x=15) -> AtIndex 0
        let result = detector.detect(Position::new(15.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );

        // Pointer in right half (x=85) -> AtIndex 1
        let result = detector.detect(Position::new(85.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );

        // Without merge: 50/50 split. Center (x=50) is at threshold → AtIndex 1.
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_skips_dragged_item() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        // Dragging item1 itself - should not detect it
        let dragged = DragData::new("item1", "task");
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_sortable_prefers_smaller_zone_over_container() {
        use crate::types::DragType;

        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container zone: large rect covering entire list (0,0) to (200,300)
        let container_zone = DropZoneState::new(
            "list-a",
            "list-a",
            Rect::new(0.0, 0.0, 200.0, 300.0),
            vec![],
        );
        zones.insert(DragId::new("list-a"), container_zone);

        // Item zones inside the container
        let item1_zone = DropZoneState::new(
            "item-a1",
            "list-a",
            Rect::new(10.0, 0.0, 180.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item-a1"), item1_zone);

        let item2_zone = DropZoneState::new(
            "item-a2",
            "list-a",
            Rect::new(10.0, 100.0, 180.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item-a2"), item2_zone);

        let item3_zone = DropZoneState::new(
            "item-a3",
            "list-a",
            Rect::new(10.0, 200.0, 180.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item-a3"), item3_zone);

        // Drag item-b1 from another container
        let dragged = DragData::new("item-b1", "sortable");

        // Run 100 times to catch non-deterministic HashMap behavior
        for i in 0..100 {
            // item-a2 is at y=100-200 (height 100)
            // 30/40/30 split: Before=100-130, Into=130-170, After=170-200
            // Use 115.0 (top 30% of item-a2) to ensure we get Before
            let result = detector.detect(Position::new(100.0, 115.0), &dragged, &zones, None);

            assert!(
                result.is_some(),
                "Iteration {}: Should detect a collision",
                i
            );

            let location = result.unwrap();
            match &location {
                DropLocation::AtIndex { container_id, .. } => {
                    assert_eq!(
                        container_id,
                        &DragId::new("list-a"),
                        "Iteration {}: Should detect in container list-a",
                        i
                    );
                }
                DropLocation::IntoItem { item_id, .. } => {
                    assert_eq!(
                        item_id,
                        &DragId::new("item-a2"),
                        "Iteration {}: Should detect collision with item-a2",
                        i
                    );
                }
                _ => panic!(
                    "Iteration {}: Expected AtIndex or IntoItem, got {:?}",
                    i, location
                ),
            }
        }
    }

    #[test]
    fn test_merge_enabled_three_zones_vertical() {
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Single item at y=0-100 (height 100, index 0 in filtered list)
        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Before zone (y=15) -> AtIndex 0
        let result = detector.detect(Position::new(50.0, 15.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );

        // IntoItem zone (y=50, in middle 40%)
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            }
        );

        // After zone (y=85) -> AtIndex 1
        let result = detector.detect(Position::new(50.0, 85.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_merge_enabled_three_zones_horizontal() {
        let detector = SortableCollisionDetector::with_merge(Orientation::Horizontal);
        let mut zones = HashMap::new();

        let mut zone =
            DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zone.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Before zone (x=15) -> AtIndex 0
        let result = detector.detect(Position::new(15.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );

        // IntoItem zone (x=50, in middle 40%)
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            }
        );

        // After zone (x=85) -> AtIndex 1
        let result = detector.detect(Position::new(85.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_non_merge_50_50_split() {
        // Without merge: 50/50 split — top half → AtIndex 0, bottom half → AtIndex 1
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Top quarter (y=25) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );

        // Bottom quarter (y=75) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 75.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );

        // Center (y=50) → AtIndex 1 (at threshold, >= sends to after zone)
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_gap_maps_to_before_next_item() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container zone covers the full list area
        let container_zone =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container_zone);

        // Two items with a gap between them (0-80, 120-200)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 120.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer in the gap should resolve to AtIndex 1 (before item2)
        let result = detector.detect(Position::new(100.0, 100.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_merge_after_band_collapses_to_before_next() {
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container_zone =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container_zone);

        // Two items with a gap between them
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 140.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Bottom 30% of item1 → after item1 → AtIndex 1
        let result = detector.detect(Position::new(100.0, 90.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_collapses_after_to_before_next() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Two items stacked vertically
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 100.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer in bottom half of item1 (y=80)
        // Without normalization, this would be After(item1) = AtIndex 1
        // With normalization, this is also AtIndex 1 (same index)
        let result = detector.detect(Position::new(50.0, 80.0), &dragged, &zones, None);

        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_sortable_bottom_edge_normalization() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Two items
        // item1: 0-100
        // item2: 120-220 (gap of 20)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 120.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer at 95 (very bottom of item1)
        // This is > center (50), so initially detects as After(item1) = AtIndex 1
        // Normalization: After(item1) → Before(item2) → both are AtIndex 1
        let result = detector.detect(Position::new(50.0, 95.0), &dragged, &zones, None);

        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_displacement_aware_collision_detection() {
        // Test that collision detection accounts for visual displacement
        // when current_target is provided.
        //
        // Scenario: 3 items stacked vertically (each 100px tall)
        //   item1: y=0-100
        //   item2: y=100-200 (dragging this)
        //   item3: y=200-300
        //
        // When dragging item2 down past item3:
        //   - current_target = Before(item3) or After(item3)
        //   - item3 should visually shift up by 100px (to y=100-200)
        //   - Collision detection should use item3's effective position

        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 100.0, 100.0, 100.0), vec![]);
        let item3 =
            DropZoneState::new("item3", "list", Rect::new(0.0, 200.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);
        zones.insert(DragId::new("item3"), item3);

        let dragged = DragData::new("item2", "sortable");

        // Without displacement (current_target = None):
        // Pointer at y=150 is in the lower half of item2's original rect (100-200)
        // But item2 is being dragged, so it should be skipped
        // The pointer is ABOVE item3's original rect (200-300)
        let _result_no_displacement =
            detector.detect(Position::new(50.0, 150.0), &dragged, &zones, None);

        // With no current_target, item3 is at y=200-300
        // Pointer at 150 is NOT inside item3, so we check container fallback
        // Since pointer (150) < item3.start (200), we get Before(item3)
        // Actually, item2 is the dragged item so it's skipped.
        // item1 is at 0-100, item3 is at 200-300, pointer at 150 is in gap
        // This should fall through to container logic if there's a container zone
        // Without container zone, let me add one

        // Let's add a container zone to make this test more realistic
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // Now test again with the container
        let result_no_displacement =
            detector.detect(Position::new(50.0, 150.0), &dragged, &zones, None);
        // Pointer at 150 is in gap (item2 was at 100-200 but is being dragged)
        // Container covers it, so we find before item3 (first item with start > 150)
        // Actually item3 starts at 200, so pointer 150 < 200 means Before(item3)
        // Filtered sorted list (excluding item2): [item1(idx 0), item3(idx 1)]
        // Before(item3) → AtIndex { index: 1 }
        assert_eq!(
            result_no_displacement,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            })
        );

        // Now test WITH displacement:
        // If current_target = AtIndex 2 (after item3), then:
        //   - source_idx = 1 (item2)
        //   - target_idx = 2 (after item3 in filtered list)
        //   - item3 (my_idx=1) is between source and target
        //   - So item3 shifts UP by -100px (to y=100-200)
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };

        // With item3 shifted to y=100-200, pointer at y=150 should now be INSIDE item3's effective rect
        let result_with_displacement = detector.detect(
            Position::new(50.0, 150.0),
            &dragged,
            &zones,
            Some(&current_target),
        );

        // Now item3 is effectively at y=100-200
        // Pointer at 150 is in center region (100 + 50 = 150 = center)
        // For non-merge mode, center=150, pointer at 150 is NOT < center, so it's After
        // Then normalization to Before(next) - but item3 is last, so After(item3)
        // After(item3) at filtered idx 1 → AtIndex { index: 2 }
        assert!(
            result_with_displacement.is_some(),
            "Should detect item3 with displacement"
        );
        match result_with_displacement.unwrap() {
            DropLocation::AtIndex { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("list"),
                    "Should target item3 at its effective position"
                );
            }
            _ => panic!("Expected AtIndex"),
        }
    }

    #[test]
    fn test_displacement_offset_computation() {
        // Test the compute_displacement_offset helper function directly
        // When all items are same size, item_size == dragged_size
        let item_size = 100.0;
        let dragged_size = 100.0;

        // Case 1: Reorder - moving item from idx 1 to idx 3 (full displacement)
        // Items at idx 2 should shift up (between source and target)
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), false, item_size, dragged_size),
            -100.0
        );
        // Items at idx 3 should not shift (at target)
        assert_eq!(
            compute_displacement_offset(3, Some(1), Some(3), false, item_size, dragged_size),
            0.0
        );
        // Items at idx 0 should not shift (before source)
        assert_eq!(
            compute_displacement_offset(0, Some(1), Some(3), false, item_size, dragged_size),
            0.0
        );
        // Items at idx == src: in the filtered list (dragged excluded), this is the
        // first item after the source position. It should shift to fill the source gap.
        assert_eq!(
            compute_displacement_offset(1, Some(1), Some(3), false, item_size, dragged_size),
            -100.0
        );

        // Case 2: Reorder - moving item from idx 3 to idx 1 (backwards, full displacement)
        // Items at idx 2 should shift down (between target and source)
        assert_eq!(
            compute_displacement_offset(2, Some(3), Some(1), false, item_size, dragged_size),
            100.0
        );
        // Items at idx 1 should shift down (at target)
        assert_eq!(
            compute_displacement_offset(1, Some(3), Some(1), false, item_size, dragged_size),
            100.0
        );
        // Items at idx 0 should not shift (before target)
        assert_eq!(
            compute_displacement_offset(0, Some(3), Some(1), false, item_size, dragged_size),
            0.0
        );

        // Case 3: Drag out - source in container, target elsewhere
        // Items after source should shift up
        assert_eq!(
            compute_displacement_offset(2, Some(1), None, false, item_size, dragged_size),
            -100.0
        );
        assert_eq!(
            compute_displacement_offset(3, Some(1), None, false, item_size, dragged_size),
            -100.0
        );
        // Items at source insert position shift (first item after source in filtered list)
        assert_eq!(
            compute_displacement_offset(1, Some(1), None, false, item_size, dragged_size),
            -100.0
        );
        // Items before source should not shift
        assert_eq!(
            compute_displacement_offset(0, Some(1), None, false, item_size, dragged_size),
            0.0
        );

        // Case 4: Drag in - source elsewhere, target in container
        // Items at or after target should shift down
        assert_eq!(
            compute_displacement_offset(1, None, Some(1), false, item_size, dragged_size),
            100.0
        );
        assert_eq!(
            compute_displacement_offset(2, None, Some(1), false, item_size, dragged_size),
            100.0
        );
        // Items before target should not shift
        assert_eq!(
            compute_displacement_offset(0, None, Some(1), false, item_size, dragged_size),
            0.0
        );

        // Case 5: IntoItem/merge - target squeezes 50%, items between get full displacement
        // Items between source and target shift fully (same as Before/After)
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), true, item_size, dragged_size),
            -100.0
        );
        assert_eq!(
            compute_displacement_offset(2, Some(3), Some(1), true, item_size, dragged_size),
            100.0
        );

        // The IntoItem target squeezes 50% toward source
        assert_eq!(
            compute_displacement_offset(3, Some(1), Some(3), true, item_size, dragged_size),
            -50.0
        );
        assert_eq!(
            compute_displacement_offset(1, Some(3), Some(1), true, item_size, dragged_size),
            50.0
        );

        // Items outside the source-target range should not displace
        assert_eq!(
            compute_displacement_offset(0, Some(1), Some(3), true, item_size, dragged_size),
            0.0
        );
        assert_eq!(
            compute_displacement_offset(4, Some(1), Some(3), true, item_size, dragged_size),
            0.0
        );

        // Case 6: Cross-container IntoItem - target squeezes, others stay
        assert_eq!(
            compute_displacement_offset(1, None, Some(1), true, item_size, dragged_size),
            50.0
        );
        // Items after target should NOT shift during cross-container IntoItem
        assert_eq!(
            compute_displacement_offset(2, None, Some(1), true, item_size, dragged_size),
            0.0
        );
    }

    #[test]
    fn test_displacement_with_heterogeneous_sizes() {
        // When dragged item is larger than displaced items, displacement
        // should use dragged item's size for full displacement
        let item_size = 60.0; // small item being displaced
        let dragged_size = 100.0; // large dragged item

        // Full displacement: items between src and tgt shift by dragged_size
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), false, item_size, dragged_size),
            -100.0
        );

        // IntoItem: items between src and tgt get full displacement by dragged_size
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), true, item_size, dragged_size),
            -100.0
        );
        // IntoItem target squeezes by 50% of own size (60 * 0.5 = 30)
        assert_eq!(
            compute_displacement_offset(3, Some(1), Some(3), true, item_size, dragged_size),
            -30.0
        );

        // When dragged item is smaller than displaced items
        let item_size = 100.0; // large item being displaced
        let dragged_size = 60.0; // small dragged item

        // Full displacement: shift by dragged_size (60), not own size (100)
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), false, item_size, dragged_size),
            -60.0
        );

        // IntoItem: items between get full displacement, target squeezes 50% own size
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(3), true, item_size, dragged_size),
            -60.0
        );
        assert_eq!(
            compute_displacement_offset(3, Some(1), Some(3), true, item_size, dragged_size),
            -50.0
        );
    }

    #[test]
    fn test_into_item_stability() {
        // Verify that when current_target is IntoItem(item), re-running detection
        // with the same pointer position still returns IntoItem (not Before/After).
        // This tests that the IntoItem zone is anchored to the item's base position,
        // preventing the displacement feedback loop.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1: y=0-100, item2: y=100-200 (merge target), item3: y=200-300
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 100.0, 100.0, 100.0), vec![]);
        let item3 =
            DropZoneState::new("item3", "list", Rect::new(0.0, 200.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);
        zones.insert(DragId::new("item3"), item3);

        let dragged = DragData::new("item1", "task");

        // Step 1: Pointer at y=150 (center of item2). No current target → base positions.
        // Item2 IntoItem zone (30/40/30): 130-170. y=150 is in middle → IntoItem.
        let result1 = detector.detect(Position::new(50.0, 150.0), &dragged, &zones, None);
        assert_eq!(
            result1,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            })
        );

        // Step 2: Now current_target = IntoItem(item2). Without the stability fix,
        // item2 would be displaced 50%, shifting its collision zone so the pointer
        // falls outside the IntoItem zone. With the fix, item2 stays at base position
        // for collision detection, so the same pointer → same IntoItem result.
        let current = DropLocation::IntoItem {
            container_id: DragId::new("list"),
            item_id: DragId::new("item2"),
        };
        let result2 = detector.detect(Position::new(50.0, 150.0), &dragged, &zones, Some(&current));
        assert_eq!(
            result2,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            })
        );
    }

    #[test]
    fn test_direction_aware_zones_dragging_down() {
        // When dragging DOWN (positive delta.y), Before zone shrinks to 15%
        // and IntoItem expands: 15/55/30 split
        // This compensates for the displacement gap above the target
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, 70.0));
        let mut zones = HashMap::new();

        // item1 (dragged) at y=0-100, item2 (target) at y=100-200
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 100.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("item1", "sortable");

        // At y=120 (20% into item2): with old 30/40/30 this would be Before
        // With new 15/55/30 for drag-down, Before zone ends at 100 + 15 = 115
        // So y=120 should now be IntoItem (merge zone)
        let result = detector.detect(Position::new(50.0, 120.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            }),
            "y=120 should be IntoItem when dragging DOWN (15% Before zone)"
        );

        // At y=110 (10% into item2): still Before even with shrunk zone
        // Filtered list: [item2(idx 0)]. Before(item2) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 110.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "y=110 should still be Before (within 15% zone)"
        );
    }

    #[test]
    fn test_direction_aware_zones_dragging_up() {
        // When dragging UP (negative delta.y), After zone shrinks to 15%
        // and IntoItem expands: 30/55/15 split
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, -70.0));
        let mut zones = HashMap::new();

        // item1 (target) at y=0-100, item2 (dragged) at y=100-200
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        let item2 =
            DropZoneState::new("item2", "list", Rect::new(0.0, 100.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("item2", "sortable");

        // At y=80 (80% into item1): with old 30/40/30 this would be After
        // With new 30/55/15 for drag-up, After zone starts at 100 - 15 = 85
        // So y=80 should now be IntoItem (merge zone)
        let result = detector.detect(Position::new(50.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            }),
            "y=80 should be IntoItem when dragging UP (15% After zone)"
        );

        // At y=90 (90% into item1): should be After even with shrunk zone
        let result = detector.detect(Position::new(50.0, 90.0), &dragged, &zones, None);
        assert!(
            !matches!(result, Some(DropLocation::IntoItem { .. })),
            "y=90 should NOT be IntoItem (within 15% After zone)"
        );
    }

    #[test]
    fn test_cross_container_uses_symmetric_zones() {
        // Cross-container drag: dragged item is NOT in the target's container
        // Should use symmetric 25/50/25 split
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // item_a in container "list-a", item_b in container "list-b"
        let item_a = DropZoneState::new(
            "item_a",
            "list-a",
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item_a"), item_a);

        let dragged = DragData::new("item_b", "sortable");

        // At y=50 (center): should be IntoItem with 25/50/25 split
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list-a"),
                item_id: DragId::new("item_a"),
            }),
            "Center should be IntoItem for cross-container drag"
        );

        // At y=20 (20%): should be Before with 25/50/25 split
        // Filtered list: [item_a(idx 0)]. Before(item_a) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 20.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list-a"),
                index: 0,
            }),
            "Top 25% should be Before for cross-container drag"
        );

        // At y=80 (80%): should be After with 25/50/25 split
        // After(item_a) at filtered idx 0 → AtIndex 1
        let result = detector.detect(Position::new(50.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list-a"),
                index: 1,
            }),
            "Bottom 25% should be After for cross-container drag"
        );
    }

    #[test]
    fn test_gap_into_item_when_merge_enabled() {
        // When merge is enabled and pointer is in the bottom half of a
        // displacement gap, return IntoItem(nextItem) instead of Before(nextItem)
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1: y=0-80, gap: y=80-120, item2: y=120-200
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 120.0, 100.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Gap is y=80-120, midpoint=100
        // Bottom half of gap (y=110): should return IntoItem(item2)
        let result = detector.detect(Position::new(50.0, 110.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            }),
            "Bottom half of gap should return IntoItem when merge enabled"
        );

        // Top half of gap (y=90): should return Before(item2)
        // Filtered list: [item1(idx 0), item2(idx 1)]. Before(item2) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 90.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Top half of gap should return AtIndex when merge enabled"
        );
    }

    #[test]
    fn test_gap_no_into_item_when_merge_disabled() {
        // Without merge, gap should always return Before(nextItem)
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1: y=0-80, gap: y=80-120, item2: y=120-200
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 120.0, 100.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "sortable");

        // Bottom half of gap (y=110): should still be AtIndex (before item2) without merge
        // Filtered list: [item1(idx 0), item2(idx 1)]. Before(item2) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 110.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Gap should return AtIndex even in bottom half when merge disabled"
        );
    }

    #[test]
    fn test_collision_skips_non_accepting_container_zones() {
        // When a container doesn't accept the dragged item's type,
        // items in that container should not be collision targets.
        // This prevents displacement and drop previews for non-accepting containers.
        //
        // Use case: dragging a group header (type "group-header") should NOT
        // produce collisions inside nested containers that only accept "sortable".
        use crate::types::DragType;

        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Parent container (accepts all types — empty accepts)
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 400.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group item zone in parent (wraps nested container)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Nested container that only accepts "sortable" (NOT "group-header")
        let inner_container = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("group-1-container"), inner_container);

        // Child items inside the group
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 100.0),
            vec![],
        );
        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);
        zones.insert(DragId::new("child-2"), child2);

        // Standalone item in parent below the group
        let standalone = DropZoneState::new(
            "standalone-1",
            "parent",
            Rect::new(0.0, 220.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("standalone-1"), standalone);

        // Drag a "group-header" type item (NOT accepted by nested container)
        let dragged = DragData::new("header-1", "group-header");

        // Pointer over child-1 area (y=50) — should NOT match children in the group
        for _ in 0..50 {
            let result = detector.detect(Position::new(100.0, 50.0), &dragged, &zones, None);
            if let Some(
                DropLocation::AtIndex { container_id, .. }
                | DropLocation::IntoItem { container_id, .. },
            ) = result.as_ref()
            {
                assert_ne!(
                    *container_id,
                    DragId::new("group-1-container"),
                    "Should NOT detect collision in non-accepting container"
                );
            }
        }

        // Pointer over standalone item (y=270) — should still be valid
        let result = detector.detect(Position::new(100.0, 270.0), &dragged, &zones, None);
        assert!(
            result.is_some(),
            "Standalone item should still be a valid target"
        );
        match result.unwrap() {
            DropLocation::AtIndex { container_id, .. }
            | DropLocation::IntoItem { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("parent"),
                    "Should target standalone-1 in parent container"
                );
            }
            _ => panic!("Expected AtIndex or IntoItem for standalone item"),
        }
    }

    #[test]
    fn test_collision_allows_accepted_types_in_container() {
        // Verify that items with accepted types still produce collisions.
        // A "sortable" type item SHOULD collide with items inside a container
        // that accepts "sortable".
        use crate::types::DragType;

        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container that accepts "sortable"
        let container = DropZoneState::new(
            "list",
            "list",
            Rect::new(0.0, 0.0, 200.0, 300.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("list"), container);

        let item1 = DropZoneState::new("item-1", "list", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item-1"), item1);

        // Drag a "sortable" type item (accepted by the container)
        let dragged = DragData::new("dragged", "sortable");

        let result = detector.detect(Position::new(100.0, 50.0), &dragged, &zones, None);
        assert!(result.is_some(), "Accepted type should produce collision");
    }

    #[test]
    fn test_nested_container_item_zone_skipped() {
        // When an item zone has inner_container_id set (it represents a nested container),
        // the collision detector should skip it for direct item matching and fall through
        // to the container zone logic or match child items in the inner container.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 400.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group item zone in parent (represents the nested container as an item)
        // This has inner_container_id set, so it should be skipped during item matching.
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Inner container zone for the group's children
        let inner_container = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner_container);

        // Child items inside the group
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 100.0),
            vec![],
        );
        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);
        zones.insert(DragId::new("child-2"), child2);

        // Standalone item below the group
        let standalone = DropZoneState::new(
            "standalone-1",
            "parent",
            Rect::new(0.0, 220.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("standalone-1"), standalone);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer over child-1 (y=50, inside group-1 area at y=0-100)
        // Should match child-1, NOT group-1 (which is skipped due to inner_container_id)
        for _ in 0..50 {
            let result = detector.detect(Position::new(100.0, 50.0), &dragged, &zones, None);
            assert!(result.is_some(), "Should detect collision inside group");
            if let DropLocation::AtIndex { container_id, .. } = result.unwrap() {
                assert_eq!(
                    container_id,
                    DragId::new("group-1-container"),
                    "Should be in the inner container"
                );
            }
        }

        // Pointer over standalone item (y=270, below the group)
        let result = detector.detect(Position::new(100.0, 270.0), &dragged, &zones, None);
        assert!(result.is_some());
        match result.unwrap() {
            DropLocation::AtIndex { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("parent"),
                    "Should target standalone-1 in parent container"
                );
            }
            _ => panic!("Expected AtIndex for standalone item"),
        }
    }

    #[test]
    fn test_merge_suppressed_in_nested_container() {
        // IntoItem should NOT be returned for items inside nested containers,
        // even when enable_merge is true. Items get 50/50 Before/After split.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 400.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group item zone with inner_container_id
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Inner container
        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Child items inside the group (each 100px)
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 100.0),
            vec![],
        );
        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);
        zones.insert(DragId::new("child-2"), child2);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer at center of child-1 (y=50): with merge this would be IntoItem,
        // but inside nested container merge should be suppressed → Before or After
        for _ in 0..50 {
            let result = detector.detect(Position::new(100.0, 50.0), &dragged, &zones, None);
            assert!(result.is_some());
            assert!(
                !matches!(result.unwrap(), DropLocation::IntoItem { .. }),
                "IntoItem should be suppressed inside nested containers"
            );
        }
    }

    #[test]
    fn test_merge_still_works_in_parent_container() {
        // IntoItem should still work for items directly in the parent container
        // (not inside a nested container), even when nested containers exist.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 400.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group item zone with inner_container_id
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Inner container
        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Standalone item in parent (not in nested container)
        let standalone = DropZoneState::new(
            "standalone-1",
            "parent",
            Rect::new(0.0, 220.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("standalone-1"), standalone);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer at center of standalone (y=270): IntoItem should work
        // 25/50/25 cross-container split: IntoItem zone = 245-295
        let result = detector.detect(Position::new(100.0, 270.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("parent"),
                item_id: DragId::new("standalone-1"),
            }),
            "IntoItem should still work for parent-level items"
        );
    }

    #[test]
    fn test_nested_container_edge_zone_before() {
        // Top edge of a nested container: when a child item is under the pointer,
        // prefer the child item over the edge zone to prevent oscillation.
        // When no child is under the pointer, the edge zone fires normally.
        let detector = SortableCollisionDetector::vertical();

        // Case 1: child overlaps top edge → child takes priority
        {
            let mut zones = HashMap::new();
            let parent = DropZoneState::new(
                "parent",
                "parent",
                Rect::new(0.0, 0.0, 200.0, 500.0),
                vec![],
            );
            zones.insert(DragId::new("parent"), parent);

            let above =
                DropZoneState::new("above", "parent", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
            zones.insert(DragId::new("above"), above);

            // Group at y=100-300 (height 200)
            let mut group_zone = DropZoneState::new(
                "group-1",
                "parent",
                Rect::new(0.0, 100.0, 200.0, 200.0),
                vec![],
            );
            group_zone.inner_container_id = Some(DragId::new("group-1-container"));
            zones.insert(DragId::new("group-1"), group_zone);

            let inner = DropZoneState::new(
                "group-1-container",
                "group-1-container",
                Rect::new(0.0, 100.0, 200.0, 200.0),
                vec![],
            );
            zones.insert(DragId::new("group-1-container"), inner);

            // child-1 starts at y=100, overlapping the top edge zone (y=100-124)
            let child1 = DropZoneState::new(
                "child-1",
                "group-1-container",
                Rect::new(0.0, 100.0, 200.0, 100.0),
                vec![],
            );
            zones.insert(DragId::new("child-1"), child1);

            let dragged = DragData::new("dragged", "sortable");

            // Edge size = 24. Pointer at y=105 → child-1 is under pointer,
            // so child takes priority (Before child-1 in container)
            // Filtered list in group-1-container: [child-1(idx 0)]. Before(child-1) → AtIndex 0
            let result = detector.detect(Position::new(100.0, 105.0), &dragged, &zones, None);
            assert_eq!(
                result,
                Some(DropLocation::AtIndex {
                    container_id: DragId::new("group-1-container"),
                    index: 0,
                }),
                "Top edge with child overlap should prefer child item"
            );
        }

        // Case 2: no child overlaps top edge → edge zone fires
        {
            let mut zones = HashMap::new();
            let parent = DropZoneState::new(
                "parent",
                "parent",
                Rect::new(0.0, 0.0, 200.0, 500.0),
                vec![],
            );
            zones.insert(DragId::new("parent"), parent);

            // Group at y=100-300, but children start at y=130 (leaving 30px gap)
            let mut group_zone = DropZoneState::new(
                "group-1",
                "parent",
                Rect::new(0.0, 100.0, 200.0, 200.0),
                vec![],
            );
            group_zone.inner_container_id = Some(DragId::new("group-1-container"));
            zones.insert(DragId::new("group-1"), group_zone);

            let inner = DropZoneState::new(
                "group-1-container",
                "group-1-container",
                Rect::new(0.0, 100.0, 200.0, 200.0),
                vec![],
            );
            zones.insert(DragId::new("group-1-container"), inner);

            // child-1 starts at y=130, NOT overlapping top edge zone (y=100-124)
            let child1 = DropZoneState::new(
                "child-1",
                "group-1-container",
                Rect::new(0.0, 130.0, 200.0, 80.0),
                vec![],
            );
            zones.insert(DragId::new("child-1"), child1);

            let dragged = DragData::new("dragged", "sortable");

            // Pointer at y=105 → no child under pointer, edge zone fires
            // Parent items (excl dragged): [group-1(idx 0)]. Before(group-1) → AtIndex 0
            let result = detector.detect(Position::new(100.0, 105.0), &dragged, &zones, None);
            assert_eq!(
                result,
                Some(DropLocation::AtIndex {
                    container_id: DragId::new("parent"),
                    index: 0,
                }),
                "Top edge without child overlap should return AtIndex(0) in parent"
            );
        }
    }

    #[test]
    fn test_nested_container_edge_zone_after() {
        // Bottom edge of a nested container should return After(group) in the parent
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group at y=100-300 (height 200)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);

        // Standalone item below the group
        let below = DropZoneState::new(
            "below",
            "parent",
            Rect::new(0.0, 320.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("below"), below);

        let dragged = DragData::new("dragged", "sortable");

        // Edge size = 24px. Bottom edge: y > 300 - 24 = 276
        // Pointer at y=290 (in bottom edge)
        // group-1 has a next sibling (below) → Before(below) in parent
        // Parent items (excl dragged): [group-1(idx 0), below(idx 1)]. Before(below) → AtIndex 1
        let result = detector.detect(Position::new(100.0, 290.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("parent"),
                index: 1,
            }),
            "Bottom edge should normalize to AtIndex(1) in parent"
        );
    }

    #[test]
    fn test_nested_container_middle_falls_through_to_children() {
        // Middle of a nested container should fall through to inner container
        // and match child items.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group at y=100-300 (height 200)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Two children inside the group
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 200.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);
        zones.insert(DragId::new("child-2"), child2);

        let dragged = DragData::new("dragged", "sortable");

        // Edge size = 24px. Middle zone: 124 < y < 276
        // Pointer at y=200 (middle, at child-2 boundary)
        // Should match child items, not group in parent
        for _ in 0..50 {
            let result = detector.detect(Position::new(100.0, 200.0), &dragged, &zones, None);
            assert!(result.is_some());
            match result.unwrap() {
                DropLocation::AtIndex { container_id, .. } => {
                    assert_eq!(
                        container_id,
                        DragId::new("group-1-container"),
                        "Middle zone should delegate to inner container"
                    );
                }
                other => panic!("Expected AtIndex in inner container, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_nested_source_resolves_to_group_for_displacement() {
        // When dragging a group header (inside a nested container), the collision
        // detector should resolve the group's item zone in the parent as the
        // effective source for displacement. This ensures parent-level items
        // displace by the group's height, not the header's height.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group item zone in parent (height 200 — represents the full group)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Inner container
        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Header inside the group (height 50 — much smaller than the group)
        let header = DropZoneState::new(
            "header-1",
            "group-1-container",
            Rect::new(0.0, 0.0, 200.0, 50.0),
            vec![],
        );
        zones.insert(DragId::new("header-1"), header);

        // Standalone item below the group (at y=220)
        let standalone = DropZoneState::new(
            "standalone-1",
            "parent",
            Rect::new(0.0, 220.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("standalone-1"), standalone);

        // Drag the header out of the group over the standalone item.
        // The current target is After(standalone-1) in parent.
        let dragged = DragData::new("header-1", "sortable");
        // Parent items (excl header-1, which is in group-1-container):
        // [group-1(idx 0), standalone-1(idx 1)]. After(standalone-1) → AtIndex 2
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("parent"),
            index: 2,
        };

        // With effective source resolution, standalone-1 should detect that
        // the header is inside a nested child of the parent, and use group-1
        // (height 200) as the effective source for displacement.
        //
        // The displacement-aware collision detection should shift standalone-1
        // UP by the group's height (200px), not the header's height (50px).
        //
        // Effective positions:
        // - group-1 (idx 0): source, no displacement
        // - standalone-1 (idx 1): should shift up by 200px → effective y = 220 - 200 = 20
        //
        // Note: group-1 has inner_container_id so it goes to nested_matches,
        // not item_matches. standalone-1 is the only regular item.

        // With pointer at y=50 (between group and displaced standalone):
        // standalone-1's effective position is 20-100, so pointer at 50 should hit it
        let result = detector.detect(
            Position::new(100.0, 50.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert!(
            result.is_some(),
            "Should detect standalone-1 at its displaced position"
        );
    }

    #[test]
    fn test_into_item_displacement_matches_visual() {
        // Verify that collision detection displacement for IntoItem matches visual
        // displacement in item.rs: items between source and IntoItem target get
        // full displacement (same as Before/After), and the target stays at base
        // position (stability fix for IntoItem).
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // 4 items, each 80px, with 10px gaps
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 90.0, 200.0, 80.0), vec![]);
        let item3 = DropZoneState::new("item3", "list", Rect::new(0.0, 180.0, 200.0, 80.0), vec![]);
        let item4 = DropZoneState::new("item4", "list", Rect::new(0.0, 270.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);
        zones.insert(DragId::new("item3"), item3);
        zones.insert(DragId::new("item4"), item4);

        // Drag item1 down over item3 (IntoItem)
        // Filtered list: [item2(0), item3(1), item4(2)], src=0, tgt=1
        // item2 (idx=0): src<=0 && 0<1 → shifts up 80px → effective y=10-90
        // item3 (idx=1): IntoItem target → stays at base y=180 (stability)
        // item4 (idx=2): no shift → y=270
        let dragged = DragData::new("item1", "sortable");
        let current_target = DropLocation::IntoItem {
            container_id: DragId::new("list"),
            item_id: DragId::new("item3"),
        };

        // Pointer at y=50: should hit item2 (now at effective y=10-90)
        let result = detector.detect(
            Position::new(100.0, 50.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert!(result.is_some());
        match result.unwrap() {
            DropLocation::AtIndex { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("list"),
                    "item2 displaced up by 80px — pointer at y=50 should hit it"
                );
            }
            DropLocation::IntoItem { item_id, .. } => {
                assert_eq!(
                    item_id,
                    DragId::new("item2"),
                    "item2 displaced up by 80px — pointer at y=50 should hit it"
                );
            }
            _ => panic!("Expected item-level collision"),
        }

        // Pointer at y=220: should hit item3 at base position (IntoItem stability)
        let result = detector.detect(
            Position::new(100.0, 220.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item3"),
            }),
            "item3 stays at base position — IntoItem target stability"
        );
    }

    #[test]
    fn test_gap_into_item_suppressed_for_displacement_gaps() {
        // When items shift to make room for insertion (Before target), the
        // displacement creates artificial gaps. Gap IntoItem should NOT trigger
        // in these artificial gaps — only in natural gaps (where the gap exists
        // at base positions too). This prevents oscillation between Before and
        // IntoItem targets.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 500.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1 at y=100-180, item2 at y=190-270
        // Dragging item3 (from y=280-360) to Before(item1)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 190.0, 200.0, 80.0), vec![]);
        let item3 = DropZoneState::new("item3", "list", Rect::new(0.0, 280.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);
        zones.insert(DragId::new("item3"), item3);

        let dragged = DragData::new("item3", "sortable");

        // current_target = AtIndex 0 (before item1) causes items to shift down.
        // Filtered list (excl item3): [item1(idx 0), item2(idx 1)]. Before(item1) → AtIndex 0
        // With source_idx=2 and target_idx=0:
        //   item1 (idx=0): 0<=0 && 0<2 → shift +80 → effective 180
        //   item2 (idx=1): 0<=1 && 1<2 → shift +80 → effective 270
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 0,
        };

        // Pointer at y=130: in displacement gap (>= base 100, < effective 180)
        // Should return AtIndex 0, NOT IntoItem — gap is artificial
        let result = detector.detect(
            Position::new(100.0, 130.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Displacement-created gap should return AtIndex, not IntoItem"
        );
    }

    #[test]
    fn test_gap_into_item_suppressed_for_downward_displacement() {
        // When dragging DOWN, items between source and target are displaced UP,
        // creating an insertion gap between the last displaced item and the first
        // undisplaced item. The gap should resolve as Before, not IntoItem.
        //
        // Setup: item0 (y=0) dragged past item1+item2 toward item3.
        // sorted_items (excluding item0): [item1, item2, item3]
        // source_idx=0 (item0 was first), target=Before(item3) → target_idx=2
        // item2 (idx=1): 0<1 && 1<2 → shift -80 → effective y=80
        // item3 (idx=2): no shift → effective y=240
        // Gap between item2 (eff 80+80=160) and item3 (240).
        // Without prev_displaced: pointer in bottom half → IntoItem(item3) ← BUG
        // With prev_displaced: item2 is displaced → Before(item3) ← CORRECT
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item0 is being dragged (excluded from sorted_items by detect())
        let item0 = DropZoneState::new("item0", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item0"), item0);

        // item1 at base y=80 (no displacement: idx=0, not in range 0<idx<2)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 80.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);

        // item2 at base y=160 (displaced up to y=80: idx=1, 0<1<2 → -80)
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 160.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item2"), item2);

        // item3 at base y=240 (no displacement: idx=2, not in range)
        let item3 = DropZoneState::new("item3", "list", Rect::new(0.0, 240.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item3"), item3);

        let dragged = DragData::new("item0", "sortable");

        // Current target: AtIndex 2 (before item3) — item0 is being inserted before item3
        // Filtered list (excl item0): [item1(idx 0), item2(idx 1), item3(idx 2)]. Before(item3) → AtIndex 2
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };

        // Pointer at y=210: in displacement gap between item2 (eff 80+80=160) and item3 (240).
        // Gap midpoint = (160 + 240) / 2 = 200. Pointer at 210 > midpoint → bottom half.
        // Without prev_displaced guard: gap_is_natural=true (210 < 240) → IntoItem(item3).
        // With prev_displaced guard: item2 is displaced → skip IntoItem → AtIndex 2.
        let result = detector.detect(
            Position::new(100.0, 210.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            }),
            "Downward displacement gap should return AtIndex, not IntoItem"
        );
    }

    #[test]
    fn test_gap_into_item_suppressed_for_group_items() {
        // Items with inner_container_id (group items) should never be IntoItem
        // targets via gap detection. Groups should be entered, not merged into.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1 at y=0-80 (regular item)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);

        // group1 at y=120-200 (group item with inner_container_id)
        let mut group1 =
            DropZoneState::new("group1", "list", Rect::new(0.0, 120.0, 200.0, 80.0), vec![]);
        group1.inner_container_id = Some(DragId::new("group1-container"));
        zones.insert(DragId::new("group1"), group1);

        let dragged = DragData::new("dragged", "sortable");

        // Pointer at y=110: in the gap between item1 and group1.
        // item1 ends at 80, group1 starts at 120. Gap is 80-120, midpoint = 100.
        // y=110 >= midpoint 100 → would be IntoItem(group1) without the fix.
        // With the fix, group1 has inner_container_id → skip IntoItem → AtIndex 1.
        // Filtered list: [item1(idx 0), group1(idx 1)]. Before(group1) → AtIndex 1
        let result = detector.detect(Position::new(100.0, 110.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Group items (with inner_container_id) should not be IntoItem targets via gap"
        );
    }

    #[test]
    fn test_min_zone_px_small_item() {
        // 40px item with merge: Before zone should be >= 15px (not 40*0.25 = 10px)
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Container
        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 200.0), vec![]),
        );
        // Small item (40px tall)
        zones.insert(
            DragId::new("item1"),
            DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 40.0), vec![]),
        );

        let dragged = DragData::new("dragged", "task");

        // At y=5 (within MIN_ZONE_PX=15), should be AtIndex 0 (before item1, not IntoItem)
        // Filtered list: [item1(idx 0)]. Before(item1) → AtIndex 0
        let result = detector.detect(Position::new(100.0, 5.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            })
        );
    }

    #[test]
    fn test_min_zone_px_no_change_large_item() {
        // 200px item: 15% of 200 = 30px > MIN_ZONE_PX (15px), so no change
        // delta.y=220 → dragging down → 15/55/30 split
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, 220.0));
        let mut zones = HashMap::new();

        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 500.0), vec![]),
        );
        // Source item
        zones.insert(
            DragId::new("dragged"),
            DropZoneState::new("dragged", "list", Rect::new(0.0, 0.0, 200.0, 200.0), vec![]),
        );
        // Large item (200px tall) at y=200
        zones.insert(
            DragId::new("item1"),
            DropZoneState::new("item1", "list", Rect::new(0.0, 200.0, 200.0, 200.0), vec![]),
        );

        let dragged = DragData::new("dragged", "task");

        // At y=220 (10% into item = 20px), with direction Down (delta.y > 0)
        // Before zone at 15% = 30px, so y=220 < 200+30=230 → AtIndex 0
        // Filtered list (excl dragged): [item1(idx 0)]. Before(item1) → AtIndex 0
        let result = detector.detect(Position::new(100.0, 220.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            })
        );

        // At y=240 (20% into item = 40px), with direction Down
        // Before zone at 15% = 30px, so y=240 > 200+30=230 → IntoItem
        let result = detector.detect(Position::new(100.0, 240.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            })
        );
    }

    #[test]
    fn test_same_container_source_idx_computed_correctly() {
        // Verify that when dragging an item within the same container,
        // the source index is correctly determined from the dragged item's
        // rect position, even though it's excluded from sorted_items.
        //
        // Regression: source_idx was always None for same-container drags
        // because sorted_items excludes the dragged item. This caused
        // compute_displacement_offset to use the (None, Some(tgt)) "drag in"
        // branch instead of (Some(src), Some(tgt)) "reorder" branch.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // 3 items: item1(0-80), item2(90-170), item3(180-260)
        // Dragging item1 (source at position 0) to Before(item2)
        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 90.0, 200.0, 80.0), vec![]);
        let item3 = DropZoneState::new("item3", "list", Rect::new(0.0, 180.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);
        zones.insert(DragId::new("item2"), item2);
        zones.insert(DragId::new("item3"), item3);

        let dragged = DragData::new("item1", "sortable");

        // With current_target = Before(item2):
        // sorted items (without item1): [item2(idx=0), item3(idx=1)]
        // source_idx = 0 (item1 at y=0, 0 sorted items before it)
        // target_idx = 0 (Before(item2) → position of item2 in sorted = 0)
        //
        // With correct source_idx=0, target_idx=0:
        //   item2 (my_idx=0): tgt(0)<=0 && 0<src(0) → 0<=0 && 0<0 → false → 0.0
        //   item3 (my_idx=1): src(0)<1 && 1<tgt(0) → 0<1 && 1<0 → false → 0.0
        //   Neither item shifts! Correct: source is at top, inserting at same spot.
        //
        // With WRONG source_idx=None:
        //   item2 (my_idx=0): (None, Some(0)) → 0>=0 → +80 shift! WRONG
        //   item3 (my_idx=1): (None, Some(0)) → 1>=0 → +80 shift! WRONG

        // Filtered list (excl item1): [item2(idx 0), item3(idx 1)]. Before(item2) → AtIndex 0
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 0,
        };

        // With correct source_idx, item3 stays at base (y=180-260).
        // Pointer at y=220 (center of item3) should detect item3.
        let result = detector.detect(
            Position::new(100.0, 220.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert!(result.is_some(), "Should detect collision");
        match result.unwrap() {
            DropLocation::AtIndex { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("list"),
                    "item3 should be at base position (source_idx correctly computed)"
                );
            }
            DropLocation::IntoItem { item_id, .. } => {
                assert_eq!(
                    item_id,
                    DragId::new("item3"),
                    "item3 should be at base position (source_idx correctly computed)"
                );
            }
            _ => panic!("Expected item-level collision"),
        }
    }

    #[test]
    fn test_cursor_position_collision() {
        // The collision detector receives raw cursor position.
        // Verify that the same cursor position produces the same collision
        // result regardless of where the user grabbed the item.
        //
        // With delta-based direction: delta.y=70 → dragging down → 15/55/30 split.
        // Target item at y=110-210. Before zone: 110 to 110+15=125. y=120 is within Before zone.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, 70.0));
        let mut zones = HashMap::new();

        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 500.0), vec![]),
        );
        // Source item at y=0-100 (filtered as dragged during detection)
        zones.insert(
            DragId::new("dragged"),
            DropZoneState::new("dragged", "list", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]),
        );
        // Target item at y=110-210
        zones.insert(
            DragId::new("target"),
            DropZoneState::new(
                "target",
                "list",
                Rect::new(0.0, 110.0, 200.0, 100.0),
                vec![],
            ),
        );

        let dragged = DragData::new("dragged", "task");

        // Cursor at y=120, within Before zone (zone ends at 125)
        // Filtered list (excl dragged): [target(idx 0)]. Before(target) → AtIndex 0
        let result = detector.detect(Position::new(100.0, 120.0), &dragged, &zones, None);

        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Cursor in Before zone of target (y=120, zone ends at 125)"
        );
    }

    // =========================================================================
    // Indicator Mode Tests
    // =========================================================================

    /// In indicator mode with merge, zone split should be symmetric 15/70/15
    #[test]
    fn test_indicator_mode_symmetric_zones_with_merge() {
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, true);
        let mut zones = HashMap::new();

        // Item at y=0-100 (height 100)
        // 15% Before: y < 15
        // 70% IntoItem: 15 <= y <= 85
        // 15% After: y > 85
        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0)]

        // Before zone (y=7, in top 15%) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 7.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            },
            "Top 15% should be Before zone in indicator mode"
        );

        // IntoItem zone (y=50, in middle 70%)
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            },
            "Middle 70% should be IntoItem zone in indicator mode"
        );

        // IntoItem zone boundary (y=20, just past 15%)
        let result = detector.detect(Position::new(50.0, 20.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            },
            "y=20 should be IntoItem (past 15% Before boundary)"
        );

        // After zone (y=90, in bottom 15%) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 90.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            "Bottom 15% should be After zone in indicator mode"
        );
    }

    /// Indicator mode without merge should use 50/50 split (same as gap mode)
    #[test]
    fn test_indicator_mode_50_50_without_merge() {
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, false);
        let mut zones = HashMap::new();

        // Item at y=0-100 (height 100)
        let zone = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0)]

        // Before zone (y=25, top half) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            },
            "Top half should be Before zone without merge"
        );

        // After zone (y=75, bottom half) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 75.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            "Bottom half should be After zone without merge"
        );
    }

    /// Indicator mode should skip displacement-gap IntoItem detection
    #[test]
    fn test_indicator_mode_skips_gap_into_item() {
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, true);
        let mut zones = HashMap::new();

        // Container with two items and a natural gap between them
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let item1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 80.0), vec![]);
        zones.insert(DragId::new("item1"), item1);

        // Gap between item1 and item2: y=80 to y=120 (40px gap)
        let item2 = DropZoneState::new("item2", "list", Rect::new(0.0, 120.0, 100.0, 80.0), vec![]);
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("dragged", "task");

        // Pointer in gap, bottom half (y=105) — in gap mode this would be IntoItem,
        // but in indicator mode the gap IntoItem should be suppressed
        // Filtered list: [item1(idx 0), item2(idx 1)]. Before(item2) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 105.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            "Indicator mode should not produce gap IntoItem — should be AtIndex instead"
        );
    }

    /// Indicator mode symmetric zones should be the same regardless of drag direction
    /// (no direction-aware biasing)
    #[test]
    fn test_indicator_mode_no_direction_bias() {
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, true);
        let mut zones = HashMap::new();

        // Container
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 100.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // Source item at y=0-100 (height 100)
        let source =
            DropZoneState::new("source", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("source"), source);

        // Target at y=200-300 (dragging DOWN, 100px item)
        let target = DropZoneState::new(
            "target",
            "list",
            Rect::new(0.0, 200.0, 100.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("target"), target);

        let dragged = DragData::new("source", "task");

        // Filtered list (excl source): [target(idx 0)]

        // y=207 — in gap mode with DOWN drag, this would be Before (top 15%)
        // In indicator mode, it should also be Before (top 15% symmetric) → AtIndex 0
        let result = detector.detect(Position::new(50.0, 207.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            },
            "y=207 should be in Before zone (15% = 215 boundary)"
        );

        // y=250 — should be IntoItem in both modes with merge
        let result = detector.detect(Position::new(50.0, 250.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("target"),
            },
            "y=250 should be IntoItem (center of item)"
        );

        // y=290 — in the After zone (bottom 15%)
        // target is last item, so After(target) → AtIndex 1
        let result = detector.detect(Position::new(50.0, 290.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            "y=290 should be in After zone (bottom 15%)"
        );
    }

    #[test]
    fn test_nested_container_indicator_mode_no_phantom_displacement() {
        // In indicator mode (gap_displacement: false), effective_axis_start should
        // return base positions for all zones. This prevents phantom displacement
        // from shifting zone boundaries and causing flutter at nested container edges.
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, true);
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Item above the group (y=0-80)
        let above = DropZoneState::new("above", "parent", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("above"), above);

        // Group item zone in parent (y=100-300)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        // Inner container zone
        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Child at top of group (y=100-180)
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);

        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 180.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("child-2"), child2);

        let dragged = DragData::new("dragged", "sortable");

        // Simulate a current_target that would cause displacement in gap mode.
        // In indicator mode, this should NOT shift any zone boundaries.
        // Children in group-1-container (excl dragged): [child-1(idx 0), child-2(idx 1)]
        // Before(child-1) → AtIndex 0
        let current_target = Some(DropLocation::AtIndex {
            container_id: DragId::new("group-1-container"),
            index: 0,
        });

        // Pointer at top boundary of group (y=105, inside edge zone).
        // With phantom displacement, child-1 could shift away causing oscillation.
        // In indicator mode, child-1 stays at base position → stable inner result.
        let result = detector.detect(
            Position::new(100.0, 105.0),
            &dragged,
            &zones,
            current_target.as_ref(),
        );
        assert!(
            result.is_some(),
            "Should detect collision at group boundary"
        );

        // Run 50 times to verify stability (no oscillation)
        for i in 0..50 {
            let r = detector.detect(
                Position::new(100.0, 105.0),
                &dragged,
                &zones,
                current_target.as_ref(),
            );
            assert_eq!(
                r, result,
                "Indicator mode result should be stable (iteration {i})"
            );
        }

        // The result should resolve to the inner container's child, not the parent edge
        match result.unwrap() {
            DropLocation::AtIndex {
                container_id,
                index,
            } => {
                assert_eq!(
                    container_id,
                    DragId::new("group-1-container"),
                    "Should resolve to inner container, not parent edge zone"
                );
                assert_eq!(index, 0, "Should target child-1 (index 0) inside group");
            }
            other => panic!("Expected AtIndex in inner container, got {:?}", other),
        }
    }

    #[test]
    fn test_nested_container_gap_mode_raw_rect_fallback() {
        // In gap mode, displacement CAN shift child rects legitimately.
        // The raw rect fallback (z.rect.contains) prevents edge zones from
        // firing when the pointer is inside a child's original bounds,
        // even if displacement has shifted the child's effective position away.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Parent container
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Item above group (y=0-80)
        let above = DropZoneState::new("above", "parent", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("above"), above);

        // Group at y=100-300
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // child-1 at y=100-180 (overlaps top edge zone of nested container)
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);

        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 180.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("child-2"), child2);

        let dragged = DragData::new("dragged", "sortable");

        // current_target that could displace child-1 downward in gap mode,
        // shifting its effective position away from the pointer.
        // Children in group-1-container (excl dragged): [child-1(idx 0), child-2(idx 1)]
        // Before(child-1) → AtIndex 0
        let current_target = Some(DropLocation::AtIndex {
            container_id: DragId::new("group-1-container"),
            index: 0,
        });

        // Pointer at y=105 — inside child-1's raw rect (100-180) but potentially
        // outside its displaced effective rect. The raw rect fallback should still
        // detect the child, preventing the edge zone from firing.
        let result = detector.detect(
            Position::new(100.0, 105.0),
            &dragged,
            &zones,
            current_target.as_ref(),
        );
        assert!(
            result.is_some(),
            "Should detect collision at group boundary"
        );

        // With the raw rect fallback, child_under_pointer returns true,
        // so the edge zone should NOT fire — result should be in inner container.
        match result.unwrap() {
            DropLocation::AtIndex { container_id, .. } => {
                assert_eq!(
                    container_id,
                    DragId::new("group-1-container"),
                    "Raw rect fallback should prevent edge zone; result should be in inner container"
                );
            }
            other => panic!(
                "Expected AtIndex in inner container (raw rect fallback), got {:?}",
                other
            ),
        }
    }

    // =========================================================================
    // Delta-Based Direction Tests
    // =========================================================================

    #[test]
    fn test_delta_direction_independent_of_grab_position() {
        // Same delta + same cursor position = same result, regardless of grab position.
        // Two scenarios: user grabs item from top (y=10) vs bottom (y=90).
        // Both move 70px down → delta.y=70 for both.
        // Cursor ends at different positions, but if we test the same cursor Y
        // we should get the same zone split (direction is from delta, not position).
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, 70.0)); // dragging down

        let mut zones = HashMap::new();
        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 500.0), vec![]),
        );
        zones.insert(
            DragId::new("dragged"),
            DropZoneState::new("dragged", "list", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]),
        );
        zones.insert(
            DragId::new("target"),
            DropZoneState::new(
                "target",
                "list",
                Rect::new(0.0, 110.0, 200.0, 100.0),
                vec![],
            ),
        );

        let dragged = DragData::new("dragged", "task");

        // Cursor at y=120: within 15% Before zone (110 to 125) → AtIndex 0
        // Filtered list (excl dragged): [target(idx 0)]. Before(target) → AtIndex 0
        let result = detector.detect(Position::new(100.0, 120.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Same delta gives same direction-aware zone split regardless of grab position"
        );
    }

    #[test]
    fn test_delta_zero_gives_symmetric_split() {
        // Zero delta → no direction → symmetric 25/50/25 split
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical)
            .with_delta(Position::new(0.0, 0.0));

        let mut zones = HashMap::new();
        // Cross-container setup: target in different container than dragged
        let item = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), item);

        let dragged = DragData::new("dragged", "sortable");

        // At y=50 (center): 25/50/25 → IntoItem (merge zone)
        let result = detector.detect(Position::new(50.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item1"),
            }),
            "Zero delta should produce symmetric 25/50/25 split"
        );

        // Filtered list: [item1(idx 0)]

        // At y=15 (15% into item): 25% Before zone → AtIndex 0
        let result = detector.detect(Position::new(50.0, 15.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Zero delta Before zone should be 25% (symmetric)"
        );

        // At y=85 (85% into item): 25% After zone → AtIndex 1 (last item)
        let result = detector.detect(Position::new(50.0, 85.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Zero delta After zone should be 25% (symmetric)"
        );
    }

    // =========================================================================
    // Overshoot tolerance tests
    // =========================================================================

    #[test]
    fn test_overshoot_above_first_item() {
        // Pointer above container top edge → Before(first_item)
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container at y=100..400
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // Items inside: item1 at y=100..200, item2 at y=200..300
        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new("item2", "list", Rect::new(0.0, 200.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 20px above container (y=80, container starts at y=100)
        // Filtered list: [item1(idx 0), item2(idx 1)]. Before(item1) → AtIndex 0
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Overshoot above should resolve to AtIndex 0 (before first item)"
        );
    }

    #[test]
    fn test_overshoot_below_last_item() {
        // Pointer below container bottom edge → After(last_item)
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container at y=100..400
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new("item2", "list", Rect::new(0.0, 200.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 20px below container bottom (y=420, container ends at y=400)
        // Filtered list: [item1(idx 0), item2(idx 1)]. After(item2) → AtIndex 2
        let result = detector.detect(Position::new(100.0, 420.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            }),
            "Overshoot below should resolve to AtIndex 2 (after last item)"
        );
    }

    #[test]
    fn test_overshoot_side_drift() {
        // Pointer drifts off the side of container → nearest item Before/After
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container at x=50..250, y=0..200
        let container =
            DropZoneState::new("list", "list", Rect::new(50.0, 0.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(50.0, 0.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(50.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 30px to the right of container (x=280), at y=50 (within item1 range)
        let result = detector.detect(Position::new(280.0, 50.0), &dragged, &zones, None);
        assert!(
            result.is_some(),
            "Side drift within overshoot tolerance should still resolve"
        );
    }

    #[test]
    fn test_overshoot_beyond_tolerance() {
        // Pointer 50px away (>40px tolerance) → None
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);

        let dragged = DragData::new("dragged", "task");

        // Pointer 50px above container (y=50, container starts at y=100, tolerance is 40px → expanded to y=60)
        let result = detector.detect(Position::new(100.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result, None,
            "Pointer beyond 40px overshoot tolerance should return None"
        );
    }

    #[test]
    fn test_overshoot_with_merge_suppresses_into_item() {
        // Overshoot + merge enabled → should never produce IntoItem
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Container at y=100..400
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new("item2", "list", Rect::new(0.0, 200.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer above container (overshoot) — should not produce IntoItem
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert!(result.is_some());
        if let DropLocation::IntoItem { .. } = result.unwrap() {
            panic!("Overshoot should never produce IntoItem");
        }
    }

    #[test]
    fn test_overshoot_empty_container() {
        // Overshoot on empty container → IntoContainer
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container at y=100..300, no items
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let dragged = DragData::new("dragged", "task");

        // Pointer 20px above empty container
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoContainer {
                container_id: DragId::new("list"),
            }),
            "Overshoot on empty container should return IntoContainer"
        );
    }

    #[test]
    fn test_overshoot_single_item_container() {
        // Single item: overshoot above → Before, below → After
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container at y=100..200
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0)]

        // Overshoot above → AtIndex 0 (before first item)
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Overshoot above single item → AtIndex 0"
        );

        // Overshoot below → AtIndex 1 (after last item)
        let result = detector.detect(Position::new(100.0, 220.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Overshoot below single item → AtIndex 1"
        );
    }

    #[test]
    fn test_overshoot_multi_container_picks_nearest() {
        // Two containers side by side — overshoot resolves to nearest by center distance
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container A at x=0..100, y=0..300
        let ca = DropZoneState::new(
            "list-a",
            "list-a",
            Rect::new(0.0, 0.0, 100.0, 300.0),
            vec![],
        );
        zones.insert(DragId::new("list-a"), ca);
        let a1 = DropZoneState::new("a1", "list-a", Rect::new(0.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("a1"), a1);

        // Container B at x=200..300, y=0..300
        let cb = DropZoneState::new(
            "list-b",
            "list-b",
            Rect::new(200.0, 0.0, 100.0, 300.0),
            vec![],
        );
        zones.insert(DragId::new("list-b"), cb);
        let b1 = DropZoneState::new("b1", "list-b", Rect::new(200.0, 0.0, 100.0, 100.0), vec![]);
        zones.insert(DragId::new("b1"), b1);

        let dragged = DragData::new("dragged", "task");

        // Pointer at x=130, y=150 — between the two containers, closer to A (center at x=50)
        // than B (center at x=250). Both expanded rects cover x=130.
        let result = detector.detect(Position::new(130.0, 150.0), &dragged, &zones, None);
        assert!(result.is_some(), "Should resolve to one of the containers");
        let cid = result.unwrap().container_id();
        assert_eq!(
            cid,
            DragId::new("list-a"),
            "Should pick container A (closer center)"
        );
    }

    #[test]
    fn test_overshoot_type_filtered_container() {
        // Non-accepting container excluded even during overshoot
        use crate::types::DragType;
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Container only accepts "image" type
        let container = DropZoneState::new(
            "list",
            "list",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![DragType::new("image")],
        );
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);

        // Dragging a "task" type (not accepted)
        let dragged = DragData::new("dragged", "task");

        // Pointer above container (overshoot) — should return None because type not accepted
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result, None,
            "Type-filtered container should not match during overshoot"
        );
    }

    #[test]
    fn test_overshoot_nested_container_resolves_in_parent() {
        // When overshooting from a nested container, parent should catch it
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        // Parent container at y=0..500
        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Item in parent at y=0..100
        let p1 = DropZoneState::new("p1", "parent", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("p1"), p1);

        // Nested container at y=100..300 (also an item in parent with inner_container_id)
        let mut nested_item = DropZoneState::new(
            "nested",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        nested_item.inner_container_id = Some(DragId::new("nested-container"));
        zones.insert(DragId::new("nested"), nested_item);

        let nested_container = DropZoneState::new(
            "nested-container",
            "nested-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("nested-container"), nested_container);

        let n1 = DropZoneState::new(
            "n1",
            "nested-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("n1"), n1);

        // Item below nested at y=300..400
        let p2 = DropZoneState::new("p2", "parent", Rect::new(0.0, 300.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("p2"), p2);

        let dragged = DragData::new("dragged", "task");

        // Pointer above entire parent (y=-20, overshoot)
        let result = detector.detect(Position::new(100.0, -20.0), &dragged, &zones, None);
        assert!(
            result.is_some(),
            "Should resolve via overshoot on parent container"
        );
        let cid = result.unwrap().container_id();
        // Should resolve in parent, not nested-container (parent center is closer when above)
        assert!(
            cid == DragId::new("parent") || cid == DragId::new("nested-container"),
            "Should resolve in parent or nested container"
        );
    }

    #[test]
    fn test_overshoot_horizontal_orientation() {
        // Horizontal list: overshoot left → Before(first), overshoot right → After(last)
        let detector = SortableCollisionDetector::horizontal();
        let mut zones = HashMap::new();

        // Container at x=100..400, y=0..100
        let mut container =
            DropZoneState::new("list", "list", Rect::new(100.0, 0.0, 300.0, 100.0), vec![]);
        container.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("list"), container);

        let mut z1 =
            DropZoneState::new("item1", "list", Rect::new(100.0, 0.0, 100.0, 100.0), vec![]);
        z1.orientation = Orientation::Horizontal;
        let mut z2 =
            DropZoneState::new("item2", "list", Rect::new(200.0, 0.0, 100.0, 100.0), vec![]);
        z2.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0), item2(idx 1)]

        // Overshoot left (x=80, container starts at x=100) → AtIndex 0
        let result = detector.detect(Position::new(80.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Horizontal overshoot left → AtIndex 0 (before first item)"
        );

        // Overshoot right (x=420, container ends at x=400) → AtIndex 2
        let result = detector.detect(Position::new(420.0, 50.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            }),
            "Horizontal overshoot right → AtIndex 2 (after last item)"
        );
    }

    #[test]
    fn test_overshoot_indicator_mode() {
        // Indicator mode (gap_displacement=false) also benefits from overshoot tolerance
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, false);
        let mut zones = HashMap::new();

        // Container at y=100..400
        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 100.0, 200.0, 300.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 100.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new("item2", "list", Rect::new(0.0, 200.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0), item2(idx 1)]

        // Overshoot above → AtIndex 0
        let result = detector.detect(Position::new(100.0, 80.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Indicator mode overshoot above → AtIndex 0 (before first item)"
        );

        // Overshoot below → AtIndex 2
        let result = detector.detect(Position::new(100.0, 420.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            }),
            "Indicator mode overshoot below → AtIndex 2 (after last item)"
        );
    }

    #[test]
    fn test_side_overshoot_into_item_with_merge() {
        // Side drift with merge enabled, pointer at item's IntoItem zone Y position
        // → should return IntoItem (not Before/After)
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Container at x=50..250, y=0..200
        let container =
            DropZoneState::new("list", "list", Rect::new(50.0, 0.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // item1: y=0..100, item2: y=100..200
        let z1 = DropZoneState::new("item1", "list", Rect::new(50.0, 0.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(50.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 30px right of container (x=280), at y=150 (item2 center = IntoItem zone)
        // With symmetric 25/50/25 split for overshoot: IntoItem zone is y=125..175
        let result = detector.detect(Position::new(280.0, 150.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            }),
            "Side overshoot at item center should produce IntoItem when merge enabled"
        );
    }

    #[test]
    fn test_side_overshoot_before_after_with_merge() {
        // Side drift with merge enabled, pointer at item's Before/After zone Y position
        // → should return Before/After (not IntoItem)
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        // Container at x=50..250, y=0..200
        let container =
            DropZoneState::new("list", "list", Rect::new(50.0, 0.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(50.0, 0.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(50.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 30px right of container (x=280), at y=110 (item2 Before zone)
        // Symmetric 25/50/25: Before zone is y=100..125
        let result = detector.detect(Position::new(280.0, 110.0), &dragged, &zones, None);
        assert!(result.is_some());
        if let DropLocation::IntoItem { .. } = result.unwrap() {
            panic!("Side overshoot at Before zone should NOT produce IntoItem");
        }

        // Pointer at y=190 (item2 After zone)
        // Symmetric 25/50/25: After zone is y=175..200
        let result = detector.detect(Position::new(280.0, 190.0), &dragged, &zones, None);
        assert!(result.is_some());
        if let DropLocation::IntoItem { .. } = result.unwrap() {
            panic!("Side overshoot at After zone should NOT produce IntoItem");
        }
    }

    #[test]
    fn test_side_overshoot_indicator_mode_into_item() {
        // Side drift in indicator mode (symmetric 15/70/15) with merge → IntoItem works
        let detector = SortableCollisionDetector::indicator_mode(Orientation::Vertical, true);
        let mut zones = HashMap::new();

        // Container at x=50..250, y=0..200
        let container =
            DropZoneState::new("list", "list", Rect::new(50.0, 0.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(50.0, 0.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(50.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 30px right of container (x=280), at y=150 (item2 center = IntoItem zone)
        // Indicator mode 15/70/15: IntoItem zone is y=115..185
        let result = detector.detect(Position::new(280.0, 150.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item2"),
            }),
            "Side overshoot in indicator mode should produce IntoItem at item center"
        );
    }

    #[test]
    fn test_side_overshoot_no_merge() {
        // Side drift without merge → Before/After only (no IntoItem), same as before
        let detector = SortableCollisionDetector::vertical(); // enable_merge: false
        let mut zones = HashMap::new();

        // Container at x=50..250, y=0..200
        let container =
            DropZoneState::new("list", "list", Rect::new(50.0, 0.0, 200.0, 200.0), vec![]);
        zones.insert(DragId::new("list"), container);

        let z1 = DropZoneState::new("item1", "list", Rect::new(50.0, 0.0, 200.0, 100.0), vec![]);
        let z2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(50.0, 100.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer 30px right (x=280), at y=150 (item2 center)
        // Without merge, 50/50 split → Before or After, never IntoItem
        let result = detector.detect(Position::new(280.0, 150.0), &dragged, &zones, None);
        assert!(result.is_some());
        if let DropLocation::IntoItem { .. } = result.unwrap() {
            panic!("Side overshoot without merge should never produce IntoItem");
        }
    }

    #[test]
    fn test_nested_bottom_edge_with_child_overlap() {
        // Bug: when the last child inside a nested container overlaps the
        // bottom edge zone, the edge zone was bypassed (child_under_pointer
        // fallthrough). This caused drops at the group bottom to go INSIDE
        // the group instead of BELOW it in the parent.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // Group at y=100-300 (height 200)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        // Two children that fill the container — last child overlaps bottom edge
        let child1 = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 100.0, 200.0, 100.0),
            vec![],
        );
        let child2 = DropZoneState::new(
            "child-2",
            "group-1-container",
            Rect::new(0.0, 200.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child1);
        zones.insert(DragId::new("child-2"), child2);

        // Standalone item below the group
        let below = DropZoneState::new(
            "below",
            "parent",
            Rect::new(0.0, 320.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("below"), below);

        let dragged = DragData::new("dragged", "sortable");

        // Edge size = 24px. Bottom edge: y > 300 - 24 = 276
        // Pointer at y=290 (in bottom edge zone). child-2 (y=200-300) IS under
        // the pointer, but the bottom edge zone should still win.
        // Parent items (excl dragged): [group-1(idx 0), below(idx 1)]. Before(below) → AtIndex 1
        let result = detector.detect(Position::new(100.0, 290.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("parent"),
                index: 1,
            }),
            "Bottom edge should return AtIndex 1 (before below) in parent even when child overlaps"
        );

        // When group is last item in parent (no next sibling)
        // Parent items (excl dragged): [group-1(idx 0)]. After(group-1) → AtIndex 1
        let mut zones2 = zones.clone();
        zones2.remove(&DragId::new("below"));
        let result = detector.detect(Position::new(100.0, 290.0), &dragged, &zones2, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("parent"),
                index: 1,
            }),
            "Bottom edge should return AtIndex 1 (after group) when group is last in parent"
        );
    }

    #[test]
    fn test_drop_in_gap_between_group_and_item() {
        // Bug: dropping in the gap between a group's bottom and the next item
        // would incorrectly resolve to After(last_item) because the collision
        // detector's displacement didn't match visual displacement. Items at
        // the source insert position (>= src) must shift in the filtered list.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let parent = DropZoneState::new(
            "parent",
            "parent",
            Rect::new(0.0, 0.0, 200.0, 500.0),
            vec![],
        );
        zones.insert(DragId::new("parent"), parent);

        // item-1 at y=0-80 (will be dragged)
        let item1 =
            DropZoneState::new("item-1", "parent", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]);
        zones.insert(DragId::new("item-1"), item1);

        // Group at y=90-290 (height 200)
        let mut group_zone = DropZoneState::new(
            "group-1",
            "parent",
            Rect::new(0.0, 90.0, 200.0, 200.0),
            vec![],
        );
        group_zone.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), group_zone);

        let inner = DropZoneState::new(
            "group-1-container",
            "group-1-container",
            Rect::new(0.0, 90.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("group-1-container"), inner);

        let child = DropZoneState::new(
            "child-1",
            "group-1-container",
            Rect::new(0.0, 90.0, 200.0, 200.0),
            vec![],
        );
        zones.insert(DragId::new("child-1"), child);

        // item-2 at y=300-380
        let item2 = DropZoneState::new(
            "item-2",
            "parent",
            Rect::new(0.0, 300.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("item-2"), item2);

        // Drag item-1, current target = AtIndex 1 (before item-2) in parent
        // Parent items (excl item-1): [group-1(idx 0), item-2(idx 1)]. Before(item-2) → AtIndex 1
        let dragged = DragData::new("item-1", "sortable");
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("parent"),
            index: 1,
        };

        // With correct displacement (>= src), all items after source shift up by 80px:
        // src=0, tgt=1 (AtIndex 1 in filtered list)
        // group (idx=0): 0<=0 && 0<1 → true → shift up -80 → effective y=10
        // item-2 (idx=1): not in range → no shift → effective y=300
        //
        // Visual gap between group (10+200=210) and item-2 (300): y=210-300.
        // Pointer at y=250 should resolve to AtIndex 1 (before item-2).
        let result = detector.detect(
            Position::new(100.0, 250.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("parent"),
                index: 1,
            }),
            "Gap between displaced group and item-2 should resolve to AtIndex 1"
        );
    }

    #[test]
    fn test_displacement_offset_filtered_list_consistency() {
        // Verify that the displacement offset for the filtered list (dragged item
        // excluded) correctly matches visual displacement from item.rs (which uses
        // the full items list including the dragged item).
        //
        // Original list: [A(0), B(1), C(2), D(3)]
        // Drag A from idx 0, target Before(D) at idx 3.
        // Filtered list: [B(0), C(1), D(2)], src_insert=0, tgt=2
        //
        // Visual (item.rs): B(1>0→shift), C(2>0→shift), D(3>0, but 3==3 at tgt→no shift)
        // Collision: B(0>=0 && 0<2→shift), C(1>=0 && 1<2→shift), D(2>=0 && 2<2→false→no shift)
        let item_size = 80.0;
        let dragged_size = 80.0;

        // B at filtered idx 0: should shift up
        assert_eq!(
            compute_displacement_offset(0, Some(0), Some(2), false, item_size, dragged_size),
            -80.0
        );
        // C at filtered idx 1: should shift up
        assert_eq!(
            compute_displacement_offset(1, Some(0), Some(2), false, item_size, dragged_size),
            -80.0
        );
        // D at filtered idx 2: at target, no shift
        assert_eq!(
            compute_displacement_offset(2, Some(0), Some(2), false, item_size, dragged_size),
            0.0
        );

        // Drag D from idx 3, target Before(A) at idx 0.
        // Filtered list: [A(0), B(1), C(2)], src_insert=3, tgt=0
        // Visual: A(0<=0 && 0<3→shift down), B(0<=1 && 1<3→shift), C(0<=2 && 2<3→shift)
        // Collision: A(tgt<=0 && 0<src? 0<=0 && 0<3→shift), B(0<=1 && 1<3→shift), C(0<=2 && 2<3→shift)
        assert_eq!(
            compute_displacement_offset(0, Some(3), Some(0), false, item_size, dragged_size),
            80.0
        );
        assert_eq!(
            compute_displacement_offset(1, Some(3), Some(0), false, item_size, dragged_size),
            80.0
        );
        assert_eq!(
            compute_displacement_offset(2, Some(3), Some(0), false, item_size, dragged_size),
            80.0
        );

        // Drag B from idx 1, target After(C) = Before(D) at idx 3.
        // Filtered list: [A(0), C(1), D(2)], src_insert=1, tgt=2
        // Visual: A(no shift), C(1<=1 && 1<2→shift up), D(no shift)
        // Collision: A(1<=0→false→no shift), C(1<=1 && 1<2→shift), D(1<=2 && 2<2→false)
        assert_eq!(
            compute_displacement_offset(0, Some(1), Some(2), false, item_size, dragged_size),
            0.0
        );
        assert_eq!(
            compute_displacement_offset(1, Some(1), Some(2), false, item_size, dragged_size),
            -80.0
        );
        assert_eq!(
            compute_displacement_offset(2, Some(1), Some(2), false, item_size, dragged_size),
            0.0
        );

        // Drag out: B removed, target elsewhere
        // Filtered: [A(0), C(1), D(2)], src_insert=1
        // A(0>=1→false), C(1>=1→shift), D(2>=1→shift)
        assert_eq!(
            compute_displacement_offset(0, Some(1), None, false, item_size, dragged_size),
            0.0
        );
        assert_eq!(
            compute_displacement_offset(1, Some(1), None, false, item_size, dragged_size),
            -80.0
        );
        assert_eq!(
            compute_displacement_offset(2, Some(1), None, false, item_size, dragged_size),
            -80.0
        );
    }

    #[test]
    fn test_per_zone_orientation_mixed() {
        // Verify that collision detection uses per-zone orientation, not per-detector.
        // Set detector to vertical, but zones have horizontal orientation.
        // Collision should resolve based on the zone's orientation (X-axis).
        let detector = SortableCollisionDetector::vertical(); // detector says vertical
        let mut zones = HashMap::new();

        // Item zone with HORIZONTAL orientation at x=0..100
        let mut zone =
            DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 50.0), vec![]);
        zone.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0)]

        // Pointer at (15, 25): x=15 is in the left half of a 100px-wide item → AtIndex 0
        // If orientation was vertical (y=25, 50px-tall item): y=25 is at midpoint → AtIndex 1
        let result = detector.detect(Position::new(15.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            },
            "Per-zone horizontal orientation should use X-axis even when detector is vertical"
        );

        // Pointer at (85, 25): x=85 is in the right half → AtIndex 1
        let result = detector.detect(Position::new(85.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            "Per-zone horizontal orientation: right half → AtIndex 1"
        );
    }

    #[test]
    fn test_per_zone_orientation_mixed_does_not_false_positive_outside_cross_axis() {
        // With horizontal zones, Y must stay inside the zone's vertical bounds.
        // A detector-level vertical fallback would incorrectly treat Y against
        // the X axis range and produce a false positive.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        let mut zone =
            DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 50.0), vec![]);
        zone.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), zone);

        let dragged = DragData::new("dragged", "task");

        // X is in-range, but Y is outside 0..50, so this must not match.
        let result = detector.detect(Position::new(50.0, 90.0), &dragged, &zones, None);
        assert!(
            result.is_none(),
            "Pointer outside horizontal zone's cross-axis bounds must not collide"
        );
    }

    #[test]
    fn test_horizontal_two_items_before_after() {
        // Two horizontal items side by side — verify correct Before/After detection
        let detector = SortableCollisionDetector::horizontal();
        let mut zones = HashMap::new();

        // Container
        let mut container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 50.0), vec![]);
        container.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("list"), container);

        // item1 at x=0..100
        let mut z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 50.0), vec![]);
        z1.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), z1);

        // item2 at x=100..200
        let mut z2 =
            DropZoneState::new("item2", "list", Rect::new(100.0, 0.0, 100.0, 50.0), vec![]);
        z2.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Filtered list: [item1(idx 0), item2(idx 1)]

        // Left quarter of item1 (x=25) → AtIndex 0
        let result = detector.detect(Position::new(25.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }
        );

        // Right quarter of item1 (x=75) → AtIndex 1 (normalized: After item1 = Before item2)
        let result = detector.detect(Position::new(75.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );

        // Right quarter of item2 (x=175) → AtIndex 2 (after last item)
        let result = detector.detect(Position::new(175.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            }
        );
    }

    #[test]
    fn test_horizontal_gap_detection() {
        // Horizontal container with a gap between items — pointer in gap resolves correctly
        let detector = SortableCollisionDetector::horizontal();
        let mut zones = HashMap::new();

        // Container spans full width
        let mut container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 220.0, 50.0), vec![]);
        container.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("list"), container);

        // item1 at x=0..100, item2 at x=120..220 (20px gap at x=100..120)
        let mut z1 = DropZoneState::new("item1", "list", Rect::new(0.0, 0.0, 100.0, 50.0), vec![]);
        z1.orientation = Orientation::Horizontal;
        let mut z2 =
            DropZoneState::new("item2", "list", Rect::new(120.0, 0.0, 100.0, 50.0), vec![]);
        z2.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("item1"), z1);
        zones.insert(DragId::new("item2"), z2);

        let dragged = DragData::new("dragged", "task");

        // Pointer in gap (x=110) → AtIndex 1 (before item2)
        // Filtered list: [item1(idx 0), item2(idx 1)]
        let result = detector.detect(Position::new(110.0, 25.0), &dragged, &zones, None);
        assert_eq!(
            result.unwrap(),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    // =========================================================================
    // Group Source Suppression Tests
    // =========================================================================

    #[test]
    fn test_group_header_never_produces_into_item() {
        // When dragging a group header (type "group-header"), IntoItem should be
        // suppressed on standalone targets that only accept "sortable". The type
        // mismatch prevents merging a header into a standalone item.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // The dragged item is a group header (no inner_container_id, but type "group-header")
        let header_zone = DropZoneState::new(
            "header-ss1",
            "list",
            Rect::new(0.0, 0.0, 200.0, 100.0),
            vec![],
        );
        zones.insert(DragId::new("header-ss1"), header_zone);

        // Standalone target item at y=120-220 — only accepts "sortable"
        let target = DropZoneState::new(
            "standalone",
            "list",
            Rect::new(0.0, 120.0, 200.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("standalone"), target);

        let dragged = DragData::new("header-ss1", "group-header");

        // Pointer at y=170 — center of standalone item.
        // Without the fix: merge_allowed=true → IntoItem(standalone).
        // With the fix: type mismatch → merge_allowed=false → Before or After.
        let result = detector.detect(Position::new(100.0, 170.0), &dragged, &zones, None);
        assert!(result.is_some(), "Should detect collision");
        if let DropLocation::IntoItem { .. } = result.unwrap() {
            panic!("Group header should never produce IntoItem on a sortable-only target");
        }
    }

    #[test]
    fn test_group_header_gap_into_item_suppressed() {
        // When dragging a group header, gap-based IntoItem should also be
        // suppressed via type mismatch with targets that only accept "sortable".
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // The dragged item is a group header
        let header_zone = DropZoneState::new(
            "header-ss1",
            "list",
            Rect::new(0.0, 0.0, 200.0, 80.0),
            vec![],
        );
        zones.insert(DragId::new("header-ss1"), header_zone);

        // item1 at y=0-80 (standalone, accepts only "sortable")
        let item1 = DropZoneState::new(
            "item1",
            "list",
            Rect::new(0.0, 0.0, 200.0, 80.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item1"), item1);

        // item2 at y=120-200 with a natural gap — accepts only "sortable"
        let item2 = DropZoneState::new(
            "item2",
            "list",
            Rect::new(0.0, 120.0, 200.0, 80.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item2"), item2);

        let dragged = DragData::new("header-ss1", "group-header");

        // Pointer at y=110: in gap between item1 and item2, bottom half.
        // Type mismatch → gap IntoItem suppressed → AtIndex 1 (before item2).
        // Filtered list (excl header-ss1): [item1(idx 0), item2(idx 1)]
        let result = detector.detect(Position::new(100.0, 110.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }),
            "Group header in gap should produce AtIndex, not IntoItem"
        );
    }

    #[test]
    fn test_same_type_items_can_still_merge() {
        // Two "sortable" items with merge enabled → IntoItem should still work.
        // Regression guard: the type-based check must not break normal merging.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // Dragged item (type "sortable")
        let source = DropZoneState::new(
            "item-a",
            "list",
            Rect::new(0.0, 0.0, 200.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item-a"), source);

        // Target item — accepts "sortable"
        let target = DropZoneState::new(
            "item-b",
            "list",
            Rect::new(0.0, 120.0, 200.0, 100.0),
            vec![DragType::new("sortable")],
        );
        zones.insert(DragId::new("item-b"), target);

        let dragged = DragData::new("item-a", "sortable");

        // Pointer at y=170 — center of target. With merge enabled and matching
        // types, this should produce IntoItem.
        let result = detector.detect(Position::new(100.0, 170.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-b"),
            }),
            "Same-type items should still produce IntoItem when merge is enabled"
        );
    }

    #[test]
    fn test_empty_accepts_allows_any_merge() {
        // Target with empty accepts list should allow IntoItem from any drag type.
        // Backward compatibility guard: empty accepts = accepts all.
        let detector = SortableCollisionDetector::with_merge(Orientation::Vertical);
        let mut zones = HashMap::new();

        let container =
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 400.0), vec![]);
        zones.insert(DragId::new("list"), container);

        // Dragged item with unusual type
        let source =
            DropZoneState::new("item-x", "list", Rect::new(0.0, 0.0, 200.0, 100.0), vec![]);
        zones.insert(DragId::new("item-x"), source);

        // Target with empty accepts (accepts all)
        let target = DropZoneState::new(
            "item-y",
            "list",
            Rect::new(0.0, 120.0, 200.0, 100.0),
            vec![], // empty = accepts all
        );
        zones.insert(DragId::new("item-y"), target);

        let dragged = DragData::new("item-x", "custom-type");

        // Pointer at y=170 — center of target. Empty accepts should allow merge.
        let result = detector.detect(Position::new(100.0, 170.0), &dragged, &zones, None);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-y"),
            }),
            "Empty accepts list should allow IntoItem from any drag type"
        );
    }

    #[test]
    fn test_repeated_drag_cycles_keep_reorder_targets_stable() {
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 320.0), vec![]),
        );
        zones.insert(
            DragId::new("item-0"),
            DropZoneState::new(
                "item-0",
                "list",
                Rect::new(0.0, 0.0, 200.0, 80.0),
                vec![DragType::new("sortable")],
            ),
        );
        zones.insert(
            DragId::new("item-1"),
            DropZoneState::new(
                "item-1",
                "list",
                Rect::new(0.0, 80.0, 200.0, 80.0),
                vec![DragType::new("sortable")],
            ),
        );
        zones.insert(
            DragId::new("item-2"),
            DropZoneState::new(
                "item-2",
                "list",
                Rect::new(0.0, 160.0, 200.0, 80.0),
                vec![DragType::new("sortable")],
            ),
        );
        zones.insert(
            DragId::new("item-3"),
            DropZoneState::new(
                "item-3",
                "list",
                Rect::new(0.0, 240.0, 200.0, 80.0),
                vec![DragType::new("sortable")],
            ),
        );

        let dragged = DragData::new("item-0", "sortable");

        // Repeat the same downward/upward drags and ensure targets do not drift.
        for _ in 0..50 {
            let down = detector.detect(Position::new(100.0, 250.0), &dragged, &zones, None);
            assert_eq!(
                down,
                Some(DropLocation::AtIndex {
                    container_id: DragId::new("list"),
                    index: 2,
                })
            );

            let up = detector.detect(Position::new(100.0, 90.0), &dragged, &zones, None);
            assert_eq!(
                up,
                Some(DropLocation::AtIndex {
                    container_id: DragId::new("list"),
                    index: 0,
                })
            );
        }
    }

    #[test]
    fn test_reverse_drag_over_item_does_not_two_cycle_targets() {
        // Repro model: [A, B, C, D], drag A down to between B/C (AtIndex 1 in
        // filtered list), then move pointer back over B. Collision is called
        // repeatedly with previous target as current_target. This should settle
        // to a stable target, not oscillate A↔B forever.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 344.0), vec![]),
        );

        // 80px items with 8px vertical gaps (matches default CSS gap)
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("b"),
            DropZoneState::new("b", "list", Rect::new(0.0, 88.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("c"),
            DropZoneState::new("c", "list", Rect::new(0.0, 176.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("d"),
            DropZoneState::new("d", "list", Rect::new(0.0, 264.0, 200.0, 80.0), vec![]),
        );

        let dragged = DragData::new("a", "sortable");

        // Start from "between B and C" (filtered index 1) to model the down-drag phase.
        let seed_target = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 1,
        });

        // Scan the pointer range over B and assert there is no 2-cycle when
        // feeding detector output back as current_target each frame.
        for y in 88..168 {
            let pointer = Position::new(100.0, y as f64 + 0.5);
            let mut current = seed_target.clone();
            let mut seq: Vec<Option<DropLocation>> = Vec::new();

            for _ in 0..8 {
                let next = detector.detect(pointer, &dragged, &zones, current.as_ref());
                seq.push(next.clone());
                current = next;
            }

            // Detect a sustained A-B-A-B 2-cycle in the tail.
            let n = seq.len();
            let two_cycle =
                seq[n - 1] == seq[n - 3] && seq[n - 2] == seq[n - 4] && seq[n - 1] != seq[n - 2];

            assert!(
                !two_cycle,
                "Detected reverse-drag 2-cycle at y={y}: {:?}",
                seq
            );
        }
    }

    #[test]
    fn test_reverse_drag_prefers_base_item_hit_over_gap_feedback() {
        // Specific reverse-path repro:
        // [A, B, C, D], dragging A with current target between B/C (AtIndex 1).
        // B is displaced up, creating a synthetic gap over B's BASE rect.
        // Pointer over B base rect should still resolve via B (AtIndex 0),
        // not remain stuck at gap target AtIndex 1.
        let detector = SortableCollisionDetector::vertical();
        let mut zones = HashMap::new();

        zones.insert(
            DragId::new("list"),
            DropZoneState::new("list", "list", Rect::new(0.0, 0.0, 200.0, 344.0), vec![]),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", "list", Rect::new(0.0, 0.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("b"),
            DropZoneState::new("b", "list", Rect::new(0.0, 88.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("c"),
            DropZoneState::new("c", "list", Rect::new(0.0, 176.0, 200.0, 80.0), vec![]),
        );
        zones.insert(
            DragId::new("d"),
            DropZoneState::new("d", "list", Rect::new(0.0, 264.0, 200.0, 80.0), vec![]),
        );

        let dragged = DragData::new("a", "sortable");
        let current_target = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 1,
        };

        // y=100 is inside B's base rect (88..168), but when AtIndex 1 is active,
        // B's effective rect is shifted away. Collision should still anchor to B.
        let result = detector.detect(
            Position::new(100.0, 100.0),
            &dragged,
            &zones,
            Some(&current_target),
        );
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 0,
            }),
            "Pointer over B base rect should resolve to before B during reversal"
        );
    }
}
