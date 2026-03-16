//! DragContext provider and state management
//!
//! This module contains the core state types for drag-and-drop operations:
//! - `DropZoneState` - State of a registered drop zone
//! - `ActiveDrag` - Information about the current active drag
//! - `DragState` - Global drag state that lives in context
//! - `DragContext` - The global drag-and-drop context with signal ownership
//! - `DragContextProvider` - Provider component for the drag context

mod auto_scroll;
pub mod keyboard;
mod pointer;
mod provider;

pub use keyboard::{find_inner_container, find_nested_exit, toggle_merge_target};
pub use provider::{DragContextProvider, DragContextProviderProps};

// Re-export for use in tests (these are pub(super) in their modules)
pub(crate) use pointer::{current_time_ms, projected_target_from};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

use crate::collision::CollisionStrategy;
use crate::types::{
    AnnouncementEvent, DragData, DragId, DragType, DropLocation, Orientation, Position, Rect,
};

/// Monotonic counter for generating unique ARIA instruction IDs across
/// multiple DragContextProvider instances on the same page.
pub(super) static PROVIDER_COUNTER: AtomicU32 = AtomicU32::new(0);

// ============================================================================
// Drop Zone State
// ============================================================================

/// State of a registered drop zone
#[derive(Clone, Debug)]
pub struct DropZoneState {
    /// Unique identifier for this drop zone
    pub id: DragId,
    /// The container this drop zone belongs to
    pub container_id: DragId,
    /// Bounding rectangle for collision detection
    pub rect: Rect,
    /// Types this zone accepts (empty = accepts all)
    pub accepts: Vec<DragType>,
    /// Whether the pointer is currently over this zone
    pub is_over: bool,
    /// If set, this item zone wraps a nested container (e.g., a group).
    /// The collision detector will skip this item zone and delegate to
    /// the inner container for gap/Before/After detection.
    pub inner_container_id: Option<DragId>,
    /// Layout orientation of the container this zone belongs to.
    /// Used by collision detection and displacement to switch axes.
    pub orientation: Orientation,
}

impl DropZoneState {
    /// Create a new drop zone state
    pub fn new(
        id: impl Into<DragId>,
        container_id: impl Into<DragId>,
        rect: Rect,
        accepts: Vec<DragType>,
    ) -> Self {
        Self {
            id: id.into(),
            container_id: container_id.into(),
            rect,
            accepts,
            is_over: false,
            inner_container_id: None,
            orientation: Orientation::default(),
        }
    }

    /// Check if this zone accepts the given drag type (single type check)
    ///
    /// For multi-type items, use `accepts_data()` instead.
    pub fn accepts_type(&self, drag_type: &DragType) -> bool {
        self.accepts.is_empty() || self.accepts.contains(drag_type)
    }

    /// Check if this zone accepts the given drag data (supports multiple types)
    ///
    /// Returns true if:
    /// - This zone's accepts list is empty (accepts all), OR
    /// - Any of the drag data's types matches any of this zone's accepted types
    ///
    /// This is the preferred method for checking acceptance when the item
    /// may have multiple drag types.
    pub fn accepts_data(&self, data: &DragData) -> bool {
        data.has_any_type(&self.accepts)
    }
}

// ============================================================================
// Active Drag State
// ============================================================================

/// Information about the current active drag
#[derive(Clone, Debug)]
pub struct ActiveDrag {
    /// Data about the item being dragged
    pub data: DragData,
    /// ID of the source element where drag started
    pub source_id: DragId,
    /// ID of the container the source element belongs to (for cross-container moves)
    pub source_container_id: Option<DragId>,
    /// Position where the drag started
    pub start_position: Position,
    /// Current pointer position
    pub current_position: Position,
    /// Delta from start position (current - start)
    pub delta: Position,
    /// Offset from the source element's top-left to the grab point.
    /// Used by DragOverlay for grab-position-aware rendering.
    pub grab_offset: Position,
}

impl ActiveDrag {
    /// Create a new active drag
    pub fn new(
        data: DragData,
        source_id: impl Into<DragId>,
        source_container_id: Option<DragId>,
        position: Position,
    ) -> Self {
        Self {
            data,
            source_id: source_id.into(),
            source_container_id,
            start_position: position,
            current_position: position,
            delta: Position::default(),
            grab_offset: Position::default(),
        }
    }

    /// Update the current position and recalculate delta
    pub fn update_position(&mut self, position: Position) {
        self.current_position = position;
        self.delta = Position {
            x: position.x - self.start_position.x,
            y: position.y - self.start_position.y,
        };
    }
}

// ============================================================================
// Global Drag State
// ============================================================================

/// Global drag state - lives in context
///
/// This struct holds all the state needed for drag-and-drop operations:
/// - The currently active drag (if any)
/// - All registered drop zones
/// - The current collision/hover target
#[derive(Clone, Debug, Default)]
pub struct DragState {
    /// Currently active drag, if any
    pub active: Option<ActiveDrag>,

    /// All registered drop zones and their current state
    pub drop_zones: HashMap<DragId, DropZoneState>,

    /// Current collision/hover target
    pub current_target: Option<DropLocation>,
}

impl DragState {
    /// Create a new empty drag state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there is an active drag operation
    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }

    /// Check if a specific item is being dragged
    pub fn is_dragging_id(&self, id: &DragId) -> bool {
        self.active
            .as_ref()
            .map(|a| &a.data.id == id)
            .unwrap_or(false)
    }

    /// Check if a drop zone is the current target
    pub fn is_over(&self, id: &DragId) -> bool {
        self.current_target
            .as_ref()
            .map(|loc| loc.contains_id(id))
            .unwrap_or(false)
    }

    /// Register a new drop zone
    pub fn register_drop_zone(&mut self, state: DropZoneState) {
        self.drop_zones.insert(state.id.clone(), state);
    }

    /// Unregister a drop zone
    pub fn unregister_drop_zone(&mut self, id: &DragId) {
        self.drop_zones.remove(id);
    }

    /// Update the rect of an existing drop zone
    pub fn update_drop_zone_rect(&mut self, id: &DragId, rect: Rect) {
        if let Some(zone) = self.drop_zones.get_mut(id) {
            zone.rect = rect;
        }
    }
}

// ============================================================================
// DragContext - Global Context with Signal Ownership
// ============================================================================

/// The global drag-and-drop context
///
/// All draggables and drop zones register here. This struct owns granular
/// signals for different aspects of drag state, enabling components to
/// subscribe only to the data they need:
///
/// - `active` — drag start/end (read by Draggable, SortableItem, DragOverlay)
/// - `current_target` — collision target changes (read by DropZone, SortableItem)
/// - `drop_zones` — registration data (non-reactive, only used during collision detection)
/// - `collision_strategy` — pluggable collision strategy (Copy, no Signal needed)
#[derive(Clone, Copy)]
pub struct DragContext {
    /// The currently active drag operation (reactive)
    pub(super) active: Signal<Option<ActiveDrag>>,
    /// Current collision/hover target (reactive)
    pub(super) current_target: Signal<Option<DropLocation>>,
    /// Registered drop zones (non-reactive — only used in collision detection)
    pub(super) drop_zones: Signal<HashMap<DragId, DropZoneState>>,
    /// The collision detection strategy (Copy enum — no boxing or Signal needed)
    pub(super) collision_strategy: CollisionStrategy,
    /// Pending target for hysteresis filtering (candidate + timestamp_ms).
    /// Target changes (Some(A) → Some(B)) are delayed by HYSTERESIS_MS to
    /// filter boundary oscillation. None→Some and Some→None commit immediately.
    pub(super) pending_target: Signal<Option<(DropLocation, f64)>>,
    /// Which item the projected center is currently traversing (reactive).
    /// Only the matching SortableItem subscribes to traversal_fraction.
    pub(super) traversal_item: Signal<Option<DragId>>,
    /// 0.0–1.0: how far through the traversal item the projected center has moved.
    /// Updated at ~60fps during drag. Only the traversal item reads this.
    pub(super) traversal_fraction: Signal<f64>,
    /// The item that just exited traversal, paired with the exit timestamp (ms).
    /// When an item exits traversal, its displacement changes discretely (e.g.,
    /// partial → full shift). Without suppressing the CSS transition, the browser
    /// animates this jump over 250ms → visible bounce. Items check this via
    /// `peek()` during re-render and append `transition: 0s`.
    /// The timestamp enables a time-based snap window (SNAP_WINDOW_MS) that
    /// outlasts the hysteresis delay, preventing bounces on direction reversal.
    pub(super) previous_traversal: Signal<Option<(DragId, f64)>>,
    /// Whether items displace to create gaps (true) or stay in place with
    /// line indicators (false). Controls visual feedback mode only — collision
    /// strategy is independent.
    pub(super) gap_displacement: bool,
    /// Monotonically increasing counter bumped on drag start and throttled
    /// drag-time viewport/layout invalidations. Drop zones and sortable items
    /// read this signal in their rect-measurement effects to refresh geometry
    /// when drag/session conditions change.
    pub(super) measure_generation: Signal<u32>,
    /// Timestamp (ms) of the last measure-generation bump.
    /// Used to throttle drag-time invalidation under viewport motion.
    pub(super) last_measure_refresh_ms: Signal<f64>,
    /// Text content for the ARIA live region. Updated on drag lifecycle
    /// events so screen readers announce state changes.
    pub(super) announcement: Signal<String>,
    /// Vertical scroll velocity in px/frame (positive = down, negative = up).
    /// When non-zero, a RAF loop in DragContextProvider scrolls the viewport.
    pub(super) scroll_velocity: Signal<f64>,
    /// Optional callback for structured keyboard drag announcements.
    /// Stored here so any method (start_keyboard_drag, keyboard_move, etc.)
    /// can dispatch events without needing to pass the callback around.
    pub(super) on_announce: Signal<Option<EventHandler<AnnouncementEvent>>>,
    /// Whether the current drag is keyboard-driven (no pointer overlay needed).
    pub(super) keyboard_drag: Signal<bool>,
    /// Virtual cursor position in the current container's items list.
    pub(super) keyboard_index: Signal<Option<usize>>,
    /// Which container the keyboard cursor is navigating.
    pub(super) keyboard_container: Signal<Option<DragId>>,
    /// Pointer id that owns the current pointer drag session.
    /// `None` for keyboard-initiated drags.
    pub(super) active_pointer_id: Signal<Option<i32>>,
    /// Unique ID for the ARIA instructions element, ensuring no duplicates
    /// when multiple DragContextProviders exist on the same page.
    pub(super) instructions_id: Signal<String>,
}

impl DragContext {
    /// Create a new DragContext with the specified collision strategy
    pub fn new(strategy: CollisionStrategy) -> Self {
        let id = PROVIDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            active: Signal::new(None),
            current_target: Signal::new(None),
            drop_zones: Signal::new(HashMap::new()),
            collision_strategy: strategy,
            pending_target: Signal::new(None),
            traversal_item: Signal::new(None),
            traversal_fraction: Signal::new(0.0),
            previous_traversal: Signal::new(None),
            gap_displacement: true,
            measure_generation: Signal::new(0),
            last_measure_refresh_ms: Signal::new(0.0),
            announcement: Signal::new(String::new()),
            scroll_velocity: Signal::new(0.0),
            on_announce: Signal::new(None),
            keyboard_drag: Signal::new(false),
            keyboard_index: Signal::new(None),
            keyboard_container: Signal::new(None),
            active_pointer_id: Signal::new(None),
            instructions_id: Signal::new(format!("dxdnd-drag-instructions-{}", id)),
        }
    }

    /// Create a new DragContext with collision strategy and gap displacement mode
    pub fn with_options(strategy: CollisionStrategy, gap_displacement: bool) -> Self {
        let id = PROVIDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            active: Signal::new(None),
            current_target: Signal::new(None),
            drop_zones: Signal::new(HashMap::new()),
            collision_strategy: strategy,
            pending_target: Signal::new(None),
            traversal_item: Signal::new(None),
            traversal_fraction: Signal::new(0.0),
            previous_traversal: Signal::new(None),
            gap_displacement,
            measure_generation: Signal::new(0),
            last_measure_refresh_ms: Signal::new(0.0),
            announcement: Signal::new(String::new()),
            scroll_velocity: Signal::new(0.0),
            on_announce: Signal::new(None),
            keyboard_drag: Signal::new(false),
            keyboard_index: Signal::new(None),
            keyboard_container: Signal::new(None),
            active_pointer_id: Signal::new(None),
            instructions_id: Signal::new(format!("dxdnd-drag-instructions-{}", id)),
        }
    }

    /// Get the current active drag state
    pub fn active(&self) -> Option<ActiveDrag> {
        self.active.read().clone()
    }

    // -------------------------------------------------------------------------
    // Registration Methods
    // -------------------------------------------------------------------------

    /// Register a drop zone with the context
    pub fn register_drop_zone(
        &self,
        id: DragId,
        container_id: DragId,
        rect: Rect,
        accepts: Vec<DragType>,
        orientation: Orientation,
    ) {
        // Non-reactive: drop_zones is only used during collision detection
        self.drop_zones.write_unchecked().insert(
            id.clone(),
            DropZoneState {
                id,
                container_id,
                rect,
                accepts,
                is_over: false,
                inner_container_id: None,
                orientation,
            },
        );
    }

    /// Register a nested container zone (dual registration).
    ///
    /// Used by nested `SortableContext` components. This registers:
    /// 1. An **item zone** in the parent container (for displacement and ordering)
    ///    with `inner_container_id` set so the collision detector skips it for
    ///    gap detection and delegates to the inner container.
    /// 2. A **container zone** for the nested container's own children.
    ///
    /// Both registrations share the same rect (the nested container's bounding box).
    pub fn register_nested_container(
        &self,
        id: DragId,
        parent_container_id: DragId,
        inner_container_id: DragId,
        rect: Rect,
        accepts: Vec<DragType>,
        orientation: Orientation,
    ) {
        let mut zones = self.drop_zones.write_unchecked();

        // 1. Item zone in parent (for displacement ordering)
        // The item zone inherits the PARENT container's orientation (for displacement).
        // Look up parent orientation from the parent container zone.
        let parent_orientation = zones
            .get(&parent_container_id)
            .map(|z| z.orientation)
            .unwrap_or_default();
        zones.insert(
            id.clone(),
            DropZoneState {
                id,
                container_id: parent_container_id,
                rect,
                accepts: accepts.clone(),
                is_over: false,
                inner_container_id: Some(inner_container_id.clone()),
                orientation: parent_orientation,
            },
        );

        // 2. Container zone for children (uses the nested container's own orientation)
        zones.insert(
            inner_container_id.clone(),
            DropZoneState {
                id: inner_container_id.clone(),
                container_id: inner_container_id,
                rect,
                accepts,
                is_over: false,
                inner_container_id: None,
                orientation,
            },
        );
    }

    /// Unregister a drop zone from the context
    pub fn unregister(&self, id: &DragId) {
        self.drop_zones.write_unchecked().remove(id);
    }

    /// Update the bounding rect of a registered drop zone
    pub fn update_rect(&self, id: &DragId, rect: Rect) {
        if let Some(zone) = self.drop_zones.write_unchecked().get_mut(id) {
            zone.rect = rect;
        }
    }

    // -------------------------------------------------------------------------
    // Query Methods
    // -------------------------------------------------------------------------

    /// Check if there is an active drag operation (subscribing read)
    pub fn is_dragging(&self) -> bool {
        self.active.read().is_some()
    }

    /// Check if a specific item is being dragged (subscribing read on `active`)
    pub fn is_dragging_id(&self, id: &DragId) -> bool {
        self.active
            .read()
            .as_ref()
            .map(|a| &a.data.id == id)
            .unwrap_or(false)
    }

    /// Check if a drop zone is the current hover target (subscribing read on `current_target`)
    pub fn is_over(&self, id: &DragId) -> bool {
        self.current_target
            .read()
            .as_ref()
            .map(|loc| loc.contains_id(id))
            .unwrap_or(false)
    }

    /// Get the current drop location target, if any (subscribing read on `current_target`)
    pub fn get_drop_location(&self) -> Option<DropLocation> {
        self.current_target.read().clone()
    }

    /// Get the projected target used for collision/displacement geometry.
    ///
    /// Uses the committed target, or a sufficiently stable pending candidate,
    /// so visual motion stays in sync during target transitions without
    /// reacting to one-frame boundary noise.
    pub fn projected_drop_location(&self) -> Option<DropLocation> {
        projected_target_from(
            self.current_target.read().clone(),
            self.pending_target.read().clone(),
            current_time_ms(),
        )
    }

    /// Get information about the active drag, if any (subscribing read on `active`)
    pub fn get_active_drag(&self) -> Option<ActiveDrag> {
        self.active.read().clone()
    }

    /// Get the container ID for a registered drop zone (non-reactive peek)
    pub fn get_zone_container(&self, item_id: &DragId) -> Option<DragId> {
        self.drop_zones
            .peek()
            .get(item_id)
            .map(|zone| zone.container_id.clone())
    }

    /// Get the height of a registered drop zone (non-reactive peek)
    ///
    /// Used by SortableItem to compute pixel-based displacement transforms
    /// when items have different heights.
    pub fn get_zone_height(&self, item_id: &DragId) -> Option<f64> {
        self.drop_zones
            .peek()
            .get(item_id)
            .map(|zone| zone.rect.height)
    }

    /// Get the width of a registered drop zone (non-reactive peek)
    ///
    /// Used by SortableItem to compute pixel-based displacement transforms
    /// for horizontal lists.
    pub fn get_zone_width(&self, item_id: &DragId) -> Option<f64> {
        self.drop_zones
            .peek()
            .get(item_id)
            .map(|zone| zone.rect.width)
    }

    /// Get the size of a drop zone along its orientation axis (non-reactive peek).
    ///
    /// Returns height for vertical containers, width for horizontal containers.
    pub fn get_zone_size(&self, item_id: &DragId) -> Option<f64> {
        self.drop_zones
            .peek()
            .get(item_id)
            .map(|zone| match zone.orientation {
                Orientation::Vertical => zone.rect.height,
                Orientation::Horizontal => zone.rect.width,
            })
    }

    /// Get the orientation of a registered drop zone (non-reactive peek)
    pub fn get_zone_orientation(&self, item_id: &DragId) -> Orientation {
        self.drop_zones
            .peek()
            .get(item_id)
            .map(|zone| zone.orientation)
            .unwrap_or_default()
    }

    /// Get the active drag signal for direct subscription in effects/memos.
    pub fn active_signal(&self) -> Signal<Option<ActiveDrag>> {
        self.active
    }

    /// Get the current target signal for direct subscription in effects/memos.
    pub fn target_signal(&self) -> Signal<Option<DropLocation>> {
        self.current_target
    }

    /// Get the traversal item signal (which item the projected center is traversing).
    pub fn traversal_item_signal(&self) -> Signal<Option<DragId>> {
        self.traversal_item
    }

    /// Get the traversal fraction signal (0.0–1.0 through the traversal item).
    pub fn traversal_fraction_signal(&self) -> Signal<f64> {
        self.traversal_fraction
    }

    /// Get the previous traversal signal (item that just exited traversal).
    /// Used by displacement to suppress CSS transition on the frame an item
    /// exits traversal, preventing the 250ms bounce artifact.
    pub fn previous_traversal_signal(&self) -> Signal<Option<(DragId, f64)>> {
        self.previous_traversal
    }

    /// Get the measure generation signal. Drop zones and sortable items read
    /// this in their rect-measurement effects so rects refresh on each drag
    /// start (accounting for browser zoom, scroll, or resize changes).
    pub fn measure_generation_signal(&self) -> Signal<u32> {
        self.measure_generation
    }

    /// Check if merge is enabled (SortableWithMerge collision strategy).
    pub fn is_merge_enabled(&self) -> bool {
        matches!(
            self.collision_strategy,
            CollisionStrategy::SortableWithMerge
        )
    }

    /// Check if gap displacement mode is active (items shift to create gaps).
    /// When false, indicator mode is active (items stay in place, line indicators show).
    pub fn gap_displacement(&self) -> bool {
        self.gap_displacement
    }

    /// Non-reactive check if a container zone accepts the currently dragged item.
    ///
    /// Uses `peek()` on both signals — suitable for use in memos that already
    /// subscribe to `active_signal()`. Returns `true` when there is no active drag
    /// or when the container zone is not found (backward compat).
    pub fn container_accepts_active(&self, container_id: &DragId) -> bool {
        let active = self.active.peek();
        let Some(active) = active.as_ref() else {
            return true; // No active drag
        };
        let zones = self.drop_zones.peek();
        let Some(zone) = zones.get(container_id) else {
            return true; // Container not found, allow
        };
        zone.accepts_data(&active.data)
    }

    /// Find the parent item that wraps a given inner container (non-reactive peek).
    ///
    /// Returns the item ID whose `inner_container_id` matches the given container ID.
    /// Used by displacement memos to detect when a drop target is inside a nested
    /// container, so sibling items in the parent can shift to accommodate growth.
    pub fn find_nested_parent(&self, inner_container_id: &DragId) -> Option<DragId> {
        let zones = self.drop_zones.peek();
        zones
            .values()
            .find(|z| z.inner_container_id.as_ref() == Some(inner_container_id))
            .map(|z| z.id.clone())
    }

    /// Check if a drop zone accepts the currently dragged item
    pub fn accepts(&self, zone_id: &DragId) -> bool {
        let active_guard = self.active.read();
        let Some(active) = active_guard.as_ref() else {
            return false;
        };
        let zones = self.drop_zones.peek();
        let Some(zone) = zones.get(zone_id) else {
            return false;
        };

        zone.accepts_data(&active.data)
    }

    // -------------------------------------------------------------------------
    // Announcement Methods (ARIA live region)
    // -------------------------------------------------------------------------

    /// Set the ARIA live region announcement text.
    ///
    /// Screen readers will announce the new text each time it changes.
    /// Called internally at drag lifecycle points; consumers can call this
    /// from their own event handlers to provide more specific announcements.
    pub fn set_announcement(&self, text: impl Into<String>) {
        let mut ann = self.announcement;
        *ann.write() = text.into();
    }

    /// Get the announcement signal for direct subscription in components.
    pub fn announcement_signal(&self) -> Signal<String> {
        self.announcement
    }

    /// Get the unique ARIA instructions element ID for this provider.
    ///
    /// Used by Draggable and SortableItem to set `aria-describedby` pointing
    /// to the correct instructions element when multiple providers exist.
    pub fn instructions_id(&self) -> Signal<String> {
        self.instructions_id
    }

    /// Get the scroll velocity signal for use in the auto-scroll RAF loop.
    pub fn scroll_velocity_signal(&self) -> Signal<f64> {
        self.scroll_velocity
    }

    /// Set the on_announce callback (called from DragContextProvider on mount).
    pub fn set_on_announce(&self, handler: EventHandler<AnnouncementEvent>) {
        *self.on_announce.write_unchecked() = Some(handler);
    }

    /// Dispatch a structured announcement event.
    ///
    /// Calls the `on_announce` callback (if set), then updates the ARIA live
    /// region with the event's default English text.
    pub fn dispatch_announcement(&self, event: AnnouncementEvent) {
        if let Some(handler) = self.on_announce.peek().as_ref() {
            handler.call(event.clone());
        }
        self.set_announcement(event.default_text());
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Extract sorted items in a container from the drop zones HashMap.
///
/// Returns item IDs in positional order (sorted by primary axis of container's
/// orientation). Excludes the dragged item if `exclude_id` is provided.
/// Also excludes container zones (where `id == container_id`) and item zones
/// with `inner_container_id` set (they delegate to nested containers).
///
/// This mirrors `sorted_items_for_container` in collision code but is a free
/// function for testability without Dioxus runtime.
pub fn sorted_items_in_container(
    zones: &HashMap<DragId, DropZoneState>,
    container_id: &DragId,
    exclude_id: Option<&DragId>,
) -> Vec<DragId> {
    let container_orientation = zones
        .get(container_id)
        .map(|z| z.orientation)
        .unwrap_or_default();

    let mut items: Vec<(&DragId, &DropZoneState)> = zones
        .iter()
        .filter(|(id, zone)| {
            zone.container_id == *container_id
                && *id != &zone.container_id // not a container zone
                && zone.inner_container_id.is_none() // not a nested wrapper
                && !exclude_id.is_some_and(|ex| *id == ex) // exclude dragged
        })
        .collect();

    items.sort_by(|(_, a), (_, b)| {
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

    items.into_iter().map(|(id, _)| id.clone()).collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::auto_scroll::*;
    use super::keyboard::*;
    use super::pointer::*;
    use super::*;
    use crate::types::DragType;

    // Test the type acceptance logic that end_drag() should use
    // We test DragState directly since DragContext requires Dioxus runtime

    #[test]
    fn test_should_reject_drop_when_type_not_accepted() {
        // Setup: Create a DragState with a drop zone that only accepts "document" type
        let mut state = DragState::new();

        // Register a drop zone that only accepts "document" type
        let zone_id = DragId::new("docs-folder");
        state.register_drop_zone(DropZoneState::new(
            zone_id.clone(),
            zone_id.clone(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![DragType::new("document")],
        ));

        // Start a drag with "video" type (not accepted by the zone)
        let drag_data = DragData::new("video-file", "video");
        state.active = Some(ActiveDrag::new(
            drag_data,
            DragId::new("video-file"),
            None,
            Position::new(50.0, 50.0),
        ));

        // Set the target to the docs-folder
        state.current_target = Some(DropLocation::IntoContainer {
            container_id: zone_id.clone(),
        });

        // Act: Check if drop should be accepted using the same logic end_drag should use
        let target_container_id = state.current_target.as_ref().unwrap().container_id();
        let zone = state.drop_zones.get(&target_container_id);
        let active = state.active.as_ref().unwrap();

        let should_accept = match zone {
            Some(z) => z.accepts_data(&active.data),
            None => true, // If zone not found, allow drop (backwards compatibility)
        };

        // Assert: Should NOT accept because the zone doesn't accept "video" type
        assert!(
            !should_accept,
            "Drop should be rejected when target zone doesn't accept the dragged type"
        );
    }

    #[test]
    fn test_should_accept_drop_when_type_matches() {
        // Setup: Create a DragState with a drop zone that accepts "document" type
        let mut state = DragState::new();

        // Register a drop zone that accepts "document" type
        let zone_id = DragId::new("docs-folder");
        state.register_drop_zone(DropZoneState::new(
            zone_id.clone(),
            zone_id.clone(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![DragType::new("document")],
        ));

        // Start a drag with "document" type (accepted by the zone)
        let drag_data = DragData::new("doc-file", "document");
        state.active = Some(ActiveDrag::new(
            drag_data,
            DragId::new("doc-file"),
            None,
            Position::new(50.0, 50.0),
        ));

        // Set the target to the docs-folder
        state.current_target = Some(DropLocation::IntoContainer {
            container_id: zone_id.clone(),
        });

        // Act: Check if drop should be accepted
        let target_container_id = state.current_target.as_ref().unwrap().container_id();
        let zone = state.drop_zones.get(&target_container_id);
        let active = state.active.as_ref().unwrap();

        let should_accept = match zone {
            Some(z) => z.accepts_data(&active.data),
            None => true,
        };

        // Assert: Should accept because the zone accepts "document" type
        assert!(
            should_accept,
            "Drop should be accepted when target zone accepts the dragged type"
        );
    }

    #[test]
    fn test_should_accept_drop_when_zone_accepts_all() {
        // Setup: Create a DragState with a drop zone that accepts all types (empty accepts)
        let mut state = DragState::new();

        // Register a drop zone that accepts all types
        let zone_id = DragId::new("trash");
        state.register_drop_zone(DropZoneState::new(
            zone_id.clone(),
            zone_id.clone(),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![], // Empty = accepts all
        ));

        // Start a drag with any type
        let drag_data = DragData::new("some-file", "random-type");
        state.active = Some(ActiveDrag::new(
            drag_data,
            DragId::new("some-file"),
            None,
            Position::new(50.0, 50.0),
        ));

        // Set the target to the trash
        state.current_target = Some(DropLocation::IntoContainer {
            container_id: zone_id.clone(),
        });

        // Act: Check if drop should be accepted
        let target_container_id = state.current_target.as_ref().unwrap().container_id();
        let zone = state.drop_zones.get(&target_container_id);
        let active = state.active.as_ref().unwrap();

        let should_accept = match zone {
            Some(z) => z.accepts_data(&active.data),
            None => true,
        };

        // Assert: Should accept because zone with empty accepts accepts all types
        assert!(
            should_accept,
            "Drop should be accepted when target zone accepts all types (empty accepts)"
        );
    }

    #[test]
    fn test_drop_zone_state_accepts_type() {
        let zone = DropZoneState::new(
            "zone1",
            "container1",
            Rect::default(),
            vec![DragType::new("task"), DragType::new("card")],
        );

        assert!(zone.accepts_type(&DragType::new("task")));
        assert!(zone.accepts_type(&DragType::new("card")));
        assert!(!zone.accepts_type(&DragType::new("other")));
    }

    #[test]
    fn test_drop_zone_state_accepts_all_when_empty() {
        let zone = DropZoneState::new("zone1", "container1", Rect::default(), vec![]);

        assert!(zone.accepts_type(&DragType::new("anything")));
        assert!(zone.accepts_type(&DragType::new("whatever")));
    }

    #[test]
    fn test_active_drag_update_position() {
        let data = DragData::new("item1", "task");
        let mut drag = ActiveDrag::new(data, "source", None, Position::new(100.0, 100.0));

        assert_eq!(drag.delta.x, 0.0);
        assert_eq!(drag.delta.y, 0.0);

        drag.update_position(Position::new(150.0, 120.0));

        assert_eq!(drag.current_position.x, 150.0);
        assert_eq!(drag.current_position.y, 120.0);
        assert_eq!(drag.delta.x, 50.0);
        assert_eq!(drag.delta.y, 20.0);
    }

    #[test]
    fn test_drag_state_is_dragging() {
        let mut state = DragState::new();
        assert!(!state.is_dragging());

        let data = DragData::new("item1", "task");
        state.active = Some(ActiveDrag::new(data, "source", None, Position::default()));

        assert!(state.is_dragging());
    }

    #[test]
    fn test_drag_state_is_dragging_id() {
        let mut state = DragState::new();
        let item_id = DragId::new("item1");

        assert!(!state.is_dragging_id(&item_id));

        let data = DragData::new("item1", "task");
        state.active = Some(ActiveDrag::new(data, "source", None, Position::default()));

        assert!(state.is_dragging_id(&item_id));
        assert!(!state.is_dragging_id(&DragId::new("other")));
    }

    #[test]
    fn test_drag_state_register_unregister_drop_zone() {
        let mut state = DragState::new();
        let zone_id = DragId::new("zone1");

        let zone = DropZoneState::new("zone1", "container1", Rect::default(), vec![]);
        state.register_drop_zone(zone);

        assert!(state.drop_zones.contains_key(&zone_id));

        state.unregister_drop_zone(&zone_id);
        assert!(!state.drop_zones.contains_key(&zone_id));
    }

    // =========================================================================
    // Multi-type acceptance tests (TDD)
    // =========================================================================

    #[test]
    fn test_drop_zone_accepts_any_type_from_multi_type_item() {
        // A drop zone that accepts "image" should accept an item with types ["sortable", "image"]
        let zone = DropZoneState::new(
            "zone1",
            "container1",
            Rect::default(),
            vec![DragType::new("image")],
        );

        // Item has multiple types: "sortable" AND "image"
        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        // Zone accepts "image", item has "image" - should accept
        assert!(
            zone.accepts_data(&data),
            "Zone should accept item with matching type"
        );
    }

    #[test]
    fn test_drop_zone_rejects_when_no_types_match() {
        // A drop zone that accepts only "document" should reject an item with types ["sortable", "image"]
        let zone = DropZoneState::new(
            "zone1",
            "container1",
            Rect::default(),
            vec![DragType::new("document")],
        );

        // Item has "sortable" and "image", but NOT "document"
        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        assert!(
            !zone.accepts_data(&data),
            "Zone should reject item with no matching type"
        );
    }

    #[test]
    fn test_drop_zone_accepts_all_when_empty_accepts() {
        // A drop zone with empty accepts should accept any item
        let zone = DropZoneState::new(
            "zone1",
            "container1",
            Rect::default(),
            vec![], // Empty = accepts all
        );

        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("whatever"), DragType::new("random")],
        );

        assert!(
            zone.accepts_data(&data),
            "Zone with empty accepts should accept all"
        );
    }

    #[test]
    fn test_cancel_drag_clears_all_state() {
        // cancel_drag should clear active drag, current target, and any pending state
        let mut state = DragState::new();

        // Setup: active drag with a target
        let data = DragData::new("item1", "task");
        state.active = Some(ActiveDrag::new(
            data,
            "source",
            Some(DragId::new("container")),
            Position::new(50.0, 50.0),
        ));
        state.current_target = Some(DropLocation::AtIndex {
            container_id: DragId::new("container"),
            index: 0,
        });

        // Verify pre-conditions
        assert!(state.is_dragging(), "Should be dragging before cancel");
        assert!(
            state.current_target.is_some(),
            "Should have target before cancel"
        );

        // Act: simulate cancel by clearing state (mirrors DragContext::cancel_drag)
        state.active = None;
        state.current_target = None;

        // Assert: all state cleared
        assert!(!state.is_dragging(), "Should not be dragging after cancel");
        assert!(
            state.current_target.is_none(),
            "Target should be None after cancel"
        );
    }

    #[test]
    fn test_drop_zone_accepts_sortable_item_with_filter_type() {
        // This is the key use case: a sortable list that filters by content type
        // Zone accepts: images and videos (for a "media" folder)
        let zone = DropZoneState::new(
            "media-zone",
            "media-container",
            Rect::default(),
            vec![DragType::new("image"), DragType::new("video")],
        );

        // Item is sortable AND is an image
        let image_item = DragData::with_types(
            "img-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        // Item is sortable AND is a document (not accepted)
        let doc_item = DragData::with_types(
            "doc-1",
            vec![DragType::new("sortable"), DragType::new("document")],
        );

        assert!(
            zone.accepts_data(&image_item),
            "Zone should accept sortable image"
        );
        assert!(
            !zone.accepts_data(&doc_item),
            "Zone should reject sortable document"
        );
    }

    // =========================================================================
    // Auto-scroll velocity tests
    // =========================================================================

    #[test]
    fn test_scroll_velocity_center_of_viewport_is_zero() {
        // Pointer in the middle of viewport — no scroll
        assert_eq!(scroll_velocity_for(400.0, 800.0), 0.0);
        assert_eq!(scroll_velocity_for(300.0, 800.0), 0.0);
        assert_eq!(scroll_velocity_for(500.0, 800.0), 0.0);
    }

    #[test]
    fn test_scroll_velocity_near_top_edge_scrolls_up() {
        // Pointer near top edge — negative velocity (scroll up)
        let vel = scroll_velocity_for(30.0, 800.0);
        assert!(vel < 0.0, "Near top should scroll up (negative): {vel}");
        // At 30px from top, distance=30, ratio=0.5, velocity=-7.5
        assert!((vel - (-7.5)).abs() < 0.01);
    }

    #[test]
    fn test_scroll_velocity_near_bottom_edge_scrolls_down() {
        // Pointer near bottom edge — positive velocity (scroll down)
        let vel = scroll_velocity_for(770.0, 800.0);
        assert!(
            vel > 0.0,
            "Near bottom should scroll down (positive): {vel}"
        );
        // At 770px with viewport 800, distance=30, ratio=0.5, velocity=7.5
        assert!((vel - 7.5).abs() < 0.01);
    }

    #[test]
    fn test_scroll_velocity_at_viewport_edge_is_max() {
        // Pointer at very top — max upward velocity
        let vel = scroll_velocity_for(0.0, 800.0);
        assert!((vel - (-SCROLL_MAX_PX)).abs() < 0.01);

        // Pointer at very bottom — max downward velocity
        let vel = scroll_velocity_for(800.0, 800.0);
        assert!((vel - SCROLL_MAX_PX).abs() < 0.01);
    }

    #[test]
    fn test_scroll_velocity_just_outside_edge_zone() {
        // Pointer at exactly the edge zone boundary — zero velocity
        assert_eq!(scroll_velocity_for(60.0, 800.0), 0.0);
        assert_eq!(scroll_velocity_for(740.0, 800.0), 0.0);
    }

    #[test]
    fn test_scroll_velocity_scales_linearly() {
        // Velocity should scale linearly with distance into the edge zone
        let vel_quarter = scroll_velocity_for(45.0, 800.0); // 15px in, ratio=0.25
        let vel_half = scroll_velocity_for(30.0, 800.0); // 30px in, ratio=0.5
        let vel_full = scroll_velocity_for(0.0, 800.0); // 60px in, ratio=1.0

        // Check ratios
        assert!((vel_half / vel_full - 0.5).abs() < 0.01);
        assert!((vel_quarter / vel_full - 0.25).abs() < 0.01);
    }

    // =========================================================================
    // Traversal projection tests
    // =========================================================================

    #[test]
    fn test_projected_traversal_uses_displaced_axis_start_for_reorder() {
        // Full list: [a, dragged, c, d]
        // Dragged source slot = 1, target index (filtered AtIndex) = 2.
        // Item `c` at full index 2 displaces up by dragged size.
        let base_start = 120.0;
        let item_size = 60.0;
        let dragged_size = 60.0;
        let start = DragContext::projected_traversal_axis_start(
            base_start,
            2,                // c full index
            Some(1),          // dragged source slot
            Some((2, false)), // AtIndex after c in filtered space
            true,
            item_size,
            dragged_size,
        );
        assert_eq!(start, 60.0);

        // Pointer at 70 is outside base rect [120,180) but inside projected [60,120).
        // Traversal should follow the projected rect to avoid reversal snap.
        assert!(70.0 < base_start);
        assert!(70.0 >= start && 70.0 < start + item_size);
    }

    #[test]
    fn test_projected_traversal_without_target_stays_at_base_position() {
        let base_start = 120.0;
        let start = DragContext::projected_traversal_axis_start(
            base_start,
            2,
            Some(1),
            None,  // no target in any container yet
            false, // has_any_target = false
            60.0,
            60.0,
        );
        assert_eq!(start, base_start);
    }

    #[test]
    fn test_projected_traversal_drag_out_collapse_uses_shared_projection() {
        // Drag out of source container after a target exists elsewhere:
        // items at and after source slot collapse up by dragged size.
        let start_after_source = DragContext::projected_traversal_axis_start(
            120.0,
            2,
            Some(1),
            None,
            true, // target exists in another container
            60.0,
            60.0,
        );
        assert_eq!(start_after_source, 60.0);

        // Item before source slot should not move.
        let start_before_source =
            DragContext::projected_traversal_axis_start(0.0, 0, Some(1), None, true, 60.0, 60.0);
        assert_eq!(start_before_source, 0.0);
    }

    #[test]
    fn test_projected_target_prefers_pending_candidate() {
        let committed = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 3,
        });
        let pending = Some((
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            },
            42.0,
        ));

        // now=0.0 sentinel in tests should still prefer pending immediately.
        let projected = projected_target_from(committed, pending, 0.0).unwrap();
        assert_eq!(
            projected,
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 1,
            }
        );
    }

    #[test]
    fn test_projected_target_falls_back_to_committed() {
        let committed = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });
        let projected = projected_target_from(committed.clone(), None, 100.0);
        assert_eq!(projected, committed);
    }

    #[test]
    fn test_projected_target_immature_pending_uses_committed() {
        let committed = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });
        let pending = Some((
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 4,
            },
            100.0,
        ));

        let projected = projected_target_from(committed.clone(), pending, 110.0);
        assert_eq!(projected, committed);
    }

    #[test]
    fn test_projected_target_mature_pending_overrides_committed() {
        let committed = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });
        let pending = Some((
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 4,
            },
            100.0,
        ));

        let projected = projected_target_from(committed, pending, 130.0);
        assert_eq!(
            projected,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 4,
            })
        );
    }

    // =========================================================================
    // Keyboard drag helper tests (sorted_items_in_container)
    // =========================================================================

    #[test]
    fn test_sorted_items_in_container_basic_order() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");

        // Container zone
        zones.insert(
            cid.clone(),
            DropZoneState::new(
                cid.clone(),
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        // Three items at different Y positions
        zones.insert(
            DragId::new("c"),
            DropZoneState::new("c", cid.clone(), Rect::new(0.0, 200.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", cid.clone(), Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("b"),
            DropZoneState::new("b", cid.clone(), Rect::new(0.0, 100.0, 300.0, 60.0), vec![]),
        );

        let result = sorted_items_in_container(&zones, &cid, None);
        assert_eq!(
            result,
            vec![DragId::new("a"), DragId::new("b"), DragId::new("c")]
        );
    }

    #[test]
    fn test_sorted_items_in_container_excludes_dragged() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");

        zones.insert(
            cid.clone(),
            DropZoneState::new(
                cid.clone(),
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", cid.clone(), Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("b"),
            DropZoneState::new("b", cid.clone(), Rect::new(0.0, 100.0, 300.0, 60.0), vec![]),
        );

        let result = sorted_items_in_container(&zones, &cid, Some(&DragId::new("a")));
        assert_eq!(result, vec![DragId::new("b")]);
    }

    #[test]
    fn test_sorted_items_in_container_excludes_nested_wrappers() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");

        zones.insert(
            cid.clone(),
            DropZoneState::new(
                cid.clone(),
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", cid.clone(), Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );

        // Nested wrapper item (has inner_container_id set)
        let mut wrapper = DropZoneState::new(
            "group-1",
            cid.clone(),
            Rect::new(0.0, 100.0, 300.0, 120.0),
            vec![],
        );
        wrapper.inner_container_id = Some(DragId::new("group-1-container"));
        zones.insert(DragId::new("group-1"), wrapper);

        let result = sorted_items_in_container(&zones, &cid, None);
        // Should only include "a", not the nested wrapper "group-1"
        assert_eq!(result, vec![DragId::new("a")]);
    }

    #[test]
    fn test_sorted_items_in_container_excludes_other_containers() {
        let mut zones = HashMap::new();
        let cid1 = DragId::new("list-1");
        let cid2 = DragId::new("list-2");

        zones.insert(
            cid1.clone(),
            DropZoneState::new(
                cid1.clone(),
                cid1.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );
        zones.insert(
            cid2.clone(),
            DropZoneState::new(
                cid2.clone(),
                cid2.clone(),
                Rect::new(400.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", cid1.clone(), Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("x"),
            DropZoneState::new(
                "x",
                cid2.clone(),
                Rect::new(400.0, 0.0, 300.0, 60.0),
                vec![],
            ),
        );

        let result = sorted_items_in_container(&zones, &cid1, None);
        assert_eq!(result, vec![DragId::new("a")]);

        let result = sorted_items_in_container(&zones, &cid2, None);
        assert_eq!(result, vec![DragId::new("x")]);
    }

    #[test]
    fn test_sorted_items_in_container_empty() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");
        zones.insert(
            cid.clone(),
            DropZoneState::new(
                cid.clone(),
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        let result = sorted_items_in_container(&zones, &cid, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_sorted_items_in_container_horizontal() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");

        // Container with horizontal orientation
        let mut container = DropZoneState::new(
            cid.clone(),
            cid.clone(),
            Rect::new(0.0, 0.0, 500.0, 60.0),
            vec![],
        );
        container.orientation = Orientation::Horizontal;
        zones.insert(cid.clone(), container);

        // Items at different X positions
        let mut item_b =
            DropZoneState::new("b", cid.clone(), Rect::new(200.0, 0.0, 60.0, 60.0), vec![]);
        item_b.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("b"), item_b);

        let mut item_a =
            DropZoneState::new("a", cid.clone(), Rect::new(0.0, 0.0, 60.0, 60.0), vec![]);
        item_a.orientation = Orientation::Horizontal;
        zones.insert(DragId::new("a"), item_a);

        let result = sorted_items_in_container(&zones, &cid, None);
        assert_eq!(result, vec![DragId::new("a"), DragId::new("b")]);
    }

    // =========================================================================
    // Merge toggle tests (toggle_merge_target)
    // =========================================================================

    #[test]
    fn test_toggle_merge_at_index_to_into_item() {
        let cid = DragId::new("list");
        let target_item = DragId::new("item-2");
        let loc = DropLocation::AtIndex {
            container_id: cid.clone(),
            index: 1,
        };

        let toggled = toggle_merge_target(&loc, &cid, &target_item, 1).unwrap();
        assert_eq!(
            toggled,
            DropLocation::IntoItem {
                container_id: cid,
                item_id: DragId::new("item-2"),
            }
        );
    }

    #[test]
    fn test_toggle_merge_at_index_different_index_to_into_item() {
        let cid = DragId::new("list");
        let target_item = DragId::new("item-3");
        let loc = DropLocation::AtIndex {
            container_id: cid.clone(),
            index: 2,
        };

        let toggled = toggle_merge_target(&loc, &cid, &target_item, 2).unwrap();
        assert_eq!(
            toggled,
            DropLocation::IntoItem {
                container_id: cid,
                item_id: DragId::new("item-3"),
            }
        );
    }

    #[test]
    fn test_toggle_merge_into_item_to_at_index() {
        let cid = DragId::new("list");
        let target_item = DragId::new("item-2");
        let loc = DropLocation::IntoItem {
            container_id: cid.clone(),
            item_id: DragId::new("item-2"),
        };

        let toggled = toggle_merge_target(&loc, &cid, &target_item, 1).unwrap();
        assert_eq!(
            toggled,
            DropLocation::AtIndex {
                container_id: cid,
                index: 1,
            }
        );
    }

    #[test]
    fn test_toggle_merge_roundtrip() {
        let cid = DragId::new("list");
        let target_item = DragId::new("item-2");
        let original = DropLocation::AtIndex {
            container_id: cid.clone(),
            index: 1,
        };

        let into = toggle_merge_target(&original, &cid, &target_item, 1).unwrap();
        let back = toggle_merge_target(&into, &cid, &target_item, 1).unwrap();

        // Should be AtIndex again (same container + index)
        assert_eq!(
            back,
            DropLocation::AtIndex {
                container_id: cid,
                index: 1,
            }
        );
    }

    #[test]
    fn test_toggle_merge_noop_for_into_container() {
        let cid = DragId::new("list");
        let target_item = DragId::new("item-1");
        let loc = DropLocation::IntoContainer {
            container_id: cid.clone(),
        };

        assert!(toggle_merge_target(&loc, &cid, &target_item, 0).is_none());
    }

    // =========================================================================
    // Nested container helper tests
    // =========================================================================

    #[test]
    fn test_find_nested_exit_finds_parent() {
        let mut zones = HashMap::new();
        let parent_cid = DragId::new("workout");
        let inner_cid = DragId::new("superset-1-container");

        // Parent container
        zones.insert(
            parent_cid.clone(),
            DropZoneState::new(
                parent_cid.clone(),
                parent_cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        // Group wrapper item in parent (has inner_container_id)
        let mut wrapper = DropZoneState::new(
            "superset-1",
            parent_cid.clone(),
            Rect::new(0.0, 100.0, 300.0, 200.0),
            vec![],
        );
        wrapper.inner_container_id = Some(inner_cid.clone());
        zones.insert(DragId::new("superset-1"), wrapper);

        // Inner container
        zones.insert(
            inner_cid.clone(),
            DropZoneState::new(
                inner_cid.clone(),
                inner_cid.clone(),
                Rect::new(0.0, 100.0, 300.0, 200.0),
                vec![],
            ),
        );

        let result = find_nested_exit(&zones, &inner_cid);
        assert!(result.is_some());
        let (parent_container, parent_item) = result.unwrap();
        assert_eq!(parent_container, parent_cid);
        assert_eq!(parent_item, DragId::new("superset-1"));
    }

    #[test]
    fn test_find_nested_exit_not_nested() {
        let mut zones = HashMap::new();
        let cid = DragId::new("list");
        zones.insert(
            cid.clone(),
            DropZoneState::new(
                cid.clone(),
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        // Top-level container — no nested exit
        assert!(find_nested_exit(&zones, &cid).is_none());
    }

    #[test]
    fn test_find_inner_container_for_group_item() {
        let mut zones = HashMap::new();
        let cid = DragId::new("workout");
        let inner_cid = DragId::new("superset-1-container");

        let mut wrapper = DropZoneState::new(
            "superset-1",
            cid.clone(),
            Rect::new(0.0, 100.0, 300.0, 200.0),
            vec![],
        );
        wrapper.inner_container_id = Some(inner_cid.clone());
        zones.insert(DragId::new("superset-1"), wrapper);

        let result = find_inner_container(&zones, &DragId::new("superset-1"));
        assert_eq!(result, Some(inner_cid));
    }

    #[test]
    fn test_find_inner_container_for_regular_item() {
        let mut zones = HashMap::new();
        let cid = DragId::new("workout");
        zones.insert(
            DragId::new("item-1"),
            DropZoneState::new("item-1", cid, Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );

        let result = find_inner_container(&zones, &DragId::new("item-1"));
        assert!(result.is_none());
    }

    // =========================================================================
    // grab_offset tests
    // =========================================================================

    #[test]
    fn test_grab_offset_computed() {
        // Grab at (150, 120) on zone at (100, 100) → offset (50, 20)
        let data = DragData::new("item1", "task");
        let grab_pos = Position::new(150.0, 120.0);
        let zone_rect = Rect::new(100.0, 100.0, 200.0, 60.0);

        // Simulate grab_offset calculation from start_drag
        let grab_offset = Position {
            x: grab_pos.x - zone_rect.x,
            y: grab_pos.y - zone_rect.y,
        };

        let mut drag = ActiveDrag::new(data, "source", None, grab_pos);
        drag.grab_offset = grab_offset;

        assert!((drag.grab_offset.x - 50.0).abs() < f64::EPSILON);
        assert!((drag.grab_offset.y - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_grab_offset_no_zone() {
        // Source zone not found → offset defaults to (0, 0)
        let data = DragData::new("item1", "task");
        let drag = ActiveDrag::new(data, "source", None, Position::new(150.0, 120.0));

        assert!((drag.grab_offset.x).abs() < f64::EPSILON);
        assert!((drag.grab_offset.y).abs() < f64::EPSILON);
    }

    // =========================================================================
    // F-004: Unique ARIA instruction ID tests
    // =========================================================================

    #[test]
    fn test_provider_counter_increments() {
        // Each call to fetch_add should produce a unique value
        let a = PROVIDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let b = PROVIDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        assert_ne!(a, b);
        assert_eq!(b, a + 1);
    }

    #[test]
    fn test_instructions_id_format() {
        // Verify the format matches what child components expect
        let id = PROVIDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let instructions_id = format!("dxdnd-drag-instructions-{}", id);
        assert!(instructions_id.starts_with("dxdnd-drag-instructions-"));
        // Should be a valid HTML id (no spaces, starts with letter)
        assert!(!instructions_id.contains(' '));
    }

    // =========================================================================
    // F-007: Announcement total count tests
    // =========================================================================

    #[test]
    fn test_container_item_count_for_announcements() {
        // Verify sorted_items_in_container returns correct count for total
        // This is the function used to compute announcement totals
        let mut zones = HashMap::new();
        let cid = DragId::new("list");

        zones.insert(
            cid.clone(),
            DropZoneState::new(
                "list",
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );
        zones.insert(
            DragId::new("a"),
            DropZoneState::new("a", cid.clone(), Rect::new(0.0, 0.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("b"),
            DropZoneState::new("b", cid.clone(), Rect::new(0.0, 60.0, 300.0, 60.0), vec![]),
        );
        zones.insert(
            DragId::new("c"),
            DropZoneState::new("c", cid.clone(), Rect::new(0.0, 120.0, 300.0, 60.0), vec![]),
        );

        // When dragging item "a", sorted_items excludes it
        let items_without_dragged =
            sorted_items_in_container(&zones, &cid, Some(&DragId::new("a")));
        assert_eq!(items_without_dragged.len(), 2); // b, c

        // The total for announcements should include the dragged item:
        // items_without_dragged.len() + 1 = 3 (the real list size)
        let total = items_without_dragged.len() + 1;
        assert_eq!(total, 3);
    }

    #[test]
    fn test_resolve_drop_index_empty_slice_always_returns_index() {
        // This test documents the bug that F-007 fixes:
        // resolve_drop_index(&[]) with AtIndex returns the index directly,
        // which is NOT the total item count.
        let location = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 0,
        };
        // With empty slice, this always returns the index value (0 here)
        assert_eq!(location.resolve_drop_index(&[]), 0);

        let location2 = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };
        // Returns 2, not the actual number of items in the container
        assert_eq!(location2.resolve_drop_index(&[]), 2);
    }

    // =========================================================================
    // Auto-scroll velocity tests (scroll_velocity_for pure function)
    // =========================================================================

    #[test]
    fn test_scroll_velocity_middle_of_viewport() {
        // Pointer in the middle of the viewport — no scrolling
        let v = scroll_velocity_for(400.0, 800.0);
        assert!((v - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_just_outside_top_edge_zone() {
        // Pointer at exactly SCROLL_EDGE_PX — boundary of top zone, no scroll
        let v = scroll_velocity_for(SCROLL_EDGE_PX, 800.0);
        assert!((v - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_just_outside_bottom_edge_zone() {
        // Pointer at exactly viewport_height - SCROLL_EDGE_PX — boundary, no scroll
        let v = scroll_velocity_for(800.0 - SCROLL_EDGE_PX, 800.0);
        assert!((v - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_at_very_top() {
        // Pointer at y=0 — maximum upward scroll speed
        let v = scroll_velocity_for(0.0, 800.0);
        assert!((v - (-SCROLL_MAX_PX)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_at_very_bottom() {
        // Pointer at y=viewport_height — maximum downward scroll speed
        let v = scroll_velocity_for(800.0, 800.0);
        assert!((v - SCROLL_MAX_PX).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_near_top_linear_scaling() {
        // Pointer halfway into top edge zone — half of max speed
        let v = scroll_velocity_for(SCROLL_EDGE_PX / 2.0, 800.0);
        let expected = -SCROLL_MAX_PX * 0.5;
        assert!((v - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_near_bottom_linear_scaling() {
        // Pointer halfway into bottom edge zone — half of max speed
        let viewport = 800.0;
        let pointer_y = viewport - SCROLL_EDGE_PX / 2.0;
        let v = scroll_velocity_for(pointer_y, viewport);
        let expected = SCROLL_MAX_PX * 0.5;
        assert!((v - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_beyond_viewport_top_clamped() {
        // Pointer above viewport (negative y) — clamped to max upward speed
        let v = scroll_velocity_for(-100.0, 800.0);
        assert!((v - (-SCROLL_MAX_PX)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_beyond_viewport_bottom_clamped() {
        // Pointer below viewport — clamped to max downward speed
        let v = scroll_velocity_for(900.0, 800.0);
        assert!((v - SCROLL_MAX_PX).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_velocity_small_viewport() {
        // Small viewport where edge zones overlap — top zone takes priority
        let v = scroll_velocity_for(30.0, 80.0);
        // 30 < SCROLL_EDGE_PX (60), so top zone logic applies
        let distance = SCROLL_EDGE_PX - 30.0;
        let ratio = (distance / SCROLL_EDGE_PX).min(1.0);
        let expected = -SCROLL_MAX_PX * ratio;
        assert!((v - expected).abs() < f64::EPSILON);
    }

    // =========================================================================
    // Keyboard helper free function tests
    // =========================================================================

    #[test]
    fn test_toggle_merge_target_at_index_to_into_item() {
        let location = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };
        let result =
            toggle_merge_target(&location, &DragId::new("list"), &DragId::new("item-2"), 2);
        assert_eq!(
            result,
            Some(DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-2"),
            })
        );
    }

    #[test]
    fn test_toggle_merge_target_into_item_to_at_index() {
        let location = DropLocation::IntoItem {
            container_id: DragId::new("list"),
            item_id: DragId::new("item-2"),
        };
        let result =
            toggle_merge_target(&location, &DragId::new("list"), &DragId::new("item-2"), 3);
        assert_eq!(
            result,
            Some(DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 3,
            })
        );
    }

    #[test]
    fn test_toggle_merge_target_into_container_returns_none() {
        let location = DropLocation::IntoContainer {
            container_id: DragId::new("list"),
        };
        let result =
            toggle_merge_target(&location, &DragId::new("list"), &DragId::new("item-1"), 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_nested_exit_with_inner_container() {
        let mut zones = HashMap::new();
        let parent_cid = DragId::new("parent-list");
        let inner_cid = DragId::new("group-1-container");

        // Parent container zone
        zones.insert(
            parent_cid.clone(),
            DropZoneState::new(
                "parent-list",
                parent_cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        // Group wrapper item in parent container with inner_container_id
        let mut wrapper_zone = DropZoneState::new(
            "group-1",
            parent_cid.clone(),
            Rect::new(0.0, 100.0, 300.0, 200.0),
            vec![],
        );
        wrapper_zone.inner_container_id = Some(inner_cid.clone());
        zones.insert(DragId::new("group-1"), wrapper_zone);

        // Inner container zone
        zones.insert(
            inner_cid.clone(),
            DropZoneState::new(
                "group-1-container",
                inner_cid.clone(),
                Rect::new(10.0, 110.0, 280.0, 180.0),
                vec![],
            ),
        );

        let result = find_nested_exit(&zones, &inner_cid);
        assert_eq!(result, Some((parent_cid, DragId::new("group-1"))));
    }

    #[test]
    fn test_find_nested_exit_no_parent() {
        let mut zones = HashMap::new();
        let cid = DragId::new("top-level");
        zones.insert(
            cid.clone(),
            DropZoneState::new(
                "top-level",
                cid.clone(),
                Rect::new(0.0, 0.0, 300.0, 500.0),
                vec![],
            ),
        );

        let result = find_nested_exit(&zones, &DragId::new("nonexistent-inner"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_inner_container_exists() {
        let mut zones = HashMap::new();
        let inner_cid = DragId::new("group-1-container");
        let mut zone = DropZoneState::new(
            "group-1",
            DragId::new("list"),
            Rect::new(0.0, 0.0, 300.0, 200.0),
            vec![],
        );
        zone.inner_container_id = Some(inner_cid.clone());
        zones.insert(DragId::new("group-1"), zone);

        let result = find_inner_container(&zones, &DragId::new("group-1"));
        assert_eq!(result, Some(inner_cid));
    }

    #[test]
    fn test_find_inner_container_not_a_group() {
        let mut zones = HashMap::new();
        zones.insert(
            DragId::new("item-1"),
            DropZoneState::new(
                "item-1",
                DragId::new("list"),
                Rect::new(0.0, 0.0, 300.0, 60.0),
                vec![],
            ),
        );

        let result = find_inner_container(&zones, &DragId::new("item-1"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_inner_container_missing_item() {
        let zones = HashMap::new();
        let result = find_inner_container(&zones, &DragId::new("nonexistent"));
        assert_eq!(result, None);
    }

    // =========================================================================
    // Projected target selection tests (projected_target_from)
    // =========================================================================

    #[test]
    fn test_projected_target_no_pending_no_committed() {
        let result = projected_target_from(None, None, 100.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_projected_target_no_pending_has_committed() {
        let committed = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };
        let result = projected_target_from(Some(committed.clone()), None, 100.0);
        assert_eq!(result, Some(committed));
    }

    #[test]
    fn test_projected_target_pending_matured() {
        let committed = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };
        let pending = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 3,
        };
        // Pending started at 50ms, now is 100ms → elapsed 50ms >= PROJECTED_PENDING_MIN_MS (18ms)
        let result = projected_target_from(Some(committed), Some((pending.clone(), 50.0)), 100.0);
        assert_eq!(result, Some(pending));
    }

    #[test]
    fn test_projected_target_pending_not_matured() {
        let committed = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        };
        let pending = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 3,
        };
        // Pending started at 95ms, now is 100ms → elapsed 5ms < PROJECTED_PENDING_MIN_MS (18ms)
        let result = projected_target_from(Some(committed.clone()), Some((pending, 95.0)), 100.0);
        assert_eq!(result, Some(committed));
    }

    #[test]
    fn test_projected_target_pending_not_matured_no_committed() {
        let pending = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 3,
        };
        // Pending not matured, no committed → returns None
        let result = projected_target_from(None, Some((pending, 99.0)), 100.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_projected_target_test_sentinel_immediate() {
        // When now_ms == 0.0 (test sentinel), pending is returned immediately
        let pending = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 1,
        };
        let result = projected_target_from(None, Some((pending.clone(), 0.0)), 0.0);
        assert_eq!(result, Some(pending));
    }
}
