//! Pointer-driven drag lifecycle and traversal computation.
//!
//! This module contains all pointer event handling for drag operations:
//! - Starting, updating, ending, and cancelling drags
//! - Hysteresis filtering for target transitions
//! - Traversal computation for continuous displacement
//! - Time helpers for cross-platform timing

use dioxus::prelude::*;

use super::auto_scroll::compute_scroll_velocity;
use super::{ActiveDrag, DragContext, DropZoneState, sorted_items_in_container};
use crate::sortable_projection::{compute_displacement_offset, to_filtered_index};
use crate::types::{DragData, DragId, DropEvent, DropLocation, Orientation, Position};

/// Hysteresis delay in milliseconds. Target changes must persist for this
/// duration before being committed, filtering boundary oscillation at ~60fps.
pub(super) const HYSTERESIS_MS: f64 = 50.0;

/// Minimum age (ms) for a pending hysteresis candidate to drive projected
/// displacement/traversal visuals.
///
/// Requiring ~one frame of stability suppresses edge flicker where collision
/// rapidly alternates across a boundary (A↔B↔A), which would otherwise cause
/// projected geometry to jump between targets every frame.
pub(super) const PROJECTED_PENDING_MIN_MS: f64 = 18.0;

/// Minimum squared distance (px²) from start position before collision
/// detection activates. Prevents false targets from browser-synthesized
/// pointermove events at drag start. 3px threshold = 9.0 squared.
const ACTIVATION_DISTANCE_SQ: f64 = 9.0;

/// Duration (ms) that the snap window stays open after an item exits traversal.
/// Must exceed HYSTERESIS_MS (50ms) + one render frame (~16ms) to cover the gap
/// between traversal exit and hysteresis-delayed target commit.
pub(super) const SNAP_WINDOW_MS: f64 = 80.0;

/// Minimum interval between drag-time layout invalidations.
///
/// During active drag, geometry may change from auto-scroll, viewport motion,
/// or renderer/layout adjustments. This throttles refresh generation bumps so
/// rect re-measurement stays responsive without spawning redundant work.
const MEASURE_REFRESH_THROTTLE_MS: f64 = 80.0;

/// Select the target used for projected motion/collision geometry.
///
/// A pending hysteresis candidate takes precedence only after it has remained
/// stable for at least [`PROJECTED_PENDING_MIN_MS`]. This damps edge flicker
/// while still aligning projected geometry with maturing target transitions.
pub(crate) fn projected_target_from(
    committed: Option<DropLocation>,
    pending: Option<(DropLocation, f64)>,
    now_ms: f64,
) -> Option<DropLocation> {
    if let Some((loc, started_at)) = pending {
        // Tests use now=0.0 sentinel and expect immediate behavior.
        if now_ms == 0.0 || now_ms - started_at >= PROJECTED_PENDING_MIN_MS {
            return Some(loc);
        }
    }
    committed
}

/// Get current time in milliseconds.
///
/// - **wasm32**: uses `performance.now()` (high-resolution browser timer)
/// - **non-wasm** (desktop/iOS/Android): uses `std::time::Instant` relative to
///   a process-local epoch
/// - **tests**: returns 0.0 to bypass hysteresis (tests call `update_drag`
///   synchronously and expect immediate target commits)
pub(crate) fn current_time_ms() -> f64 {
    #[cfg(test)]
    {
        0.0
    }
    #[cfg(all(not(test), target_arch = "wasm32"))]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    }
    #[cfg(all(not(test), not(target_arch = "wasm32")))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        let epoch = EPOCH.get_or_init(Instant::now);
        epoch.elapsed().as_secs_f64() * 1000.0
    }
}

impl DragContext {
    /// Reset transient drag-motion state shared by pointer and keyboard drags.
    pub(super) fn reset_drag_motion_state(&self) {
        *self.pending_target.write_unchecked() = None;
        *self.traversal_item.write_unchecked() = None;
        *self.traversal_fraction.write_unchecked() = 0.0;
        *self.previous_traversal.write_unchecked() = None;
        *self.scroll_velocity.write_unchecked() = 0.0;
    }

    /// Increment measure generation and stamp refresh time.
    pub(super) fn bump_measure_generation(&self) {
        let now = current_time_ms();
        let mut measure_gen = self.measure_generation;
        *measure_gen.write() += 1;
        *self.last_measure_refresh_ms.write_unchecked() = now;
    }

    /// Throttled measure-generation bump for drag-time viewport/layout motion.
    ///
    /// On tests (`current_time_ms() == 0.0`) this is a no-op to avoid
    /// introducing non-essential signal churn in synchronous unit paths.
    pub(super) fn maybe_refresh_measurements(&self) {
        let now = current_time_ms();
        if now == 0.0 {
            return;
        }
        let last = *self.last_measure_refresh_ms.peek();
        if now - last >= MEASURE_REFRESH_THROTTLE_MS {
            self.bump_measure_generation();
        }
    }

    /// Check whether an incoming pointer event can control the active drag.
    ///
    /// `None` means "unscoped" (keyboard APIs / tests) and is always allowed.
    fn pointer_allowed(&self, pointer_id: Option<i32>) -> bool {
        match (self.active_pointer_id.peek().as_ref(), pointer_id) {
            (_, None) => true,
            (Some(active_id), Some(id)) => *active_id == id,
            (None, Some(_)) => true,
        }
    }

    pub(super) fn start_drag_internal(
        &self,
        data: DragData,
        source_id: DragId,
        position: Position,
        pointer_id: Option<i32>,
    ) {
        // Ignore re-entrant starts while a drag session is already active.
        if self.active.peek().is_some() {
            return;
        }

        // Look up the source zone from registered drop zones (non-reactive peek)
        let zones = self.drop_zones.peek();
        let source_zone = zones.get(&source_id);
        let source_container_id = source_zone.map(|zone| zone.container_id.clone());

        // Clear stale per-drag state before starting.
        self.reset_drag_motion_state();
        *self.current_target.write_unchecked() = None;
        *self.active_pointer_id.write_unchecked() = pointer_id;

        // Start drag in pointer mode by default; keyboard mode is set explicitly
        // by start_keyboard_drag right after calling this.
        *self.keyboard_drag.write_unchecked() = false;
        *self.keyboard_index.write_unchecked() = None;
        *self.keyboard_container.write_unchecked() = None;

        // Bump measure generation to trigger rect re-measurement in all drop
        // zones and sortable items. This ensures cached bounding rects are
        // fresh after browser zoom, scroll, or viewport resize.
        self.bump_measure_generation();

        // Compute grab offset from source zone rect (offset from top-left to grab point)
        let grab_offset = source_zone
            .map(|z| Position {
                x: position.x - z.rect.x,
                y: position.y - z.rect.y,
            })
            .unwrap_or_default();

        // Use write() to notify subscribers (Draggable, SortableItem)
        let mut active = self.active;
        *active.write() = Some(ActiveDrag {
            data,
            source_id,
            source_container_id,
            start_position: position,
            current_position: position,
            delta: Position::default(),
            grab_offset,
        });

        self.set_announcement("Item grabbed");
    }

    /// Start a new drag operation
    pub fn start_drag(&self, data: DragData, source_id: DragId, position: Position) {
        self.start_drag_internal(data, source_id, position, None);
    }

    /// Start a pointer-driven drag operation and lock it to `pointer_id`.
    pub fn start_pointer_drag(
        &self,
        data: DragData,
        source_id: DragId,
        position: Position,
        pointer_id: i32,
    ) {
        self.start_drag_internal(data, source_id, position, Some(pointer_id));
    }

    /// Update the current drag position and run collision detection.
    ///
    /// Uses temporal hysteresis on target-to-target transitions: when the
    /// collision result changes from Some(A) to Some(B), the new candidate
    /// must persist for [`HYSTERESIS_MS`] before being committed. This filters
    /// boundary oscillation at ~60fps while keeping first-target acquisition
    /// (None→Some) and target loss (Some→None) instantaneous.
    ///
    /// On non-wasm (unit tests), `current_time_ms()` returns 0.0 which
    /// bypasses hysteresis entirely — all targets commit immediately.
    pub fn update_drag(&self, position: Position) {
        self.update_drag_with_pointer(position, None);
    }

    /// Pointer-scoped drag update. Events from non-owning pointers are ignored.
    pub fn update_drag_with_pointer(&self, position: Position, pointer_id: Option<i32>) {
        if !self.pointer_allowed(pointer_id) {
            return;
        }

        let now = current_time_ms();

        // Keyboard drags bypass pointer-based collision entirely.
        // Target is set directly by keyboard_move().
        if *self.keyboard_drag.peek() {
            return;
        }

        // Update position silently (hot path, ~60 calls/sec)
        let (target, old_target, source_container, delta) = {
            let mut active_guard = self.active.write_unchecked();
            if let Some(active) = active_guard.as_mut() {
                active.delta = Position {
                    x: position.x - active.start_position.x,
                    y: position.y - active.start_position.y,
                };
                active.current_position = position;

                // Skip collision detection until pointer moves past activation threshold.
                // Some platforms emit immediate move events at the start position;
                // running collision there produces false targets because the source
                // item is excluded from matching.
                let dist_sq = active.delta.x * active.delta.x + active.delta.y * active.delta.y;
                if dist_sq < ACTIVATION_DISTANCE_SQ {
                    return;
                }

                // Clone data needed for collision detection to avoid borrow conflict
                let data_clone = active.data.clone();
                let source_container = active.source_container_id.clone();
                let delta = active.delta;
                let drop_zones = self.drop_zones.peek();
                let committed_target = self.current_target.peek().clone();
                let pending_target = self.pending_target.peek().clone();
                let projected_target =
                    projected_target_from(committed_target.clone(), pending_target, now);

                // Use raw pointer position for collision detection.
                // This aligns collision with the DragOverlay (which follows the cursor),
                // so the user sees collision happen where the overlay is.
                let target = self.collision_strategy.detect(
                    position,
                    &data_clone,
                    &drop_zones,
                    projected_target.as_ref(),
                    self.gap_displacement,
                    delta,
                );

                (target, committed_target, source_container, delta)
            } else {
                return;
            }
        }; // active write guard dropped here

        // Throttled drag-time rect invalidation (viewport/layout motion).
        self.maybe_refresh_measurements();

        // Update traversal state for continuous displacement only when merge is
        // enabled. Plain reorder mode uses canonical discrete projection; keeping
        // traversal active there can cause reverse-direction bounce as items
        // alternate between partial and full offsets near boundaries.
        if self.gap_displacement && self.is_merge_enabled() {
            self.update_traversal(position, &source_container, delta);
        }

        // Update auto-scroll velocity based on pointer proximity to viewport edges
        let velocity = compute_scroll_velocity(position.y);
        *self.scroll_velocity.write_unchecked() = velocity;

        if target == old_target {
            // Reverted back to the committed target (or stayed at None) before a
            // hysteresis candidate matured. Drop the stale candidate so end_drag
            // cannot commit an outdated location.
            if self.pending_target.peek().is_some() {
                *self.pending_target.write_unchecked() = None;
            }
            return; // No change — nothing to do
        }

        // Decide whether to commit immediately or start/check hysteresis
        let should_commit = match (&old_target, &target) {
            // None → Some: first target acquisition — commit immediately
            (None, Some(_)) => true,
            // Some → None: pointer left all zones — commit immediately
            (Some(_), None) => true,
            // Some(A) → Some(B): target change — apply hysteresis
            (Some(_), Some(_)) => {
                let pending = self.pending_target.peek();
                match pending.as_ref() {
                    Some((candidate, timestamp)) if target.as_ref() == Some(candidate) => {
                        // Same candidate still active — check if it's matured
                        // (In tests, now=0.0 and timestamp=0.0 → 0.0 >= 50.0 is false,
                        // but the `now == 0.0` fast path below handles that)
                        now - timestamp >= HYSTERESIS_MS || now == 0.0
                    }
                    _ => {
                        // New candidate or no pending — start timer
                        drop(pending);
                        *self.pending_target.write_unchecked() =
                            Some((target.clone().unwrap(), now));
                        // In test mode (now=0.0), commit immediately
                        now == 0.0
                    }
                }
            }
            // None → None: unreachable (caught by target == old_target above)
            (None, None) => return,
        };

        if should_commit {
            // Clear pending state
            *self.pending_target.write_unchecked() = None;
            // Notify subscribers
            let mut current_target = self.current_target;
            *current_target.write() = target;
            self.set_announcement("Moved to new position");
        }
    }

    /// End the current drag operation and return the drop event if valid
    ///
    /// Returns `None` if:
    /// - There is no active drag
    /// - There is no current target
    /// - The target zone doesn't accept the dragged item's type
    pub fn end_drag(&self) -> Option<DropEvent> {
        self.end_drag_with_pointer(None)
    }

    /// Pointer-scoped drop finalization. Events from non-owning pointers are ignored.
    pub fn end_drag_with_pointer(&self, pointer_id: Option<i32>) -> Option<DropEvent> {
        if !self.pointer_allowed(pointer_id) {
            return None;
        }

        // No active drag session: clear stale transient state defensively.
        if self.active.peek().is_none() {
            self.reset_drag_motion_state();
            *self.current_target.write_unchecked() = None;
            *self.active_pointer_id.write_unchecked() = None;
            return None;
        }

        // If there's a pending hysteresis target that hasn't committed yet,
        // commit it now — the user intended to drop on it.
        let pending = self.pending_target.peek().clone();
        if let Some((pending_loc, _)) = pending {
            let mut ct = self.current_target;
            *ct.write() = Some(pending_loc);
        }

        self.reset_drag_motion_state();
        *self.active_pointer_id.write_unchecked() = None;
        *self.keyboard_drag.write_unchecked() = false;
        *self.keyboard_index.write_unchecked() = None;
        *self.keyboard_container.write_unchecked() = None;

        let mut active_sig = self.active;
        let mut target_sig = self.current_target;
        let active = active_sig.write().take()?;
        let location = target_sig.write().take()?;

        // Check if the target zone accepts the dragged item's types
        let target_container_id = location.container_id();
        if let Some(zone) = self.drop_zones.peek().get(&target_container_id)
            && !zone.accepts_data(&active.data)
        {
            self.set_announcement("Drop cancelled, item returned to start");
            return None;
        }

        // Compute the source index: the dragged item's position among sorted
        // items in its source container (original position before drag).
        let source_index = if let Some(ref source_cid) = active.source_container_id {
            let zones = self.drop_zones.peek();
            let items = sorted_items_in_container(&zones, source_cid, None);
            items.iter().position(|id| *id == active.source_id)
        } else {
            None
        };

        self.set_announcement("Item dropped");
        Some(DropEvent {
            dragged: active.data,
            location,
            source: active.source_id,
            source_container: active.source_container_id,
            source_index,
        })
    }

    /// Cancel the current drag operation
    pub fn cancel_drag(&self) {
        self.cancel_drag_with_pointer(None);
    }

    /// Pointer-scoped cancel. Events from non-owning pointers are ignored.
    pub fn cancel_drag_with_pointer(&self, pointer_id: Option<i32>) {
        if !self.pointer_allowed(pointer_id) {
            return;
        }

        self.reset_drag_motion_state();
        *self.active_pointer_id.write_unchecked() = None;
        // Clear keyboard drag state
        *self.keyboard_drag.write_unchecked() = false;
        *self.keyboard_index.write_unchecked() = None;
        *self.keyboard_container.write_unchecked() = None;
        let mut active = self.active;
        let mut target = self.current_target;
        *active.write() = None;
        *target.write() = None;
        self.set_announcement("Drag cancelled, item returned to start");
    }

    // -------------------------------------------------------------------------
    // Traversal Computation
    // -------------------------------------------------------------------------

    /// Compute projected axis start for traversal hit-testing.
    ///
    /// Traversal must follow the same projected sortable model as collision and
    /// displacement. Otherwise, reversing direction can briefly put the pointer
    /// outside original (base) rects while still visually inside displaced items,
    /// causing traversal to drop to `None` and produce snap artifacts.
    pub(super) fn projected_traversal_axis_start(
        base_start: f64,
        my_full_index: usize,
        source_slot: Option<usize>,
        target_index_and_mode: Option<(usize, bool)>,
        has_any_target: bool,
        item_size: f64,
        dragged_size: f64,
    ) -> f64 {
        let offset = match (source_slot, target_index_and_mode) {
            (Some(src), Some((tgt, is_partial))) => {
                // `my_full_index` includes source slot. Convert this item to the
                // filtered index space used by collision/displacement projection.
                let my_filtered = to_filtered_index(my_full_index, src);
                let target_filtered = if is_partial {
                    to_filtered_index(tgt, src)
                } else {
                    tgt
                };
                compute_displacement_offset(
                    my_filtered,
                    Some(src),
                    Some(target_filtered),
                    is_partial,
                    item_size,
                    dragged_size,
                )
            }
            // Drag-out collapse applies once any target exists.
            (Some(src), None) if has_any_target => {
                let my_filtered = to_filtered_index(my_full_index, src);
                compute_displacement_offset(
                    my_filtered,
                    Some(src),
                    None,
                    false,
                    item_size,
                    dragged_size,
                )
            }
            _ => 0.0,
        };

        base_start + offset
    }

    /// Update traversal signals based on where the pointer is.
    ///
    /// Finds which item the pointer is currently passing through
    /// and how far through it (0.0–1.0). This drives continuous displacement.
    fn update_traversal(
        &self,
        position: Position,
        source_container: &Option<DragId>,
        delta: Position,
    ) {
        let (trav_item, trav_frac) = self.compute_traversal(position, source_container, delta);

        let old_trav = self.traversal_item.peek().clone();
        if trav_item != old_trav {
            // Store exiting item with exit timestamp for snap window (suppresses CSS
            // transition bounce). Items exiting traversal change displacement discretely
            // (e.g., partial → full); without transition suppression, the CSS baseline
            // animates the jump → bounce. Timestamp enables time-based expiry.
            *self.previous_traversal.write_unchecked() = old_trav.map(|id| (id, current_time_ms()));
            // Notify (infrequent — only when crossing item boundary)
            let mut sig = self.traversal_item;
            *sig.write() = trav_item;
        } else {
            // Same traversal item — clear previous if snap window has expired.
            // Window must cover hysteresis delay (50ms) + one render frame (~16ms).
            let expired = self
                .previous_traversal
                .peek()
                .as_ref()
                .is_some_and(|(_, ts)| current_time_ms() - ts >= SNAP_WINDOW_MS);
            if expired {
                *self.previous_traversal.write_unchecked() = None;
            }
        }

        // Always update fraction — only the traversal item subscribes.
        // write_unchecked takes &self (no &mut needed on the DragContext copy).
        *self.traversal_fraction.write_unchecked() = trav_frac;
    }

    /// Compute which item the pointer is traversing and the fraction through it.
    ///
    /// Returns `(Some(item_id), fraction)` when the pointer is within an item's
    /// projected rect (base + canonical displacement), or `(None, 0.0)` when
    /// in a gap between items.
    ///
    /// The `delta` parameter is accepted for API consistency but not currently
    /// used in traversal computation (traversal uses raw pointer position).
    fn compute_traversal(
        &self,
        position: Position,
        source_container: &Option<DragId>,
        _delta: Position,
    ) -> (Option<DragId>, f64) {
        let source_cid = match source_container {
            Some(cid) => cid,
            None => return (None, 0.0),
        };

        let zones = self.drop_zones.peek();

        // Get active drag info to know the dragged item ID
        let active = self.active.peek();
        let dragged_id = match active.as_ref() {
            Some(a) => &a.data.id,
            None => return (None, 0.0),
        };

        // Resolve the source container's orientation for axis selection.
        let container_orientation = zones
            .get(source_cid)
            .map(|z| z.orientation)
            .unwrap_or_default();

        // Collect items in the source container, sorted by primary axis position.
        // Include the dragged item to resolve the source slot for projection.
        let mut container_items: Vec<(&DragId, &DropZoneState)> = zones
            .iter()
            .filter(|(id, zone)| {
                zone.container_id == *source_cid
                    && *id != &zone.container_id // not a container zone
                    && zone.inner_container_id.is_none() // not a nested container wrapper
            })
            .collect();

        container_items.sort_by(|(_, a), (_, b)| {
            let a_pos = match container_orientation {
                Orientation::Vertical => a.rect.y,
                Orientation::Horizontal => a.rect.x,
            };
            let b_pos = match container_orientation {
                Orientation::Vertical => b.rect.y,
                Orientation::Horizontal => b.rect.x,
            };
            a_pos
                .partial_cmp(&b_pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build full-list IDs (includes dragged) for target-index resolution.
        let full_ids: Vec<DragId> = container_items
            .iter()
            .map(|(id, _)| (*id).clone())
            .collect();

        // Source slot is the dragged item's position in the full source list.
        let source_slot = full_ids.iter().position(|id| id == dragged_id);
        let dragged_size = zones.get(dragged_id).map(|z| match container_orientation {
            Orientation::Vertical => z.rect.height,
            Orientation::Horizontal => z.rect.width,
        });

        // Resolve target index/mode for this source container. Use projected
        // target (pending candidate preferred) so traversal stays aligned with
        // collision/displacement during hysteresis windows.
        let projected_target = projected_target_from(
            self.current_target.peek().clone(),
            self.pending_target.peek().clone(),
            current_time_ms(),
        );
        let target_index_and_mode = projected_target.as_ref().and_then(|loc| {
            if loc.container_id() == *source_cid {
                Some((
                    loc.resolve_drop_index(&full_ids),
                    matches!(loc, DropLocation::IntoItem { .. }),
                ))
            } else {
                None
            }
        });
        let has_any_target = projected_target.is_some();

        // Find which item's projected rect contains the pointer along the primary axis.
        let pointer_pos = match container_orientation {
            Orientation::Vertical => position.y,
            Orientation::Horizontal => position.x,
        };
        for (full_idx, (id, zone)) in container_items.iter().enumerate() {
            if *id == dragged_id {
                continue; // Skip the dragged item itself
            }
            let base_start = match container_orientation {
                Orientation::Vertical => zone.rect.y,
                Orientation::Horizontal => zone.rect.x,
            };
            let size = match container_orientation {
                Orientation::Vertical => zone.rect.height,
                Orientation::Horizontal => zone.rect.width,
            };

            if size <= f64::EPSILON {
                continue;
            }

            let start = Self::projected_traversal_axis_start(
                base_start,
                full_idx,
                source_slot,
                target_index_and_mode,
                has_any_target,
                size,
                dragged_size.unwrap_or(size),
            );
            let end = start + size;

            if pointer_pos >= start && pointer_pos < end {
                let fraction = ((pointer_pos - start) / size).clamp(0.0, 1.0);
                return (Some((*id).clone()), fraction);
            }
        }

        (None, 0.0)
    }
}
