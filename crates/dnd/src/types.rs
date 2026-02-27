//! Core types for drag-and-drop operations
//!
//! This module contains all the fundamental types used throughout the dx-dnd library:
//! - Identity types (`DragId`, `DragType`, `DragData`)
//! - Geometry types (`Position`, `Rect`)
//! - Drop location abstraction (`DropLocation`)
//! - Event types for drag operations

use dioxus::prelude::{ReadableExt, WritableExt};

// ============================================================================
// Identity Types (Task 1.1)
// ============================================================================

/// Unique identifier for any draggable or droppable entity
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DragId(pub String);

impl DragId {
    /// Create a new DragId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<&str> for DragId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for DragId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for DragId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Type discriminator - drop zones can filter by accepted types
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DragType(pub String);

impl DragType {
    /// Create a new DragType from a string
    pub fn new(drag_type: impl Into<String>) -> Self {
        Self(drag_type.into())
    }
}

impl From<&str> for DragType {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for DragType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for DragType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The payload carried during a drag operation
///
/// Note: `Box<dyn Any + Send + Sync>` doesn't work well in WASM.
/// For type-safe payloads, users should create wrapper types or use
/// the pattern layer which provides typed events.
///
/// # Multiple Types
///
/// DragData supports multiple drag types, allowing items to be both
/// "sortable" AND have custom filter types (e.g., "image", "document").
/// This enables type-filtered sortable lists.
///
/// # Example
///
/// ```ignore
/// // Single type (backward compatible)
/// let data = DragData::new("item-1", "task");
///
/// // Multiple types
/// let data = DragData::with_types("item-1", vec![
///     DragType::new("sortable"),
///     DragType::new("image"),
/// ]);
/// ```
#[derive(Clone, Debug)]
pub struct DragData {
    /// Unique identifier for the dragged item
    pub id: DragId,
    /// All type discriminators for drop zone filtering (supports multiple types)
    /// The first element is the primary type.
    pub drag_types: Vec<DragType>,
}

impl DragData {
    /// Create new DragData with a single drag type
    pub fn new(id: impl Into<DragId>, drag_type: impl Into<DragType>) -> Self {
        Self {
            id: id.into(),
            drag_types: vec![drag_type.into()],
        }
    }

    /// Create new DragData with multiple drag types
    ///
    /// The first type in the list is the primary type (accessible via `primary_type()`).
    pub fn with_types(id: impl Into<DragId>, drag_types: Vec<DragType>) -> Self {
        Self {
            id: id.into(),
            drag_types: if drag_types.is_empty() {
                vec![DragType::new("")]
            } else {
                drag_types
            },
        }
    }

    /// Get the primary (first) drag type
    #[inline]
    pub fn primary_type(&self) -> &DragType {
        &self.drag_types[0]
    }

    /// Check if this DragData has a specific type
    pub fn has_type(&self, drag_type: &DragType) -> bool {
        self.drag_types.contains(drag_type)
    }

    /// Check if this DragData has any of the given types
    ///
    /// Returns true if:
    /// - The accepts list is empty (accepts all), OR
    /// - Any of this item's types matches any of the accepts types
    pub fn has_any_type(&self, accepts: &[DragType]) -> bool {
        accepts.is_empty() || self.drag_types.iter().any(|t| accepts.contains(t))
    }
}

// ============================================================================
// Type Combination Utilities
// ============================================================================

/// Combine primary and additional drag types into a single Vec.
///
/// If no types are provided (primary is None and additional is empty),
/// uses the `default_type` as the sole type.
///
/// # Arguments
///
/// * `primary` - Optional primary drag type (e.g., from `drag_type` prop)
/// * `additional` - Additional types (e.g., from `additional_types` or `content_types` prop)
/// * `default_type` - Fallback type if no types provided (e.g., "sortable" or "")
///
/// # Returns
///
/// A Vec containing:
/// - If primary is Some: [primary, ...additional]
/// - If primary is None but additional non-empty: [...additional]
/// - If both are empty: [default_type]
///
/// # Example
///
/// ```ignore
/// // SortableItem with default "sortable" type + content type
/// let types = combine_drag_types(None, &[DragType::new("image")], "sortable");
/// // Result: ["sortable", "image"]
///
/// // Draggable with custom primary type
/// let types = combine_drag_types(Some(&DragType::new("task")), &[], "");
/// // Result: ["task"]
///
/// // Draggable with no types (uses default)
/// let types = combine_drag_types(None, &[], "");
/// // Result: [""]
/// ```
pub fn combine_drag_types(
    primary: Option<&DragType>,
    additional: &[DragType],
    default_type: &str,
) -> Vec<DragType> {
    let mut types = Vec::with_capacity(1 + additional.len());

    if let Some(dt) = primary {
        types.push(dt.clone());
    }

    types.extend(additional.iter().cloned());

    // If no types provided, use the default_type
    if types.is_empty() {
        types.push(DragType::new(default_type));
    }

    types
}

// ============================================================================
// Geometry Types (Task 1.2)
// ============================================================================

/// Position in 2D space
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    /// Create a new position
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate distance to another position
    pub fn distance_to(&self, other: Position) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// Bounding rectangle for collision detection
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a position is contained within this rectangle
    pub fn contains(&self, pos: Position) -> bool {
        pos.x >= self.x
            && pos.x <= self.x + self.width
            && pos.y >= self.y
            && pos.y <= self.y + self.height
    }

    /// Get the center point of this rectangle
    pub fn center(&self) -> Position {
        Position {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    /// Return a new rect expanded by `amount` on all sides.
    /// Useful for overshoot tolerance — expanding a container's bounds
    /// to catch pointers that slightly exit the container.
    pub fn expanded(&self, amount: f64) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }

    /// Create from a web_sys::DomRect (converts immediately to owned)
    pub fn from_dom_rect(dom_rect: &web_sys::DomRect) -> Self {
        Self {
            x: dom_rect.x(),
            y: dom_rect.y(),
            width: dom_rect.width(),
            height: dom_rect.height(),
        }
    }
}

// ============================================================================
// DropLocation Enum (Task 1.3)
// ============================================================================

/// Describes WHERE something will be dropped
///
/// This is the key abstraction that unifies all drag-and-drop patterns.
/// Each variant represents a different semantic meaning for the drop location.
#[derive(Clone, Debug, PartialEq)]
pub enum DropLocation {
    /// Drop at a specific index in a container's item list.
    /// Index is relative to the item list WITH the dragged item removed
    /// (the "filtered list" / "final position" convention).
    AtIndex { container_id: DragId, index: usize },

    /// Drop into a container without specific position
    IntoContainer { container_id: DragId },

    /// Drop onto the center of an item (triggers grouping)
    IntoItem {
        container_id: DragId,
        item_id: DragId,
    },
}

impl DropLocation {
    /// Get the container ID regardless of variant
    pub fn container_id(&self) -> DragId {
        match self {
            DropLocation::AtIndex { container_id, .. } => container_id.clone(),
            DropLocation::IntoContainer { container_id } => container_id.clone(),
            DropLocation::IntoItem { container_id, .. } => container_id.clone(),
        }
    }

    /// Check if a given ID is referenced in this drop location
    pub fn contains_id(&self, id: &DragId) -> bool {
        match self {
            DropLocation::AtIndex { container_id, .. } => container_id == id,
            DropLocation::IntoContainer { container_id } => container_id == id,
            DropLocation::IntoItem {
                container_id,
                item_id,
            } => container_id == id || item_id == id,
        }
    }

    /// Returns true if THIS CONTAINER is the primary target (not a child item)
    ///
    /// Use this for container hover styling. A container should only show hover
    /// when it's directly targeted (AtIndex, IntoContainer), not when an item
    /// inside it is targeted (IntoItem).
    pub fn is_container_targeted(&self, id: &DragId) -> bool {
        match self {
            DropLocation::AtIndex { container_id, .. } => container_id == id,
            DropLocation::IntoContainer { container_id } => container_id == id,
            DropLocation::IntoItem { .. } => false,
        }
    }

    /// Returns true if THIS ITEM is the primary target
    ///
    /// Use this for item hover styling. An item should show hover when it's
    /// directly targeted (IntoItem).
    pub fn is_item_targeted(&self, id: &DragId) -> bool {
        match self {
            DropLocation::IntoItem { item_id, .. } => item_id == id,
            DropLocation::AtIndex { .. } => false,
            DropLocation::IntoContainer { .. } => false,
        }
    }

    /// Resolve this drop location to a concrete index in the given items list.
    ///
    /// Maps each variant to an insertion index:
    /// - `AtIndex { index }` -> index
    /// - `IntoContainer` -> items.len() (append)
    /// - `IntoItem { item_id }` -> index of item_id (or items.len() if not found)
    pub fn resolve_drop_index(&self, items: &[DragId]) -> usize {
        match self {
            DropLocation::AtIndex { index, .. } => *index,
            DropLocation::IntoContainer { .. } => items.len(),
            DropLocation::IntoItem { item_id, .. } => items
                .iter()
                .position(|id| id == item_id)
                .unwrap_or(items.len()),
        }
    }
}

// ============================================================================
// Event Types (Task 1.4)
// ============================================================================

/// What happened as a result of a drop
#[derive(Clone, Debug)]
pub struct DropEvent {
    /// Data about the dragged item
    pub dragged: DragData,
    /// Where it was dropped
    pub location: DropLocation,
    /// ID of the source element (the dragged item)
    pub source: DragId,
    /// ID of the container the source came from (for cross-container moves)
    pub source_container: Option<DragId>,
    /// Index of the source item in its original container (before drag)
    pub source_index: Option<usize>,
}

impl DropEvent {
    /// Create a new drop event
    pub fn new(
        dragged: DragData,
        location: DropLocation,
        source: impl Into<DragId>,
        source_container: Option<DragId>,
        source_index: Option<usize>,
    ) -> Self {
        Self {
            dragged,
            location,
            source: source.into(),
            source_container,
            source_index,
        }
    }

    /// Find and move item from any source container to target container.
    ///
    /// This is a convenience method that handles the common case of moving items
    /// between containers using primitives (Draggable + DropZone). It searches all
    /// containers for the dragged item, removes it from the source, and appends it
    /// to the target container.
    ///
    /// # Arguments
    ///
    /// * `containers` - A slice of (container_id, list_signal) pairs
    /// * `get_id` - A function that extracts a DragId from an item
    ///
    /// # Returns
    ///
    /// Returns `true` if the move was successfully applied, `false` if the item
    /// or target container could not be found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let containers = [
    ///     (DragId::new("source"), source_signal),
    ///     (DragId::new("target"), target_signal),
    /// ];
    ///
    /// DragContextProvider {
    ///     on_drop: move |e: DropEvent| {
    ///         e.apply(&containers, |f: &FileItem| f.drag_id());
    ///     },
    /// }
    /// ```
    pub fn apply<T: Clone + 'static>(
        &self,
        containers: &[(DragId, dioxus::prelude::Signal<Vec<T>>)],
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        let item_id = &self.dragged.id;
        let target_id = self.location.container_id();

        // Find and remove from source container
        let mut removed_item: Option<T> = None;
        for (_, signal) in containers {
            // Copy the signal (Signal is Copy) so we can call read()/write() on it
            let mut signal = *signal;
            let idx = signal.read().iter().position(|t| get_id(t) == *item_id);
            if let Some(idx) = idx {
                removed_item = Some(signal.write().remove(idx));
                break;
            }
        }

        // Insert into target container
        if let Some(item) = removed_item {
            if let Some((_, target_signal)) = containers.iter().find(|(id, _)| *id == target_id) {
                let mut target_signal = *target_signal;
                target_signal.write().push(item);
                return true;
            }
        }
        false
    }

    /// Apply this drop event to plain Vec containers (no Signal dependency).
    ///
    /// Same logic as `apply()` but works on mutable Vec slices.
    ///
    /// # Arguments
    ///
    /// * `containers` - A mutable slice of (container_id, items_vec) pairs
    /// * `get_id` - A function that extracts a DragId from an item
    pub fn apply_to_vecs<T: Clone>(
        &self,
        containers: &mut [(DragId, Vec<T>)],
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        let item_id = &self.dragged.id;
        let target_id = self.location.container_id();

        // Find and remove from source container
        let mut removed_item: Option<T> = None;
        for (_, items) in containers.iter_mut() {
            let idx = items.iter().position(|t| get_id(t) == *item_id);
            if let Some(idx) = idx {
                removed_item = Some(items.remove(idx));
                break;
            }
        }

        // Insert into target container
        if let Some(item) = removed_item {
            if let Some((_, target_items)) = containers.iter_mut().find(|(id, _)| *id == target_id)
            {
                target_items.push(item);
                return true;
            }
        }
        false
    }
}

/// Event for sortable list reordering (same container)
#[derive(Clone, Debug)]
pub struct ReorderEvent {
    /// The container where reordering occurred
    pub container_id: DragId,
    /// Original index of the item in the full items list
    pub from_index: usize,
    /// Target index using filtered-list convention (position after dragged item removed)
    pub to_index: usize,
    /// ID of the item being reordered
    pub item_id: DragId,
}

impl ReorderEvent {
    /// Create a new reorder event
    pub fn new(
        container_id: impl Into<DragId>,
        from_index: usize,
        to_index: usize,
        item_id: impl Into<DragId>,
    ) -> Self {
        Self {
            container_id: container_id.into(),
            from_index,
            to_index,
            item_id: item_id.into(),
        }
    }

    /// Apply this reorder event to a single container.
    ///
    /// This is a convenience method for the common single-container case.
    /// It's equivalent to calling `apply(&[(self.container_id, items)], get_id)`.
    ///
    /// # Arguments
    ///
    /// * `items` - Signal containing the items to reorder
    /// * `get_id` - A function that extracts a DragId from an item
    ///
    /// # Returns
    ///
    /// Returns `true` if the reorder was successfully applied.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableContext {
    ///     on_reorder: move |e: ReorderEvent| {
    ///         e.apply_single(&tasks, |t: &Task| t.id());
    ///     },
    ///     // ...
    /// }
    /// ```
    pub fn apply_single<T: Clone + 'static>(
        &self,
        items: dioxus::prelude::Signal<Vec<T>>,
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        self.apply(&[(self.container_id.clone(), items)], get_id)
    }

    /// Apply this reorder event to a set of container lists.
    ///
    /// This is a convenience method that handles the common case of reordering
    /// items within a container. It finds the item, removes it from its current
    /// position, and inserts it at the new position.
    ///
    /// # Arguments
    ///
    /// * `containers` - A slice of (container_id, list_signal) pairs
    /// * `get_id` - A function that extracts a DragId from an item
    ///
    /// # Returns
    ///
    /// Returns `true` if the reorder was successfully applied, `false` if the
    /// item or container could not be found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableGroup {
    ///     on_reorder: move |e: ReorderEvent| {
    ///         e.apply(
    ///             &[
    ///                 (DragId::new("list-a"), list_a),
    ///                 (DragId::new("list-b"), list_b),
    ///             ],
    ///             |task: &Task| DragId::new(&task.id),
    ///         );
    ///     },
    /// }
    /// ```
    pub fn apply<T: Clone + 'static>(
        &self,
        containers: &[(DragId, dioxus::prelude::Signal<Vec<T>>)],
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        // Find the container
        let container = containers.iter().find(|(id, _)| *id == self.container_id);

        if let Some((_, signal)) = container {
            let mut signal = *signal;
            let mut items = signal.write();
            self.apply_to_vec(&mut items, &get_id)
        } else {
            false
        }
    }

    /// Apply this reorder event to a plain Vec (no Signal dependency).
    ///
    /// Finds the item, removes it, and inserts at `to_index` (filtered-list convention).
    /// After removing the item from its current position, inserts at
    /// `to_index.min(items.len())` directly.
    pub fn apply_to_vec<T: Clone>(
        &self,
        items: &mut Vec<T>,
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        let from_idx = items.iter().position(|t| get_id(t) == self.item_id);

        if let Some(from) = from_idx {
            let item = items.remove(from);
            let len = items.len();
            items.insert(self.to_index.min(len), item);
            true
        } else {
            false
        }
    }
}

/// Event for moving between containers
#[derive(Clone, Debug)]
pub struct MoveEvent {
    /// ID of the item being moved
    pub item_id: DragId,
    /// Container the item is moving from
    pub from_container: DragId,
    /// Index in the source container
    pub from_index: usize,
    /// Container the item is moving to
    pub to_container: DragId,
    /// Target index using filtered-list convention (position in destination after removal from source)
    pub to_index: usize,
}

impl MoveEvent {
    /// Create a new move event
    pub fn new(
        item_id: impl Into<DragId>,
        from_container: impl Into<DragId>,
        from_index: usize,
        to_container: impl Into<DragId>,
        to_index: usize,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            from_container: from_container.into(),
            from_index,
            to_container: to_container.into(),
            to_index,
        }
    }

    /// Apply this move event to a set of container lists.
    ///
    /// This is a convenience method that handles the common case of moving items
    /// between containers. It finds the item in the source container, removes it,
    /// and inserts it at `to_index` in the destination container.
    ///
    /// # Arguments
    ///
    /// * `containers` - A slice of (container_id, list_signal) pairs
    /// * `get_id` - A function that extracts a DragId from an item
    ///
    /// # Returns
    ///
    /// Returns `true` if the move was successfully applied, `false` if the item
    /// or containers could not be found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableGroup {
    ///     on_move: move |e: MoveEvent| {
    ///         e.apply(
    ///             &[
    ///                 (DragId::new("list-a"), list_a),
    ///                 (DragId::new("list-b"), list_b),
    ///             ],
    ///             |task: &Task| DragId::new(&task.id),
    ///         );
    ///     },
    /// }
    /// ```
    pub fn apply<T: Clone + 'static>(
        &self,
        containers: &[(DragId, dioxus::prelude::Signal<Vec<T>>)],
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        // Find source and destination containers
        let source = containers.iter().find(|(id, _)| *id == self.from_container);
        let dest = containers.iter().find(|(id, _)| *id == self.to_container);

        match (source, dest) {
            (Some((_, src_signal)), Some((_, dst_signal))) => {
                let mut src_signal = *src_signal;
                let mut dst_signal = *dst_signal;

                // Find and remove item from source
                let item = {
                    let mut src = src_signal.write();
                    let idx = src.iter().position(|t| get_id(t) == self.item_id);
                    idx.map(|i| src.remove(i))
                };

                // Insert into destination at to_index
                if let Some(item) = item {
                    let mut dst = dst_signal.write();
                    let len = dst.len();
                    dst.insert(self.to_index.min(len), item);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Apply this move event to plain Vec containers (no Signal dependency).
    ///
    /// Finds the item in the source Vec, removes it, and inserts at
    /// `to_index` in the destination Vec.
    pub fn apply_to_vecs<T: Clone>(
        &self,
        containers: &mut [(DragId, Vec<T>)],
        get_id: impl Fn(&T) -> DragId,
    ) -> bool {
        // Find source and destination indices
        let src_idx = containers
            .iter()
            .position(|(id, _)| *id == self.from_container);
        let dst_idx = containers
            .iter()
            .position(|(id, _)| *id == self.to_container);

        match (src_idx, dst_idx) {
            (Some(si), Some(di)) => {
                // Remove from source
                let item = {
                    let src = &mut containers[si].1;
                    let idx = src.iter().position(|t| get_id(t) == self.item_id);
                    idx.map(|i| src.remove(i))
                };

                // Insert into destination at to_index
                if let Some(item) = item {
                    let dst = &mut containers[di].1;
                    let len = dst.len();
                    dst.insert(self.to_index.min(len), item);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Event fired when an item is dropped onto another item (merge/group action)
///
/// This event is fired when `enable_merge` is true on `SortableGroup` and the
/// user drops an item in the center 40% zone of another item (IntoItem collision).
/// Use this for creating supersets, grouping items, or other merge operations.
///
/// This type captures merge intent (`item_id` onto `target_id`). The final data
/// mutation strategy is application-specific and should typically live in your
/// `on_merge` handler.
#[derive(Clone, Debug, PartialEq)]
pub struct MergeEvent {
    /// Container where the source item came from (for cross-container merges)
    pub from_container: DragId,
    /// Container containing the target item
    pub to_container: DragId,
    /// The item being dragged
    pub item_id: DragId,
    /// The item that was dropped onto
    pub target_id: DragId,
}

impl MergeEvent {
    /// Create a new merge event
    pub fn new(
        from_container: impl Into<DragId>,
        to_container: impl Into<DragId>,
        item_id: impl Into<DragId>,
        target_id: impl Into<DragId>,
    ) -> Self {
        Self {
            from_container: from_container.into(),
            to_container: to_container.into(),
            item_id: item_id.into(),
            target_id: target_id.into(),
        }
    }

    /// Apply this merge event by moving the dragged item adjacent to the target.
    ///
    /// This method:
    /// 1. Removes the dragged item from its current position
    /// 2. Calls `set_parent` to update the item's parent/group membership
    /// 3. Inserts the item immediately after the target
    ///
    /// This is a convenience helper for adjacency-style merge behavior
    /// (`[target, source]`). For richer grouped-list lifecycle behavior
    /// (header creation, orphan cleanup, nested conventions), prefer the
    /// `grouped_merge*` helpers in `crate::grouped`.
    ///
    /// For complex merge scenarios, perform your domain-specific mutations
    /// directly in `on_merge` instead of relying on this default insertion policy.
    ///
    /// # Arguments
    ///
    /// * `items` - Signal containing the items
    /// * `get_id` - A function that extracts a DragId from an item
    /// * `set_parent` - A function that sets the parent/group on an item
    ///
    /// # Returns
    ///
    /// Returns `true` if the merge was successfully applied.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableGroup {
    ///     enable_merge: true,
    ///     on_merge: move |e: MergeEvent| {
    ///         // Optional: create group header before apply if needed
    ///         let group_id = ensure_group_exists(&mut items, &e.target_id);
    ///
    ///         e.apply(
    ///             &tasks,
    ///             |t: &Task| t.id(),
    ///             |t: &mut Task, _target| { t.group_id = Some(group_id.clone()); },
    ///         );
    ///     },
    /// }
    /// ```
    pub fn apply<T: Clone + 'static>(
        &self,
        items: &dioxus::prelude::Signal<Vec<T>>,
        get_id: impl Fn(&T) -> DragId,
        set_parent: impl FnOnce(&mut T, Option<DragId>),
    ) -> bool {
        let mut items = *items;
        let mut items = items.write();
        self.apply_to_vec(&mut items, get_id, set_parent)
    }

    /// Apply this merge event to a plain Vec (no Signal dependency).
    ///
    /// Convenience helper with the same default policy as [`Self::apply`]:
    /// remove dragged item, call `set_parent`, and insert immediately after target.
    pub fn apply_to_vec<T: Clone>(
        &self,
        items: &mut Vec<T>,
        get_id: impl Fn(&T) -> DragId,
        set_parent: impl FnOnce(&mut T, Option<DragId>),
    ) -> bool {
        // Find and remove the dragged item
        let dragged_idx = items.iter().position(|t| get_id(t) == self.item_id);
        let dragged_item = dragged_idx.map(|idx| items.remove(idx));

        if let Some(mut item) = dragged_item {
            // Apply the parent/group update
            set_parent(&mut item, Some(self.target_id.clone()));

            // Find target position (after removal, indices may have shifted)
            let target_idx = items.iter().position(|t| get_id(t) == self.target_id);

            // Insert after target (or at end if target not found)
            let insert_idx = target_idx.map(|idx| idx + 1).unwrap_or(items.len());
            items.insert(insert_idx, item);
            true
        } else {
            false
        }
    }
}

/// Orientation for sortable lists
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum Orientation {
    /// Items are arranged vertically (default)
    #[default]
    Vertical,
    /// Items are arranged horizontally
    Horizontal,
}

// ============================================================================
// Announcement Events (Keyboard Drag Accessibility)
// ============================================================================

/// Structured announcement events for keyboard drag-and-drop.
///
/// Fired at key lifecycle points during keyboard drags. Consumers can
/// provide a callback (`on_announce` on `DragContextProvider`) to override
/// the default English text for i18n.
///
/// If no callback is provided, `default_text()` produces English strings
/// suitable for ARIA live regions.
#[derive(Clone, Debug, PartialEq)]
pub enum AnnouncementEvent {
    /// Item was grabbed via keyboard (Space/Enter)
    Grabbed {
        item_id: DragId,
        position: usize,
        total: usize,
        container_id: DragId,
    },
    /// Item was moved within the same container (Arrow keys)
    Moved {
        item_id: DragId,
        position: usize,
        total: usize,
        container_id: DragId,
    },
    /// Item was moved to a different container (Tab)
    MovedToContainer {
        item_id: DragId,
        position: usize,
        total: usize,
        container_id: DragId,
    },
    /// Item was dropped (Space/Enter during drag)
    Dropped {
        item_id: DragId,
        position: usize,
        total: usize,
        container_id: DragId,
    },
    /// Drag was cancelled (Escape)
    Cancelled { item_id: DragId },
}

impl AnnouncementEvent {
    /// Default English announcement text for ARIA live regions.
    pub fn default_text(&self) -> String {
        match self {
            AnnouncementEvent::Grabbed {
                position, total, ..
            } => format!("Grabbed item, position {position} of {total}"),
            AnnouncementEvent::Moved {
                position, total, ..
            } => format!("Position {position} of {total}"),
            AnnouncementEvent::MovedToContainer {
                position,
                total,
                container_id,
                ..
            } => format!("Moved to container {container_id}, position {position} of {total}"),
            AnnouncementEvent::Dropped {
                position, total, ..
            } => format!("Item dropped, position {position} of {total}"),
            AnnouncementEvent::Cancelled { .. } => {
                "Drag cancelled, item returned to start".to_string()
            }
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
    fn test_drag_id_creation() {
        let id1 = DragId::new("test");
        let id2 = DragId::from("test");
        let id3: DragId = "test".into();

        assert_eq!(id1, id2);
        assert_eq!(id2, id3);
    }

    #[test]
    fn test_drag_id_display() {
        let id = DragId::new("my-item");
        assert_eq!(format!("{}", id), "my-item");
        assert_eq!(id.to_string(), "my-item");
    }

    #[test]
    fn test_drag_type_display() {
        let dt = DragType::new("sortable");
        assert_eq!(format!("{}", dt), "sortable");
        assert_eq!(dt.to_string(), "sortable");
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(3.0, 4.0);

        assert!((p1.distance_to(p2) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 10.0, 100.0, 100.0);

        assert!(rect.contains(Position::new(50.0, 50.0)));
        assert!(rect.contains(Position::new(10.0, 10.0)));
        assert!(rect.contains(Position::new(110.0, 110.0)));
        assert!(!rect.contains(Position::new(5.0, 50.0)));
        assert!(!rect.contains(Position::new(50.0, 5.0)));
    }

    #[test]
    fn test_rect_center() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let center = rect.center();

        assert_eq!(center.x, 50.0);
        assert_eq!(center.y, 50.0);
    }

    #[test]
    fn test_rect_expanded() {
        let rect = Rect::new(100.0, 200.0, 50.0, 80.0);
        let expanded = rect.expanded(10.0);

        assert_eq!(expanded.x, 90.0);
        assert_eq!(expanded.y, 190.0);
        assert_eq!(expanded.width, 70.0);
        assert_eq!(expanded.height, 100.0);

        // Original point just outside should now be inside expanded rect
        assert!(!rect.contains(Position::new(95.0, 205.0)));
        assert!(expanded.contains(Position::new(95.0, 205.0)));
    }

    #[test]
    fn test_rect_expanded_zero_is_identity() {
        let rect = Rect::new(10.0, 20.0, 100.0, 200.0);
        let expanded = rect.expanded(0.0);

        assert_eq!(expanded.x, rect.x);
        assert_eq!(expanded.y, rect.y);
        assert_eq!(expanded.width, rect.width);
        assert_eq!(expanded.height, rect.height);
    }

    #[test]
    fn test_drop_location_container_id() {
        let container = DragId::new("container");

        let loc1 = DropLocation::AtIndex {
            container_id: container.clone(),
            index: 0,
        };
        let loc2 = DropLocation::IntoContainer {
            container_id: container.clone(),
        };
        assert_eq!(loc1.container_id(), container);
        assert_eq!(loc2.container_id(), container);
    }

    #[test]
    fn test_drop_location_contains_id() {
        let container = DragId::new("container");
        let other = DragId::new("other");

        let loc = DropLocation::AtIndex {
            container_id: container.clone(),
            index: 2,
        };

        assert!(loc.contains_id(&container));
        assert!(!loc.contains_id(&other));
    }

    #[test]
    fn test_orientation_default() {
        let orientation = Orientation::default();
        assert_eq!(orientation, Orientation::Vertical);
    }

    #[test]
    fn test_reorder_event_to_index_direct() {
        // to_index is used directly (filtered-list convention)
        let event = ReorderEvent {
            container_id: DragId::new("list"),
            from_index: 0,
            to_index: 2,
            item_id: DragId::new("a"),
        };
        assert_eq!(event.to_index, 2);
    }

    #[test]
    fn test_move_event_to_index_direct() {
        // to_index is used directly (filtered-list convention)
        let event = MoveEvent {
            item_id: DragId::new("item-x"),
            from_container: DragId::new("list-a"),
            from_index: 0,
            to_container: DragId::new("list-b"),
            to_index: 1,
        };
        assert_eq!(event.to_index, 1);
    }

    // =========================================================================
    // Multi-type DragData tests (TDD - tests written before implementation)
    // =========================================================================

    #[test]
    fn test_drag_data_with_multiple_types() {
        // DragData should support multiple drag types
        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        assert_eq!(data.id, DragId::new("item-1"));
        assert_eq!(data.drag_types.len(), 2);
        assert!(data.drag_types.contains(&DragType::new("sortable")));
        assert!(data.drag_types.contains(&DragType::new("image")));
    }

    #[test]
    fn test_drag_data_new_creates_single_type_vec() {
        // Backward compatibility: DragData::new should still work
        // and internally store the type in drag_types vec
        let data = DragData::new("item-1", "sortable");

        assert_eq!(data.id, DragId::new("item-1"));
        assert_eq!(data.drag_types.len(), 1);
        assert!(data.drag_types.contains(&DragType::new("sortable")));
    }

    #[test]
    fn test_drag_data_has_type() {
        // DragData should have a helper method to check if it has a specific type
        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        assert!(data.has_type(&DragType::new("sortable")));
        assert!(data.has_type(&DragType::new("image")));
        assert!(!data.has_type(&DragType::new("document")));
    }

    #[test]
    fn test_drag_data_primary_type() {
        // DragData should expose the first type as "primary" for backward compat
        let data = DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );

        // primary_type() returns the first type (or a default)
        assert_eq!(data.primary_type(), &DragType::new("sortable"));
    }

    #[test]
    fn test_drag_data_primary_type_from_new() {
        let data = DragData::new("item-1", "task");
        assert_eq!(data.primary_type(), &DragType::new("task"));
    }

    // =========================================================================
    // combine_drag_types utility tests
    // =========================================================================

    #[test]
    fn test_combine_drag_types_with_primary_only() {
        let primary = DragType::new("task");
        let types = combine_drag_types(Some(&primary), &[], "");

        assert_eq!(types.len(), 1);
        assert_eq!(types[0], DragType::new("task"));
    }

    #[test]
    fn test_combine_drag_types_with_additional_only() {
        let additional = vec![DragType::new("image"), DragType::new("media")];
        let types = combine_drag_types(None, &additional, "sortable");

        assert_eq!(types.len(), 2);
        assert_eq!(types[0], DragType::new("image"));
        assert_eq!(types[1], DragType::new("media"));
    }

    #[test]
    fn test_combine_drag_types_with_primary_and_additional() {
        let primary = DragType::new("sortable");
        let additional = vec![DragType::new("image")];
        let types = combine_drag_types(Some(&primary), &additional, "");

        assert_eq!(types.len(), 2);
        assert_eq!(types[0], DragType::new("sortable"));
        assert_eq!(types[1], DragType::new("image"));
    }

    #[test]
    fn test_combine_drag_types_uses_default_when_empty() {
        let types = combine_drag_types(None, &[], "sortable");

        assert_eq!(types.len(), 1);
        assert_eq!(types[0], DragType::new("sortable"));
    }

    #[test]
    fn test_combine_drag_types_uses_empty_default() {
        let types = combine_drag_types(None, &[], "");

        assert_eq!(types.len(), 1);
        assert_eq!(types[0], DragType::new(""));
    }

    // =========================================================================
    // DropLocation::IntoItem tests
    // =========================================================================

    #[test]
    fn test_drop_location_into_item_container_id() {
        let container = DragId::new("list");
        let item = DragId::new("item-1");
        let location = DropLocation::IntoItem {
            container_id: container.clone(),
            item_id: item.clone(),
        };

        assert_eq!(location.container_id(), container);
    }

    #[test]
    fn test_drop_location_into_item_contains_id() {
        let container = DragId::new("list");
        let item = DragId::new("item-1");
        let other = DragId::new("other");

        let location = DropLocation::IntoItem {
            container_id: container.clone(),
            item_id: item.clone(),
        };

        assert!(location.contains_id(&container));
        assert!(location.contains_id(&item));
        assert!(!location.contains_id(&other));
    }

    // =========================================================================
    // is_container_targeted / is_item_targeted tests
    // =========================================================================

    #[test]
    fn test_at_index_is_container_targeted() {
        let location = DropLocation::AtIndex {
            container_id: DragId::new("main"),
            index: 1,
        };

        assert!(
            location.is_container_targeted(&DragId::new("main")),
            "AtIndex location should target the container"
        );
        assert!(
            !location.is_item_targeted(&DragId::new("main")),
            "AtIndex location should not target items"
        );
    }

    #[test]
    fn test_is_container_targeted_into_item_variant() {
        let location = DropLocation::IntoItem {
            container_id: DragId::new("list"),
            item_id: DragId::new("target"),
        };

        assert!(
            !location.is_container_targeted(&DragId::new("target")),
            "IntoItem location should not target as container"
        );
        assert!(
            location.is_item_targeted(&DragId::new("target")),
            "IntoItem location should target the item"
        );
    }

    #[test]
    fn test_is_container_targeted_at_index_variant() {
        let location = DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 5,
        };

        assert!(
            location.is_container_targeted(&DragId::new("list")),
            "AtIndex location should target the container"
        );
        assert!(
            !location.is_item_targeted(&DragId::new("list")),
            "AtIndex location should not target items"
        );
    }

    #[test]
    fn test_is_container_targeted_into_container_variant() {
        let location = DropLocation::IntoContainer {
            container_id: DragId::new("dropzone"),
        };

        assert!(
            location.is_container_targeted(&DragId::new("dropzone")),
            "IntoContainer location should target the container"
        );
        assert!(
            !location.is_item_targeted(&DragId::new("dropzone")),
            "IntoContainer location should not target items"
        );
    }

    // =========================================================================
    // apply_to_vec / apply_to_vecs tests (no Dioxus runtime required)
    // =========================================================================

    #[test]
    fn test_reorder_event_apply_to_vec() {
        let mut items = vec!["a", "b", "c", "d"];
        let event = ReorderEvent {
            container_id: DragId::new("list"),
            from_index: 0,
            to_index: 2, // filtered: after removing "a", insert at position 2
            item_id: DragId::new("a"),
        };

        let result = event.apply_to_vec(&mut items, |s| DragId::new(*s));
        assert!(result);
        // "a" removed from index 0 -> [b, c, d], then inserted at index 2 -> [b, c, a, d]
        assert_eq!(items, vec!["b", "c", "a", "d"]);
    }

    #[test]
    fn test_move_event_apply_to_vecs() {
        let mut containers = vec![
            (DragId::new("src"), vec!["a", "b"]),
            (DragId::new("dst"), vec!["x", "y"]),
        ];
        let event = MoveEvent {
            item_id: DragId::new("b"),
            from_container: DragId::new("src"),
            from_index: 1,
            to_container: DragId::new("dst"),
            to_index: 0, // insert at position 0 in destination
        };

        let result = event.apply_to_vecs(&mut containers, |s| DragId::new(*s));
        assert!(result);
        assert_eq!(containers[0].1, vec!["a"]);
        assert_eq!(containers[1].1, vec!["b", "x", "y"]);
    }

    #[test]
    fn test_merge_event_apply_to_vec() {
        let mut items: Vec<(String, Option<String>)> =
            vec![("a".into(), None), ("b".into(), None), ("c".into(), None)];
        let event = MergeEvent {
            from_container: DragId::new("list"),
            to_container: DragId::new("list"),
            item_id: DragId::new("a"),
            target_id: DragId::new("c"),
        };

        let result = event.apply_to_vec(
            &mut items,
            |t| DragId::new(&t.0),
            |t, parent| {
                t.1 = parent.map(|p| p.to_string());
            },
        );
        assert!(result);
        // "a" removed from index 0, inserted after "c" (now at index 1) -> index 2
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].0, "b");
        assert_eq!(items[1].0, "c");
        assert_eq!(items[2].0, "a");
        assert_eq!(items[2].1, Some("c".to_string()));
    }

    #[test]
    fn test_drop_event_apply_to_vecs() {
        let mut containers = vec![
            (DragId::new("src"), vec!["a", "b"]),
            (DragId::new("dst"), vec!["x"]),
        ];
        let event = DropEvent {
            dragged: DragData::new("a", "type"),
            location: DropLocation::IntoContainer {
                container_id: DragId::new("dst"),
            },
            source: DragId::new("a"),
            source_container: Some(DragId::new("src")),
            source_index: Some(0),
        };

        let result = event.apply_to_vecs(&mut containers, |s| DragId::new(*s));
        assert!(result);
        assert_eq!(containers[0].1, vec!["b"]);
        assert_eq!(containers[1].1, vec!["x", "a"]);
    }

    // =========================================================================
    // DropEvent source_index tests
    // =========================================================================

    #[test]
    fn test_drop_event_has_source_index_field() {
        let event = DropEvent {
            dragged: DragData::new("a", "type"),
            location: DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            },
            source: DragId::new("a"),
            source_container: Some(DragId::new("list")),
            source_index: Some(0),
        };
        assert_eq!(event.source_index, Some(0));
    }

    #[test]
    fn test_drop_event_source_index_none() {
        let event = DropEvent {
            dragged: DragData::new("a", "type"),
            location: DropLocation::IntoContainer {
                container_id: DragId::new("dst"),
            },
            source: DragId::new("a"),
            source_container: Some(DragId::new("src")),
            source_index: None,
        };
        assert_eq!(event.source_index, None);
    }

    #[test]
    fn test_drop_event_new_with_source_index() {
        let event = DropEvent::new(
            DragData::new("item-1", "sortable"),
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 3,
            },
            "item-1",
            Some(DragId::new("list")),
            Some(1),
        );
        assert_eq!(event.source_index, Some(1));
        assert_eq!(event.source, DragId::new("item-1"));
    }

    // =========================================================================
    // AnnouncementEvent tests
    // =========================================================================

    #[test]
    fn test_announcement_grabbed_default_text() {
        let event = AnnouncementEvent::Grabbed {
            item_id: DragId::new("item-1"),
            position: 1,
            total: 5,
            container_id: DragId::new("list"),
        };
        assert_eq!(event.default_text(), "Grabbed item, position 1 of 5");
    }

    #[test]
    fn test_announcement_moved_default_text() {
        let event = AnnouncementEvent::Moved {
            item_id: DragId::new("item-1"),
            position: 3,
            total: 5,
            container_id: DragId::new("list"),
        };
        assert_eq!(event.default_text(), "Position 3 of 5");
    }

    #[test]
    fn test_announcement_moved_to_container_default_text() {
        let event = AnnouncementEvent::MovedToContainer {
            item_id: DragId::new("item-1"),
            position: 1,
            total: 3,
            container_id: DragId::new("done"),
        };
        assert_eq!(
            event.default_text(),
            "Moved to container done, position 1 of 3"
        );
    }

    #[test]
    fn test_announcement_dropped_default_text() {
        let event = AnnouncementEvent::Dropped {
            item_id: DragId::new("item-1"),
            position: 2,
            total: 5,
            container_id: DragId::new("list"),
        };
        assert_eq!(event.default_text(), "Item dropped, position 2 of 5");
    }

    #[test]
    fn test_announcement_cancelled_default_text() {
        let event = AnnouncementEvent::Cancelled {
            item_id: DragId::new("item-1"),
        };
        assert_eq!(
            event.default_text(),
            "Drag cancelled, item returned to start"
        );
    }

    #[test]
    fn test_announcement_event_equality() {
        let a = AnnouncementEvent::Moved {
            item_id: DragId::new("x"),
            position: 1,
            total: 3,
            container_id: DragId::new("list"),
        };
        let b = AnnouncementEvent::Moved {
            item_id: DragId::new("x"),
            position: 1,
            total: 3,
            container_id: DragId::new("list"),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_announcement_position_changes_text() {
        let at_1 = AnnouncementEvent::Moved {
            item_id: DragId::new("x"),
            position: 1,
            total: 5,
            container_id: DragId::new("list"),
        };
        let at_3 = AnnouncementEvent::Moved {
            item_id: DragId::new("x"),
            position: 3,
            total: 5,
            container_id: DragId::new("list"),
        };
        assert_ne!(at_1.default_text(), at_3.default_text());
        assert!(at_1.default_text().contains("1 of 5"));
        assert!(at_3.default_text().contains("3 of 5"));
    }
}
