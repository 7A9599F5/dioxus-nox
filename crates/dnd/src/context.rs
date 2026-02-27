//! DragContext provider and state management
//!
//! This module contains the core state types for drag-and-drop operations:
//! - `DropZoneState` - State of a registered drop zone
//! - `ActiveDrag` - Information about the current active drag
//! - `DragState` - Global drag state that lives in context
//! - `DragContext` - The global drag-and-drop context with signal ownership
//! - `DragContextProvider` - Provider component for the drag context

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

/// Monotonic counter for generating unique ARIA instruction IDs across
/// multiple DragContextProvider instances on the same page.
static PROVIDER_COUNTER: AtomicU32 = AtomicU32::new(0);

use crate::collision::CollisionStrategy;
#[cfg(target_arch = "wasm32")]
use crate::patterns::sortable::item::NextAnimationFrame;
use crate::sortable_projection::{compute_displacement_offset, to_filtered_index};
use crate::types::{
    AnnouncementEvent, DragData, DragId, DragType, DropEvent, DropLocation, Orientation, Position,
    Rect,
};
use crate::utils::{extract_attribute, filter_class_style};

/// Hysteresis delay in milliseconds. Target changes must persist for this
/// duration before being committed, filtering boundary oscillation at ~60fps.
const HYSTERESIS_MS: f64 = 50.0;

/// Minimum age (ms) for a pending hysteresis candidate to drive projected
/// displacement/traversal visuals.
///
/// Requiring ~one frame of stability suppresses edge flicker where collision
/// rapidly alternates across a boundary (A↔B↔A), which would otherwise cause
/// projected geometry to jump between targets every frame.
const PROJECTED_PENDING_MIN_MS: f64 = 18.0;

/// Minimum squared distance (px²) from start position before collision
/// detection activates. Prevents false targets from browser-synthesized
/// pointermove events at drag start. 3px threshold = 9.0 squared.
const ACTIVATION_DISTANCE_SQ: f64 = 9.0;

/// Duration (ms) that the snap window stays open after an item exits traversal.
/// Must exceed HYSTERESIS_MS (50ms) + one render frame (~16ms) to cover the gap
/// between traversal exit and hysteresis-delayed target commit.
const SNAP_WINDOW_MS: f64 = 80.0;

/// Minimum interval between drag-time layout invalidations.
///
/// During active drag, geometry may change from auto-scroll, viewport motion,
/// or renderer/layout adjustments. This throttles refresh generation bumps so
/// rect re-measurement stays responsive without spawning redundant work.
const MEASURE_REFRESH_THROTTLE_MS: f64 = 80.0;

/// Distance from viewport edge (px) where auto-scroll triggers.
#[cfg(any(target_arch = "wasm32", test))]
const SCROLL_EDGE_PX: f64 = 60.0;

/// Maximum scroll speed per animation frame (px/frame).
#[cfg(any(target_arch = "wasm32", test))]
const SCROLL_MAX_PX: f64 = 15.0;

/// Pure computation of scroll velocity given pointer Y and viewport height.
///
/// Returns velocity in px/frame: negative = scroll up, positive = scroll down,
/// zero = no scroll. Velocity scales linearly with distance into the edge zone.
///
/// Called from the WASM `compute_scroll_velocity` and from unit tests.
#[cfg(any(target_arch = "wasm32", test))]
fn scroll_velocity_for(pointer_y: f64, viewport_height: f64) -> f64 {
    if pointer_y < SCROLL_EDGE_PX {
        // Near top — scroll up (negative)
        let distance = SCROLL_EDGE_PX - pointer_y;
        let ratio = (distance / SCROLL_EDGE_PX).min(1.0);
        -SCROLL_MAX_PX * ratio
    } else if pointer_y > viewport_height - SCROLL_EDGE_PX {
        // Near bottom — scroll down (positive)
        let distance = pointer_y - (viewport_height - SCROLL_EDGE_PX);
        let ratio = (distance / SCROLL_EDGE_PX).min(1.0);
        SCROLL_MAX_PX * ratio
    } else {
        0.0
    }
}

/// Compute viewport-edge auto-scroll velocity based on pointer Y position.
///
/// On WASM, reads `window.innerHeight` and delegates to [`scroll_velocity_for`].
#[cfg(target_arch = "wasm32")]
fn compute_scroll_velocity(pointer_y: f64) -> f64 {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return 0.0,
    };
    let inner_height = match window.inner_height() {
        Ok(v) => v.as_f64().unwrap_or(0.0),
        Err(_) => return 0.0,
    };
    scroll_velocity_for(pointer_y, inner_height)
}

/// Non-wasm stub: always returns 0.0 (no auto-scroll outside browser).
#[cfg(not(target_arch = "wasm32"))]
fn compute_scroll_velocity(_pointer_y: f64) -> f64 {
    0.0
}

/// Select the target used for projected motion/collision geometry.
///
/// A pending hysteresis candidate takes precedence only after it has remained
/// stable for at least [`PROJECTED_PENDING_MIN_MS`]. This damps edge flicker
/// while still aligning projected geometry with maturing target transitions.
fn projected_target_from(
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
fn current_time_ms() -> f64 {
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
    active: Signal<Option<ActiveDrag>>,
    /// Current collision/hover target (reactive)
    current_target: Signal<Option<DropLocation>>,
    /// Registered drop zones (non-reactive — only used in collision detection)
    drop_zones: Signal<HashMap<DragId, DropZoneState>>,
    /// The collision detection strategy (Copy enum — no boxing or Signal needed)
    collision_strategy: CollisionStrategy,
    /// Pending target for hysteresis filtering (candidate + timestamp_ms).
    /// Target changes (Some(A) → Some(B)) are delayed by HYSTERESIS_MS to
    /// filter boundary oscillation. None→Some and Some→None commit immediately.
    pending_target: Signal<Option<(DropLocation, f64)>>,
    /// Which item the projected center is currently traversing (reactive).
    /// Only the matching SortableItem subscribes to traversal_fraction.
    traversal_item: Signal<Option<DragId>>,
    /// 0.0–1.0: how far through the traversal item the projected center has moved.
    /// Updated at ~60fps during drag. Only the traversal item reads this.
    traversal_fraction: Signal<f64>,
    /// The item that just exited traversal, paired with the exit timestamp (ms).
    /// When an item exits traversal, its displacement changes discretely (e.g.,
    /// partial → full shift). Without suppressing the CSS transition, the browser
    /// animates this jump over 250ms → visible bounce. Items check this via
    /// `peek()` during re-render and append `transition: 0s`.
    /// The timestamp enables a time-based snap window (SNAP_WINDOW_MS) that
    /// outlasts the hysteresis delay, preventing bounces on direction reversal.
    previous_traversal: Signal<Option<(DragId, f64)>>,
    /// Whether items displace to create gaps (true) or stay in place with
    /// line indicators (false). Controls visual feedback mode only — collision
    /// strategy is independent.
    gap_displacement: bool,
    /// Monotonically increasing counter bumped on drag start and throttled
    /// drag-time viewport/layout invalidations. Drop zones and sortable items
    /// read this signal in their rect-measurement effects to refresh geometry
    /// when drag/session conditions change.
    measure_generation: Signal<u32>,
    /// Timestamp (ms) of the last measure-generation bump.
    /// Used to throttle drag-time invalidation under viewport motion.
    last_measure_refresh_ms: Signal<f64>,
    /// Text content for the ARIA live region. Updated on drag lifecycle
    /// events so screen readers announce state changes.
    announcement: Signal<String>,
    /// Vertical scroll velocity in px/frame (positive = down, negative = up).
    /// When non-zero, a RAF loop in DragContextProvider scrolls the viewport.
    scroll_velocity: Signal<f64>,
    /// Optional callback for structured keyboard drag announcements.
    /// Stored here so any method (start_keyboard_drag, keyboard_move, etc.)
    /// can dispatch events without needing to pass the callback around.
    on_announce: Signal<Option<EventHandler<AnnouncementEvent>>>,
    /// Whether the current drag is keyboard-driven (no pointer overlay needed).
    keyboard_drag: Signal<bool>,
    /// Virtual cursor position in the current container's items list.
    keyboard_index: Signal<Option<usize>>,
    /// Which container the keyboard cursor is navigating.
    keyboard_container: Signal<Option<DragId>>,
    /// Pointer id that owns the current pointer drag session.
    /// `None` for keyboard-initiated drags.
    active_pointer_id: Signal<Option<i32>>,
    /// Unique ID for the ARIA instructions element, ensuring no duplicates
    /// when multiple DragContextProviders exist on the same page.
    instructions_id: Signal<String>,
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
    // Drag Lifecycle Methods
    // -------------------------------------------------------------------------

    /// Reset transient drag-motion state shared by pointer and keyboard drags.
    fn reset_drag_motion_state(&self) {
        *self.pending_target.write_unchecked() = None;
        *self.traversal_item.write_unchecked() = None;
        *self.traversal_fraction.write_unchecked() = 0.0;
        *self.previous_traversal.write_unchecked() = None;
        *self.scroll_velocity.write_unchecked() = 0.0;
    }

    /// Increment measure generation and stamp refresh time.
    fn bump_measure_generation(&self) {
        let now = current_time_ms();
        let mut measure_gen = self.measure_generation;
        *measure_gen.write() += 1;
        *self.last_measure_refresh_ms.write_unchecked() = now;
    }

    /// Throttled measure-generation bump for drag-time viewport/layout motion.
    ///
    /// On tests (`current_time_ms() == 0.0`) this is a no-op to avoid
    /// introducing non-essential signal churn in synchronous unit paths.
    fn maybe_refresh_measurements(&self) {
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

    fn start_drag_internal(
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
        if let Some(zone) = self.drop_zones.peek().get(&target_container_id) {
            if !zone.accepts_data(&active.data) {
                self.set_announcement("Drop cancelled, item returned to start");
                return None;
            }
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
    fn projected_traversal_axis_start(
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
                .map_or(false, |(_, ts)| current_time_ms() - ts >= SNAP_WINDOW_MS);
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

    // -------------------------------------------------------------------------
    // Keyboard Drag Methods
    // -------------------------------------------------------------------------

    /// Check if the current drag is keyboard-driven (non-reactive peek).
    pub fn is_keyboard_drag(&self) -> bool {
        *self.keyboard_drag.peek()
    }

    /// Get the keyboard drag signal for direct subscription.
    pub fn keyboard_drag_signal(&self) -> Signal<bool> {
        self.keyboard_drag
    }

    /// Get the current keyboard container (non-reactive peek).
    pub fn keyboard_container(&self) -> Option<DragId> {
        self.keyboard_container.peek().clone()
    }

    /// Start a keyboard-driven drag operation.
    ///
    /// Calls `start_drag()` internally with the source zone's center as position,
    /// then sets keyboard-specific state. The keyboard index tracks a virtual
    /// cursor position in the items list, independent of collision detection.
    pub fn start_keyboard_drag(
        &self,
        data: DragData,
        source_id: DragId,
        container_id: DragId,
        items: &[DragId],
        source_index: usize,
    ) {
        // Capture item ID before start_drag consumes data
        let data_id = data.id.clone();

        // Use center of source zone rect as the drag start position
        let position = self
            .drop_zones
            .peek()
            .get(&source_id)
            .map(|z| z.rect.center())
            .unwrap_or_default();

        self.start_drag(data, source_id, position);

        // Set keyboard-specific state
        *self.keyboard_drag.write_unchecked() = true;
        *self.keyboard_index.write_unchecked() = Some(source_index);
        *self.keyboard_container.write_unchecked() = Some(container_id.clone());

        let total = items.len();
        let pos = source_index + 1; // 1-based for announcement
        self.dispatch_announcement(AnnouncementEvent::Grabbed {
            item_id: data_id,
            position: pos,
            total,
            container_id,
        });
    }

    /// Move the keyboard cursor by `delta` positions within the current container.
    ///
    /// Takes the items list directly (from `SortableState.items`) to avoid
    /// reconstructing order from zones. Filters out the dragged item from the
    /// navigable list, clamps to valid range, and writes `DropLocation` directly
    /// to `current_target` (bypassing hysteresis for instant discrete steps).
    ///
    /// Returns `Some((new_1based_position, total))` for announcements,
    /// or `None` if no keyboard drag is active.
    pub fn keyboard_move(&self, delta: i32, items: &[DragId]) -> Option<(usize, usize)> {
        if !*self.keyboard_drag.peek() {
            return None;
        }

        let container_id = self.keyboard_container.peek().clone()?;
        let current_index = (*self.keyboard_index.peek())?;

        // Get the dragged item ID
        let active = self.active.peek();
        let dragged_id = active.as_ref().map(|a| a.data.id.clone())?;

        // Build navigable list (exclude dragged item)
        let navigable: Vec<&DragId> = items.iter().filter(|id| **id != dragged_id).collect();
        let nav_len = navigable.len();

        if nav_len == 0 {
            return None;
        }

        // Clamp new index: valid positions are 0..=nav_len
        // Position nav_len means "after the last navigable item"
        let new_index = (current_index as i32 + delta).clamp(0, nav_len as i32) as usize;

        // At boundary — can't move further in this container.
        // Caller can try keyboard_exit_to_parent() for nested containers.
        if new_index == current_index {
            return None;
        }

        // Write the new DropLocation directly (bypass hysteresis)
        let new_target = DropLocation::AtIndex {
            container_id: container_id.clone(),
            index: new_index,
        };

        // Write target directly (instant for discrete keyboard steps)
        let mut target_sig = self.current_target;
        *self.pending_target.write_unchecked() = None;
        *target_sig.write() = Some(new_target);

        // Update keyboard index
        *self.keyboard_index.write_unchecked() = Some(new_index);

        let pos = new_index + 1; // 1-based for announcement
        Some((pos, nav_len + 1)) // total includes the dragged item's slot
    }

    /// End a keyboard drag, returning the drop event if valid.
    ///
    /// Clears keyboard-specific state and delegates to `end_drag()`.
    pub fn end_keyboard_drag(&self) -> Option<DropEvent> {
        *self.keyboard_drag.write_unchecked() = false;
        *self.keyboard_index.write_unchecked() = None;
        *self.keyboard_container.write_unchecked() = None;

        self.end_drag()
    }

    /// Switch the keyboard cursor to a different container.
    ///
    /// Finds all container zones sorted by position, steps forward/backward,
    /// and wraps around. Skips containers that don't accept the dragged type.
    /// Returns `Some((container_id, position, total))` for announcements.
    pub fn keyboard_switch_container(&self, forward: bool) -> Option<(DragId, usize, usize)> {
        if !*self.keyboard_drag.peek() {
            return None;
        }

        let current_cid = self.keyboard_container.peek().clone()?;
        let active = self.active.peek();
        let drag_data = active.as_ref().map(|a| &a.data)?;

        let zones = self.drop_zones.peek();

        // Find all container zones (where id == container_id) that accept
        // the dragged type, sorted by primary axis position.
        let mut containers: Vec<(&DragId, &DropZoneState)> = zones
            .iter()
            .filter(|(id, zone)| {
                // Container zone: id == container_id
                *id == &zone.container_id
                // Must accept the dragged item's types
                && zone.accepts_data(drag_data)
            })
            .collect();

        if containers.len() <= 1 {
            return None; // No other container to switch to
        }

        // Sort by vertical position (Y for vertical layouts)
        containers.sort_by(|(_, a), (_, b)| {
            a.rect
                .y
                .partial_cmp(&b.rect.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find current container's position
        let current_pos = containers.iter().position(|(id, _)| **id == current_cid)?;

        // Step forward/backward with wrapping
        let new_pos = if forward {
            (current_pos + 1) % containers.len()
        } else {
            (current_pos + containers.len() - 1) % containers.len()
        };

        let new_cid = containers[new_pos].0.clone();

        // Get items in the new container using sorted_items_in_container
        let dragged_id = drag_data.id.clone();
        drop(active); // Release borrow before calling method that borrows zones
        let new_items = sorted_items_in_container(&zones, &new_cid, Some(&dragged_id));

        // Reset keyboard index to 0 (first position in new container)
        let new_index = 0usize;

        // Write initial target in new container
        let new_target = if !new_items.is_empty() {
            DropLocation::AtIndex {
                container_id: new_cid.clone(),
                index: 0,
            }
        } else {
            DropLocation::IntoContainer {
                container_id: new_cid.clone(),
            }
        };

        let mut target_sig = self.current_target;
        *self.pending_target.write_unchecked() = None;
        *target_sig.write() = Some(new_target);

        *self.keyboard_index.write_unchecked() = Some(new_index);
        *self.keyboard_container.write_unchecked() = Some(new_cid.clone());

        let total = new_items.len() + 1; // Include dragged item slot
        let pos = new_index + 1; // 1-based

        Some((new_cid, pos, total))
    }

    /// Toggle the current keyboard target between Before/After and IntoItem.
    ///
    /// Only works when merge is enabled (SortableWithMerge collision strategy)
    /// and the target item's zone accepts the dragged data. Returns the toggled
    /// `DropLocation` for announcement, or `None` if merge isn't possible.
    pub fn keyboard_toggle_merge(&self) -> Option<DropLocation> {
        if !*self.keyboard_drag.peek() || !self.is_merge_enabled() {
            return None;
        }

        let container_id = self.keyboard_container.peek().clone()?;
        let target = self.current_target.peek().clone()?;

        // Check that the target item accepts the dragged data
        let active = self.active.peek();
        let drag_data = active.as_ref().map(|a| &a.data)?;
        let target_item_id = match &target {
            DropLocation::IntoItem { item_id, .. } => item_id.clone(),
            DropLocation::AtIndex {
                container_id: cid,
                index,
            } => {
                // Resolve the item at this index in the container
                let zones = self.drop_zones.peek();
                let items = sorted_items_in_container(&zones, cid, Some(&drag_data.id));
                drop(zones);
                if *index < items.len() {
                    items[*index].clone()
                } else if !items.is_empty() {
                    items[items.len() - 1].clone()
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let zones = self.drop_zones.peek();
        if let Some(zone) = zones.get(&target_item_id) {
            if !zone.accepts_data(drag_data) {
                return None;
            }
        }
        drop(zones);
        drop(active);

        let keyboard_index = self.keyboard_index.peek().unwrap_or(0);
        let new_target =
            toggle_merge_target(&target, &container_id, &target_item_id, keyboard_index)?;
        let mut target_sig = self.current_target;
        *self.pending_target.write_unchecked() = None;
        *target_sig.write() = Some(new_target.clone());

        Some(new_target)
    }

    /// Enter a nested container when the keyboard cursor is on a group item.
    ///
    /// If the current target item has `inner_container_id`, switches the keyboard
    /// cursor into the inner container at position 0. Returns the inner container
    /// ID, position, and total for announcements.
    pub fn keyboard_enter_nested(&self) -> Option<(DragId, usize, usize)> {
        if !*self.keyboard_drag.peek() {
            return None;
        }

        // Get the current target item
        let target = self.current_target.peek().clone()?;
        let (target_item_id, _container_id) = match &target {
            DropLocation::AtIndex {
                container_id,
                index,
            } => {
                let zones = self.drop_zones.peek();
                let active = self.active.peek();
                let dragged_id = active.as_ref().map(|a| a.data.id.clone());
                let items = sorted_items_in_container(&zones, container_id, dragged_id.as_ref());
                drop(active);
                drop(zones);
                if *index < items.len() {
                    (items[*index].clone(), container_id.clone())
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let zones = self.drop_zones.peek();
        let inner_cid = find_inner_container(&zones, &target_item_id)?;

        // Get items in the inner container
        let active = self.active.peek();
        let dragged_id = active.as_ref().map(|a| a.data.id.clone())?;
        drop(active);

        let inner_items = sorted_items_in_container(&zones, &inner_cid, Some(&dragged_id));
        drop(zones);

        // Switch keyboard to inner container at index 0
        let new_target = if !inner_items.is_empty() {
            DropLocation::AtIndex {
                container_id: inner_cid.clone(),
                index: 0,
            }
        } else {
            DropLocation::IntoContainer {
                container_id: inner_cid.clone(),
            }
        };

        let mut target_sig = self.current_target;
        *self.pending_target.write_unchecked() = None;
        *target_sig.write() = Some(new_target);

        *self.keyboard_index.write_unchecked() = Some(0);
        *self.keyboard_container.write_unchecked() = Some(inner_cid.clone());

        let total = inner_items.len() + 1;
        Some((inner_cid, 1, total))
    }

    /// Exit from a nested container to the parent container.
    ///
    /// If the current keyboard container is a nested inner container, moves the
    /// cursor to the parent container positioned after the group wrapper item.
    /// Returns the parent container ID, position, and total for announcements.
    pub fn keyboard_exit_to_parent(&self) -> Option<(DragId, usize, usize)> {
        if !*self.keyboard_drag.peek() {
            return None;
        }

        let current_cid = self.keyboard_container.peek().clone()?;
        let zones = self.drop_zones.peek();

        // Find the parent container and the wrapper item
        let (parent_cid, wrapper_item_id) = find_nested_exit(&zones, &current_cid)?;

        // Get items in the parent container
        let active = self.active.peek();
        let dragged_id = active.as_ref().map(|a| a.data.id.clone())?;
        drop(active);

        let parent_items = sorted_items_in_container(&zones, &parent_cid, Some(&dragged_id));
        drop(zones);

        // Find the wrapper item's index in parent items
        let wrapper_idx = parent_items
            .iter()
            .position(|id| *id == wrapper_item_id)
            .unwrap_or(0);

        // Position after the wrapper item
        let new_index = wrapper_idx + 1;

        let new_target = DropLocation::AtIndex {
            container_id: parent_cid.clone(),
            index: new_index,
        };

        let mut target_sig = self.current_target;
        *self.pending_target.write_unchecked() = None;
        *target_sig.write() = Some(new_target);

        *self.keyboard_index.write_unchecked() = Some(new_index);
        *self.keyboard_container.write_unchecked() = Some(parent_cid.clone());

        let total = parent_items.len() + 1;
        let pos = new_index + 1;
        Some((parent_cid, pos, total))
    }
}

// ============================================================================
// Keyboard Drag Helpers (free functions for testability)
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

/// Toggle a DropLocation between Before/After and IntoItem.
///
/// When the current target is Before or After, switches to IntoItem for the
/// same item. When IntoItem, switches back to Before (using the given container).
/// Returns `None` for non-togglable locations (IntoContainer, AtIndex, etc.).
/// Focus a SortableItem element by its DragId after a keyboard drop.
///
/// Spawns an async task that waits two animation frames (for DOM re-render)
/// then focuses the element with the matching `data-dnd-id` attribute.
#[cfg(target_arch = "wasm32")]
fn keyboard_focus_item(item_id: DragId) {
    use wasm_bindgen::JsCast;
    spawn(async move {
        NextAnimationFrame::new().await;
        NextAnimationFrame::new().await;
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let selector = format!("[data-dnd-id=\"{}\"]", item_id.0);
            if let Ok(Some(el)) = document.query_selector(&selector) {
                if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                    let _ = html_el.focus();
                }
            }
        }
    });
}

/// Toggle a DropLocation between AtIndex and IntoItem.
///
/// When the current target is AtIndex, switches to IntoItem for the item
/// at that index. When IntoItem, switches back to AtIndex.
/// The caller resolves `target_item_id` from the current keyboard position.
pub fn toggle_merge_target(
    location: &DropLocation,
    keyboard_container: &DragId,
    target_item_id: &DragId,
    keyboard_index: usize,
) -> Option<DropLocation> {
    match location {
        DropLocation::AtIndex { container_id, .. } => Some(DropLocation::IntoItem {
            container_id: container_id.clone(),
            item_id: target_item_id.clone(),
        }),
        DropLocation::IntoItem { .. } => Some(DropLocation::AtIndex {
            container_id: keyboard_container.clone(),
            index: keyboard_index,
        }),
        _ => None,
    }
}

/// Find the parent container and item index for exiting a nested container.
///
/// Given a nested inner container ID (e.g., "group-1-container"), searches
/// zones for the parent item whose `inner_container_id` matches, then returns
/// the parent container ID and the parent item ID (for positioning After it).
pub fn find_nested_exit(
    zones: &HashMap<DragId, DropZoneState>,
    inner_container_id: &DragId,
) -> Option<(DragId, DragId)> {
    // Find the item zone in the parent container whose inner_container_id matches
    zones
        .values()
        .find(|z| z.inner_container_id.as_ref() == Some(inner_container_id))
        .map(|parent_item| (parent_item.container_id.clone(), parent_item.id.clone()))
}

/// Check if an item has an inner container (is a group wrapper).
///
/// Returns the inner container ID if the item zone has `inner_container_id` set.
pub fn find_inner_container(
    zones: &HashMap<DragId, DropZoneState>,
    item_id: &DragId,
) -> Option<DragId> {
    zones
        .get(item_id)
        .and_then(|z| z.inner_container_id.clone())
}

// ============================================================================
// DragContextProvider Component
// ============================================================================

/// Props for the DragContextProvider component
#[derive(Props, Clone)]
pub struct DragContextProviderProps {
    /// Children elements
    pub children: Element,

    /// Callback when an item is dropped
    #[props(default)]
    pub on_drop: EventHandler<DropEvent>,

    /// Collision detection strategy
    #[props(default)]
    pub collision_detection: CollisionStrategy,

    /// Whether items displace to create gaps (true, default) or stay in place
    /// with line indicators (false). Controls visual feedback only — collision
    /// strategy is independent.
    #[props(default = true)]
    pub gap_displacement: bool,

    /// Optional callback for structured keyboard drag announcements.
    ///
    /// When provided, called at each keyboard drag lifecycle point
    /// (grab, move, switch container, drop, cancel). If not provided,
    /// default English text is used via `AnnouncementEvent::default_text()`.
    ///
    /// Use this callback for i18n or custom announcement wording.
    #[props(default)]
    pub on_announce: EventHandler<AnnouncementEvent>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Forwarded to the wrapper div. The provider uses `display: contents`
    /// and pointer/keyboard event handlers — these are preserved while
    /// consumer attributes are merged.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Always returns `false`: props contain [`Element`] (children), [`EventHandler`]
/// (on_announce), and [`Attribute`]s — none of which support meaningful equality
/// comparison. Returning `false` tells Dioxus to always re-render this component,
/// which is the intended behavior for reactive signal-driven updates.
impl PartialEq for DragContextProviderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// Provider component for the drag-and-drop context
///
/// Wrap your application (or the part that needs drag-and-drop) with this
/// component. It provides:
/// - Global drag state management
/// - Pointer event handling for drag updates
/// - Drop event dispatching
///
/// # Example
///
/// ```ignore
/// rsx! {
///     DragContextProvider {
///         on_drop: move |event: DropEvent| {
///             // Handle the drop
///         },
///         // Your draggables and drop zones here
///     }
/// }
/// ```
#[component]
pub fn DragContextProvider(props: DragContextProviderProps) -> Element {
    // Create the context with the specified collision strategy and gap displacement mode
    // DragContext is Copy (contains only Signal handles), so no outer Signal wrapper needed
    let context =
        use_hook(|| DragContext::with_options(props.collision_detection, props.gap_displacement));

    // Provide DragContext directly via context (not wrapped in Signal)
    use_context_provider(|| context);

    // Auto-scroll effect: starts a RAF loop while dragging to scroll the
    // viewport when the pointer is near the top/bottom edge. The loop
    // continues even when the pointer is stationary (unlike pointermove).
    {
        let active_sig = context.active_signal();
        #[cfg(target_arch = "wasm32")]
        let scroll_vel = context.scroll_velocity_signal();
        #[cfg(target_arch = "wasm32")]
        let ctx_for_scroll = context;
        use_effect(move || {
            let is_active = active_sig.read().is_some();
            if !is_active {
                return;
            }

            #[cfg(target_arch = "wasm32")]
            {
                // Spawn RAF loop for auto-scroll and viewport-driven rect refresh.
                spawn(async move {
                    loop {
                        NextAnimationFrame::new().await;
                        // Check if drag is still active (non-reactive peek)
                        if active_sig.peek().is_none() {
                            break;
                        }

                        // Keep geometry fresh while dragging even if the pointer
                        // is stationary (e.g., auto-scroll / viewport motion).
                        ctx_for_scroll.maybe_refresh_measurements();

                        let vel = *scroll_vel.peek();
                        if vel == 0.0 {
                            continue; // No scroll needed, but keep loop alive
                        }
                        if let Some(window) = web_sys::window() {
                            window.scroll_by_with_x_and_y(0.0, vel);
                        }
                    }
                });
            }
        });
    }

    // Clone handler for use in closure
    let on_drop = props.on_drop;

    // Store the on_announce callback on the context so keyboard lifecycle
    // methods (start_keyboard_drag, etc.) can dispatch structured events.
    context.set_on_announce(props.on_announce);

    let is_keyboard_active = context.is_keyboard_drag();

    // Extract consumer class and style, merge with library styles
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");
    let base_style = "display: contents;";
    let merged_style = match consumer_style {
        Some(s) if !s.is_empty() => format!("{} {}", base_style, s),
        _ => base_style.to_string(),
    };
    let merged_class = consumer_class.unwrap_or_default();
    let remaining_attrs = filter_class_style(props.attributes);

    rsx! {
        div {
            class: "{merged_class}",
            style: "{merged_style}",
            "data-keyboard-active": if is_keyboard_active { "true" },

            onpointermove: move |e| {
                context.update_drag_with_pointer(
                    Position {
                        x: e.client_coordinates().x,
                        y: e.client_coordinates().y,
                    },
                    Some(e.data().pointer_id()),
                );
            },
            onpointerup: move |e| {
                if let Some(event) = context.end_drag_with_pointer(Some(e.data().pointer_id())) {
                    on_drop.call(event);
                }
            },
            onpointercancel: move |e| {
                context.cancel_drag_with_pointer(Some(e.data().pointer_id()));
            },
            onkeydown: move |e: KeyboardEvent| {
                let key = e.key();

                // During keyboard drag: handle navigation and drop
                if context.is_keyboard_drag() {
                    // Helper: reconstruct items list for current keyboard container
                    let get_all_items = |cid: &DragId| -> Option<(Vec<DragId>, DragId)> {
                        let active_id = context.active.peek().as_ref().map(|a| a.data.id.clone())?;
                        let zones = context.drop_zones.peek();
                        let mut items = sorted_items_in_container(&zones, cid, Some(&active_id));
                        drop(zones);
                        items.push(active_id.clone());
                        Some((items, active_id))
                    };

                    match key {
                        Key::Escape => {
                            let item_id = context.active.peek().as_ref().map(|a| a.data.id.clone());
                            context.cancel_drag();
                            if let Some(item_id) = item_id {
                                context.dispatch_announcement(AnnouncementEvent::Cancelled { item_id });
                            }
                            e.prevent_default();
                        }
                        Key::ArrowDown | Key::ArrowRight => {
                            if let Some(cid) = context.keyboard_container() {
                                if let Some((all_items, item_id)) = get_all_items(&cid) {
                                    if let Some((pos, total)) = context.keyboard_move(1, &all_items) {
                                        // Check if the new target is a group item — enter it
                                        if let Some((inner_cid, inner_pos, inner_total)) = context.keyboard_enter_nested() {
                                            context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                                item_id, position: inner_pos, total: inner_total, container_id: inner_cid,
                                            });
                                        } else {
                                            context.dispatch_announcement(AnnouncementEvent::Moved {
                                                item_id, position: pos, total, container_id: cid,
                                            });
                                        }
                                    } else {
                                        // At boundary — try exiting nested container
                                        if let Some((parent_cid, pos, total)) = context.keyboard_exit_to_parent() {
                                            context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                                item_id, position: pos, total, container_id: parent_cid,
                                            });
                                        }
                                    }
                                }
                            }
                            e.prevent_default();
                        }
                        Key::ArrowUp | Key::ArrowLeft => {
                            if let Some(cid) = context.keyboard_container() {
                                if let Some((all_items, item_id)) = get_all_items(&cid) {
                                    if let Some((pos, total)) = context.keyboard_move(-1, &all_items) {
                                        // Check if the new target is a group item — enter it
                                        if let Some((inner_cid, inner_pos, inner_total)) = context.keyboard_enter_nested() {
                                            context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                                item_id, position: inner_pos, total: inner_total, container_id: inner_cid,
                                            });
                                        } else {
                                            context.dispatch_announcement(AnnouncementEvent::Moved {
                                                item_id, position: pos, total, container_id: cid,
                                            });
                                        }
                                    } else {
                                        // At boundary — try exiting nested container
                                        if let Some((parent_cid, pos, total)) = context.keyboard_exit_to_parent() {
                                            context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                                item_id, position: pos, total, container_id: parent_cid,
                                            });
                                        }
                                    }
                                }
                            }
                            e.prevent_default();
                        }
                        Key::Character(ref c) if c == " " => {
                            // Keyboard drop (Space)
                            let (item_id, cid, index, total) = {
                                let a = context.active.peek();
                                let id = a.as_ref().map(|a| a.data.id.clone());
                                let c = context.keyboard_container();
                                let i = *context.keyboard_index.peek();
                                // Compute real total: items in container + dragged item
                                let t = c.as_ref().and_then(|cid| {
                                    get_all_items(cid).map(|(items, _)| items.len())
                                });
                                (id, c, i, t)
                            };
                            let _focus_id = item_id.clone();
                            if let Some(event) = context.end_keyboard_drag() {
                                if let (Some(item_id), Some(cid), Some(idx), Some(total)) = (item_id, cid, index, total) {
                                    context.dispatch_announcement(AnnouncementEvent::Dropped {
                                        item_id, position: idx + 1, total, container_id: cid,
                                    });
                                }
                                on_drop.call(event);

                                // Focus the dropped item at its new position
                                #[cfg(target_arch = "wasm32")]
                                if let Some(focus_id) = _focus_id {
                                    keyboard_focus_item(focus_id);
                                }
                            }
                            e.prevent_default();
                        }
                        Key::Enter => {
                            // Keyboard drop (Enter)
                            let (item_id, cid, index, total) = {
                                let a = context.active.peek();
                                let id = a.as_ref().map(|a| a.data.id.clone());
                                let c = context.keyboard_container();
                                let i = *context.keyboard_index.peek();
                                // Compute real total: items in container + dragged item
                                let t = c.as_ref().and_then(|cid| {
                                    get_all_items(cid).map(|(items, _)| items.len())
                                });
                                (id, c, i, t)
                            };
                            let _focus_id = item_id.clone();
                            if let Some(event) = context.end_keyboard_drag() {
                                if let (Some(item_id), Some(cid), Some(idx), Some(total)) = (item_id, cid, index, total) {
                                    context.dispatch_announcement(AnnouncementEvent::Dropped {
                                        item_id, position: idx + 1, total, container_id: cid,
                                    });
                                }
                                on_drop.call(event);

                                // Focus the dropped item at its new position
                                #[cfg(target_arch = "wasm32")]
                                if let Some(focus_id) = _focus_id {
                                    keyboard_focus_item(focus_id);
                                }
                            }
                            e.prevent_default();
                        }
                        Key::Tab => {
                            let forward = !e.modifiers().shift();
                            let item_id = context.active.peek().as_ref().map(|a| a.data.id.clone());
                            if let Some((cid, pos, total)) = context.keyboard_switch_container(forward) {
                                if let Some(item_id) = item_id {
                                    context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                        item_id, position: pos, total, container_id: cid,
                                    });
                                }
                            }
                            e.prevent_default();
                        }
                        // M key: toggle merge target (Before/After ↔ IntoItem)
                        Key::Character(ref c) if c == "m" || c == "M" => {
                            if let Some(new_target) = context.keyboard_toggle_merge() {
                                let is_merge = matches!(new_target, DropLocation::IntoItem { .. });
                                if is_merge {
                                    let target_item = match &new_target {
                                        DropLocation::IntoItem { item_id, .. } => item_id.0.clone(),
                                        _ => unreachable!(),
                                    };
                                    context.set_announcement(format!(
                                        "Merge with {}",
                                        target_item
                                    ));
                                } else {
                                    let pos = (*context.keyboard_index.peek()).map(|i| i + 1).unwrap_or(1);
                                    context.set_announcement(format!(
                                        "Position before item, position {}",
                                        pos,
                                    ));
                                }
                            }
                            e.prevent_default();
                        }
                        _ => {}
                    }

                    // scrollIntoView for the current target after keyboard navigation
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(target) = context.current_target.peek().clone() {
                            let target_item_id = match &target {
                                DropLocation::IntoItem { item_id, .. } => Some(item_id.0.as_str()),
                                DropLocation::AtIndex { container_id, index } => {
                                    // Resolve item at this index for scrolling
                                    let zones = context.drop_zones.peek();
                                    let active = context.active.peek();
                                    let dragged_id = active.as_ref().map(|a| &a.data.id);
                                    let items = sorted_items_in_container(&zones, container_id, dragged_id);
                                    drop(active);
                                    drop(zones);
                                    // We can't return a reference into items, so we use the selector directly below
                                    if *index < items.len() {
                                        // Use query selector with the item's data-dnd-id
                                        let id_str = items[*index].0.clone();
                                        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                                            let selector = format!("[data-dnd-item][data-dnd-id=\"{}\"]", id_str);
                                            if let Ok(Some(el)) = document.query_selector(&selector) {
                                                let opts = web_sys::ScrollIntoViewOptions::new();
                                                opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                                                el.scroll_into_view_with_scroll_into_view_options(&opts);
                                            }
                                        }
                                    }
                                    None // Already handled scroll above
                                }
                                _ => None,
                            };
                            if let Some(id) = target_item_id {
                                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                                    let selector = format!("[data-dnd-item][data-dnd-id=\"{}\"]", id);
                                    if let Ok(Some(el)) = document.query_selector(&selector) {
                                        let opts = web_sys::ScrollIntoViewOptions::new();
                                        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                                        el.scroll_into_view_with_scroll_into_view_options(&opts);
                                    }
                                }
                            }
                        }
                    }

                    return;
                }

                // Not in keyboard drag: Escape cancels pointer drag
                if key == Key::Escape && context.is_dragging() {
                    context.cancel_drag();
                    e.prevent_default();
                }
            },
            ..remaining_attrs,

            span {
                id: "{context.instructions_id().read()}",
                style: "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                "Press space bar to start a drag. While dragging, use arrow keys to move, tab to switch lists, M to toggle merge. Press space bar to drop, or press escape to cancel."
            }

            // ARIA live region for screen reader announcements.
            // Announced each time the text changes (assertive priority).
            div {
                role: "status",
                aria_live: "assertive",
                aria_atomic: "true",
                style: "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                {context.announcement_signal().read().clone()}
            }

            {props.children}
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
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
}
