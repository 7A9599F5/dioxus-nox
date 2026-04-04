//! Convenience helpers for flat, grouped lists.
//!
//! This module provides helpers for building grouped lists with headers and
//! members stored in a single flat sequence. It handles:
//! - Moving entire groups by dragging their headers
//! - Moving items into/out of groups while reordering
//! - Creating groups via merge events
//! - Cleaning up groups that no longer have enough members

use crate::context::DragContext;
use crate::types::{DragId, MergeEvent, MoveEvent, ReorderEvent};
use crate::utils::find_contiguous_block;
use dioxus::prelude::*;
use std::collections::HashMap;

/// Trait for flat list items that can participate in grouping.
///
/// Implement this for your item enum or struct to gain access to the
/// grouped-list helper methods.
pub trait GroupedItem: Clone {
    /// The identifier type for groups (supersets, sections, etc.).
    type GroupId: Clone + Eq + std::hash::Hash;

    /// Return the drag id for this item.
    fn drag_id(&self) -> DragId;

    /// Return the group id for this item, if it belongs to a group.
    fn group_id(&self) -> Option<&Self::GroupId>;

    /// Whether this item is the group header.
    fn is_group_header(&self) -> bool;

    /// Set the group id for a member item.
    fn set_group_id(&mut self, group_id: Option<Self::GroupId>);

    /// Construct a new header item for the given group id.
    fn make_group_header(_group_id: Self::GroupId) -> Self {
        panic!(
            "GroupedItem::make_group_header must be implemented or make_group_header_with must be overridden"
        )
    }

    /// Construct a new header item using item-specific configuration.
    fn make_group_header_with(&self, group_id: Self::GroupId) -> Self {
        Self::make_group_header(group_id)
    }
}

/// Adapter for grouped list operations on a flat `Vec<T>`.
pub struct GroupedList<'a, T: GroupedItem> {
    items: &'a mut Vec<T>,
}

/// Default minimum number of members before a group is dissolved.
pub const DEFAULT_MIN_GROUP_MEMBERS: usize = 2;

/// Generate a default group id using UUID v7.
pub fn default_group_id() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let millis = js_sys::Date::now() as u64;
        let secs = millis / 1000;
        let nanos = ((millis % 1000) * 1_000_000) as u32;
        let ts = uuid::Timestamp::from_unix(uuid::NoContext, secs, nanos);
        format!("group-{}", uuid::Uuid::new_v7(ts))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("group-{}", uuid::Uuid::now_v7())
    }
}

/// Active group header drag metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveGroupHeader<GroupId> {
    /// The group id for the active header drag.
    pub group_id: GroupId,
    /// The drag id for the active header item.
    pub header_id: DragId,
}

/// Return the active group header drag, if any.
pub fn active_group_header<T: GroupedItem>(
    ctx: &DragContext,
    items: &[T],
) -> Option<ActiveGroupHeader<T::GroupId>> {
    let active = ctx.get_active_drag()?;
    let header = items
        .iter()
        .find(|item| item.drag_id() == active.data.id && item.is_group_header())?;
    let group_id = header.group_id().cloned()?;
    Some(ActiveGroupHeader {
        group_id,
        header_id: active.data.id,
    })
}

/// Apply grouped reorder logic with default settings.
pub fn grouped_reorder_default<T: GroupedItem<GroupId = String> + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &ReorderEvent,
) -> bool {
    grouped_reorder(items, event, DEFAULT_MIN_GROUP_MEMBERS)
}

/// Apply grouped merge logic with default settings.
pub fn grouped_merge_default<T: GroupedItem + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &MergeEvent,
    make_group_id: impl FnOnce() -> T::GroupId,
) -> bool {
    grouped_merge_with(items, event, DEFAULT_MIN_GROUP_MEMBERS, make_group_id)
}

/// Apply grouped merge logic with default UUID v7 group ids.
pub fn grouped_merge<T: GroupedItem<GroupId = String> + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &MergeEvent,
) -> bool {
    grouped_merge_with(items, event, DEFAULT_MIN_GROUP_MEMBERS, default_group_id)
}

/// Apply grouped reorder logic to a signal-backed list and clean up orphans.
pub fn grouped_reorder<T: GroupedItem<GroupId = String> + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &ReorderEvent,
    min_members: usize,
) -> bool {
    let mut items = *items;
    let mut items = items.write();
    let mut grouped = GroupedList::new(&mut items);
    let changed = grouped.reorder(event);
    if changed {
        grouped.cleanup_orphaned_groups(min_members);
    }
    changed
}

/// Apply grouped merge logic to a signal-backed list and clean up orphans.
///
/// For the common case with UUID v7 group ids, use [`grouped_merge`] instead.
pub fn grouped_merge_with<T: GroupedItem + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &MergeEvent,
    min_members: usize,
    make_group_id: impl FnOnce() -> T::GroupId,
) -> bool {
    let mut items = *items;
    let mut items = items.write();

    let mut grouped = GroupedList::new(&mut items);
    let changed = grouped.merge_with(event, make_group_id);
    if changed {
        grouped.cleanup_orphaned_groups(min_members);
    }

    changed
}

// ============================================================================
// Container ID Convention
// ============================================================================

/// Suffix appended to group IDs to form nested container IDs.
///
/// A group with id `"superset-1"` registers its inner container as
/// `"superset-1-container"`.
pub const CONTAINER_SUFFIX: &str = "-container";

/// Extract a group ID from a container DragId by stripping the `-container` suffix.
///
/// Returns `None` if the ID does not end with [`CONTAINER_SUFFIX`].
pub fn group_id_from_container(container_id: &DragId) -> Option<String> {
    container_id
        .0
        .strip_suffix(CONTAINER_SUFFIX)
        .map(|s| s.to_string())
}

// ============================================================================
// Top-Level Entry (rendering partition)
// ============================================================================

/// A top-level entry in a grouped list: either a group (header + members) or
/// a standalone item.
///
/// Used with [`partition_grouped_items`] to convert a flat item list into a
/// structure suitable for rendering with nested `SortableContext` components.
#[derive(Clone, Debug, PartialEq)]
pub enum TopLevelEntry<T: GroupedItem> {
    /// A group consisting of a header and its contiguous members.
    Group {
        /// The group identifier.
        group_id: T::GroupId,
        /// All items in the group (header first, then members).
        items: Vec<T>,
    },
    /// A standalone item that does not belong to any group.
    Standalone(T),
}

impl<T: GroupedItem<GroupId = String>> TopLevelEntry<T> {
    /// Return the [`DragId`] for this entry.
    ///
    /// For groups, this is the group ID (matching the nested container's item
    /// zone in the parent). For standalone items, this is the item's drag ID.
    pub fn drag_id(&self) -> DragId {
        match self {
            TopLevelEntry::Group { group_id, .. } => DragId::new(group_id),
            TopLevelEntry::Standalone(item) => item.drag_id(),
        }
    }

    /// Returns the drag IDs of items inside a group (for the nested
    /// `SortableContext`'s `items` prop). Returns an empty vec for standalone.
    pub fn group_item_ids(&self) -> Vec<DragId> {
        match self {
            TopLevelEntry::Group { items, .. } => items.iter().map(|i| i.drag_id()).collect(),
            TopLevelEntry::Standalone(_) => Vec::new(),
        }
    }
}

impl<T: GroupedItem> TopLevelEntry<T> {
    /// Whether this entry is a group.
    pub fn is_group(&self) -> bool {
        matches!(self, TopLevelEntry::Group { .. })
    }

    /// Return group contents if this is a group entry.
    pub fn as_group(&self) -> Option<(&T::GroupId, &[T])> {
        match self {
            TopLevelEntry::Group { group_id, items } => Some((group_id, items)),
            TopLevelEntry::Standalone(_) => None,
        }
    }

    /// Return the item if this is a standalone entry.
    pub fn as_standalone(&self) -> Option<&T> {
        match self {
            TopLevelEntry::Standalone(item) => Some(item),
            TopLevelEntry::Group { .. } => None,
        }
    }
}

/// Partition a flat list of grouped items into [`TopLevelEntry`] values.
///
/// Iterates through items sequentially. When a group header is encountered,
/// it collects the header plus all contiguous following members with the same
/// group ID into a `Group` entry. All other items become `Standalone` entries.
pub fn partition_grouped_items<T: GroupedItem>(items: &[T]) -> Vec<TopLevelEntry<T>> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < items.len() {
        if items[i].is_group_header()
            && let Some(group_id) = items[i].group_id().cloned()
        {
            let mut group_items = vec![items[i].clone()];
            i += 1;
            while i < items.len() {
                if items[i].group_id() == Some(&group_id) && !items[i].is_group_header() {
                    group_items.push(items[i].clone());
                    i += 1;
                } else {
                    break;
                }
            }
            result.push(TopLevelEntry::Group {
                group_id,
                items: group_items,
            });
            continue;
        }
        result.push(TopLevelEntry::Standalone(items[i].clone()));
        i += 1;
    }

    result
}

// ============================================================================
// Flat Insert Position
// ============================================================================

/// Find the insertion index in a flat item list given a container-level index.
///
/// Maps from a position in the container's top-level item list (group IDs +
/// standalone IDs) to the corresponding position in the flat data list.
///
/// - If `index < container_items.len()`: finds `container_items[index]` in
///   `items`. If it matches a group header, returns the position of the
///   group's first item (the header).
/// - If `index >= container_items.len()`: returns `items.len()` (append).
pub fn find_flat_insert_position<T: GroupedItem<GroupId = String>>(
    items: &[T],
    container_items: &[DragId],
    index: usize,
) -> usize {
    if index >= container_items.len() {
        return items.len();
    }

    let target_id = &container_items[index];

    // Direct item match
    if let Some(idx) = items.iter().position(|i| i.drag_id() == *target_id) {
        return idx;
    }

    // target_id might be a group ID (nested container's item zone in parent)
    // Find the group's first item (header) position
    let group_id = target_id.as_str();
    items
        .iter()
        .position(|i| i.group_id().is_some_and(|g| g.as_str() == group_id))
        .unwrap_or(items.len())
}

// ============================================================================
// Cross-Container Move
// ============================================================================

/// Apply grouped cross-container move logic with default settings.
///
/// Handles both group header drags (moves entire group block) and regular
/// item drags (updates group membership). Cleans up orphaned groups afterward.
///
/// Returns `true` if the list was modified.
pub fn grouped_move_default<T: GroupedItem<GroupId = String> + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &MoveEvent,
) -> bool {
    grouped_move(items, event, DEFAULT_MIN_GROUP_MEMBERS)
}

/// Apply grouped cross-container move logic to a signal-backed list.
///
/// - **Group header drag**: collects the entire group block (header + members),
///   removes it, and reinserts at the target position.
/// - **Regular item drag**: removes the item, updates its group ID based on
///   the target container (using [`group_id_from_container`]), and inserts at
///   the target position.
///
/// Cleans up orphaned groups (fewer than `min_members`) after the move.
///
/// Returns `true` if the list was modified.
pub fn grouped_move<T: GroupedItem<GroupId = String> + 'static>(
    items: &dioxus::prelude::Signal<Vec<T>>,
    event: &MoveEvent,
    min_members: usize,
) -> bool {
    let mut items = *items;
    let mut items = items.write();

    let from_idx = items.iter().position(|i| i.drag_id() == event.item_id);
    let Some(from) = from_idx else {
        return false;
    };

    // Group header drag → move entire group block
    if items[from].is_group_header()
        && let Some(group_id) = items[from].group_id().cloned()
    {
        // Collect all items in this group (header + members)
        let mut group_items = Vec::new();
        let mut indices_to_remove = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if item.group_id() == Some(&group_id) {
                group_items.push(item.clone());
                indices_to_remove.push(i);
            }
        }

        // Remove all group items (reverse to preserve indices)
        for index in indices_to_remove.into_iter().rev() {
            items.remove(index);
        }

        // Insert entire group at new position.
        // Derive container items from remaining flat list to map to_index.
        let container_items: Vec<DragId> = partition_grouped_items(&items)
            .iter()
            .map(|e| e.drag_id())
            .collect();
        let to =
            find_flat_insert_position(&items, &container_items, event.to_index).min(items.len());

        for (offset, item) in group_items.into_iter().enumerate() {
            items.insert(to + offset, item);
        }
        return true;
    }

    // Regular item cross-container move
    let mut item = items.remove(from);

    // Determine target group from container ID
    let to_group = group_id_from_container(&event.to_container);
    item.set_group_id(to_group);

    // Find insert position by mapping container-level to_index to flat list.
    // If moving into a group container, derive items for that container.
    // If moving to the parent container, derive top-level entries.
    let to = if let Some(gid) = group_id_from_container(&event.to_container) {
        // Moving into a group container: container items are the group's members
        let container_items: Vec<DragId> = items
            .iter()
            .filter(|i| i.group_id().map(|g| g.as_str()) == Some(gid.as_str()))
            .map(|i| i.drag_id())
            .collect();
        find_flat_insert_position(&items, &container_items, event.to_index)
    } else {
        // Moving to parent container: container items are top-level entries
        let container_items: Vec<DragId> = partition_grouped_items(&items)
            .iter()
            .map(|e| e.drag_id())
            .collect();
        find_flat_insert_position(&items, &container_items, event.to_index)
    }
    .min(items.len());

    items.insert(to, item);

    // Cleanup orphaned groups
    GroupedList::new(&mut *items).cleanup_orphaned_groups(min_members);
    true
}

// ============================================================================
// Rendering Helpers
// ============================================================================

/// Position metadata for a grouped list item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupedPosition {
    /// Whether this item is a group header.
    pub is_header: bool,
    /// Whether this item is a member (non-header) item.
    pub is_member: bool,
    /// Whether this item belongs to a group.
    pub is_grouped: bool,
    /// Whether this item is the first member in its group (or standalone).
    pub is_first_member: bool,
    /// Whether this item is the last member in its group (or standalone).
    pub is_last_member: bool,
}

/// Styling helpers for grouped list items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupedStyleInfo {
    /// Class names to apply for grouped styling.
    pub class_name: String,
    /// Value for the `data-group-role` attribute.
    pub data_group_role: Option<&'static str>,
    /// Value for the `data-group-pos` attribute.
    pub data_group_pos: Option<&'static str>,
}

/// Compute grouped position metadata for an item at the given index.
pub fn grouped_position<T: GroupedItem>(items: &[T], index: usize) -> GroupedPosition {
    let Some(item) = items.get(index) else {
        return GroupedPosition::default();
    };

    let is_header = item.is_group_header();
    let is_member = !is_header;
    let group_id = item.group_id();
    let is_grouped = group_id.is_some();

    let is_first_member = if !is_member {
        false
    } else if !is_grouped {
        true
    } else {
        match items.get(index.saturating_sub(1)) {
            Some(prev) => prev.group_id() != group_id || prev.is_group_header(),
            None => true,
        }
    };

    let is_last_member = if !is_member {
        false
    } else if !is_grouped {
        true
    } else {
        match items.get(index + 1) {
            Some(next) => next.group_id() != group_id || next.is_group_header(),
            None => true,
        }
    };

    GroupedPosition {
        is_header,
        is_member,
        is_grouped,
        is_first_member,
        is_last_member,
    }
}

/// Build grouped classes and data attributes from a grouped position.
pub fn grouped_style_info(position: GroupedPosition) -> GroupedStyleInfo {
    let mut classes = Vec::new();
    let mut data_group_role = None;
    let mut data_group_pos = None;

    if position.is_header {
        classes.push("grouped-header");
        data_group_role = Some("header");
    }

    if position.is_member {
        classes.push("grouped-member");
        data_group_role = Some("member");

        if position.is_first_member {
            classes.push("grouped-member-first");
        }
        if position.is_last_member {
            classes.push("grouped-member-last");
        }

        data_group_pos = Some(if position.is_first_member && position.is_last_member {
            "only"
        } else if position.is_first_member {
            "first"
        } else if position.is_last_member {
            "last"
        } else {
            "middle"
        });
    }

    GroupedStyleInfo {
        class_name: classes.join(" "),
        data_group_role,
        data_group_pos,
    }
}

impl<'a, T: GroupedItem> GroupedList<'a, T> {
    /// Create a new grouped list adapter for the given items.
    pub fn new(items: &'a mut Vec<T>) -> Self {
        Self { items }
    }

    /// Apply a merge event to a grouped list.
    ///
    /// If the target isn't already in a group, a new group header is inserted
    /// above it using `make_group_id`, then the dragged item is inserted after
    /// the target and assigned to the group.
    pub fn merge_with(
        &mut self,
        event: &MergeEvent,
        make_group_id: impl FnOnce() -> T::GroupId,
    ) -> bool {
        let dragged_idx = self
            .items
            .iter()
            .position(|item| item.drag_id() == event.item_id);

        let Some(dragged_idx) = dragged_idx else {
            return false;
        };

        if self.items[dragged_idx].is_group_header() || event.item_id == event.target_id {
            return false;
        }

        let group_id = if let Some(existing_group) = self.group_for_item(&event.target_id) {
            existing_group
        } else {
            let new_group_id = make_group_id();
            if let Some(target_idx) = self.find_index(&event.target_id)
                && let Some(header) = self
                    .items
                    .get(target_idx)
                    .map(|item| item.make_group_header_with(new_group_id.clone()))
            {
                self.items.insert(target_idx, header);
                if let Some(target_item) = self.items.get_mut(target_idx + 1) {
                    target_item.set_group_id(Some(new_group_id.clone()));
                }
            }
            new_group_id
        };

        let dragged_idx = self
            .items
            .iter()
            .position(|item| item.drag_id() == event.item_id);
        let Some(dragged_idx) = dragged_idx else {
            return false;
        };

        let mut item = self.items.remove(dragged_idx);
        item.set_group_id(Some(group_id.clone()));

        let target_idx = self.find_index(&event.target_id);
        let Some(target_idx) = target_idx else {
            self.items.insert(self.items.len(), item);
            return true;
        };

        if !self.items[target_idx].is_group_header()
            && self.items[target_idx].group_id() != Some(&group_id)
        {
            self.items[target_idx].set_group_id(Some(group_id.clone()));
        }

        let insert_idx =
            find_contiguous_block(self.items, target_idx, |item| item.group_id().cloned())
                .filter(|(block_group_id, _)| *block_group_id == group_id)
                .map(|(_, block_range)| (*block_range.end()).saturating_add(1))
                .unwrap_or_else(|| target_idx + 1);
        self.items.insert(insert_idx, item);
        true
    }

    /// Remove group headers and clear membership for orphaned groups.
    ///
    /// Any group with fewer than `min_members` (excluding the header) is dissolved.
    pub fn cleanup_orphaned_groups(&mut self, min_members: usize) {
        let mut group_counts: HashMap<T::GroupId, usize> = HashMap::new();

        for item in self.items.iter() {
            if item.is_group_header() {
                continue;
            }

            if let Some(group_id) = item.group_id() {
                *group_counts.entry(group_id.clone()).or_insert(0) += 1;
            }
        }

        let mut orphaned: Vec<T::GroupId> = group_counts
            .iter()
            .filter(|(_, count)| **count < min_members)
            .map(|(id, _)| id.clone())
            .collect();

        for item in self.items.iter() {
            if item.is_group_header()
                && let Some(group_id) = item.group_id()
                && !group_counts.contains_key(group_id)
            {
                orphaned.push(group_id.clone());
            }
        }

        for group_id in orphaned {
            for item in self.items.iter_mut() {
                if item.is_group_header() {
                    continue;
                }
                if item.group_id() == Some(&group_id) {
                    item.set_group_id(None);
                }
            }

            self.items
                .retain(|item| !(item.is_group_header() && item.group_id() == Some(&group_id)));
        }
    }

    fn group_for_item(&self, item_id: &DragId) -> Option<T::GroupId> {
        self.items
            .iter()
            .find(|item| item.drag_id() == *item_id)
            .and_then(|item| item.group_id().cloned())
    }

    fn group_for_insert_index(&self, index: usize) -> Option<T::GroupId> {
        if index == 0 || index > self.items.len() {
            return None;
        }

        let probe_index = index.saturating_sub(1);
        let (group_id, range) =
            find_contiguous_block(self.items, probe_index, |item| item.group_id().cloned())?;

        if self.block_has_header(&group_id, &range) {
            Some(group_id)
        } else {
            None
        }
    }

    fn group_block_for_insert_index(
        &self,
        index: usize,
    ) -> Option<(T::GroupId, std::ops::RangeInclusive<usize>)> {
        if index >= self.items.len() {
            return None;
        }

        let (group_id, range) =
            find_contiguous_block(self.items, index, |item| item.group_id().cloned())?;

        if index == *range.start() {
            return None;
        }

        if self.block_has_header(&group_id, &range) {
            Some((group_id, range))
        } else {
            None
        }
    }

    fn block_has_header(
        &self,
        group_id: &T::GroupId,
        range: &std::ops::RangeInclusive<usize>,
    ) -> bool {
        range.clone().any(|index| {
            let item = &self.items[index];
            item.is_group_header() && item.group_id() == Some(group_id)
        })
    }

    fn find_index(&self, item_id: &DragId) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.drag_id() == *item_id)
    }
}

// Reorder operations require GroupId = String because container-level item
// lists include group IDs (e.g., "superset-1") as entries, and resolving
// those to flat list positions requires comparing DragId strings against
// group IDs.
impl<'a, T: GroupedItem<GroupId = String>> GroupedList<'a, T> {
    /// Apply a reorder event to a grouped list.
    ///
    /// - Dragging a group header moves the entire group as a block.
    /// - Dragging a member item repositions it, updating group membership
    ///   based on the target position.
    ///
    /// Uses [`find_flat_insert_position`] to map the container-level `to_index`
    /// to a flat list position.
    pub fn reorder(&mut self, event: &ReorderEvent) -> bool {
        let from_index = self
            .items
            .iter()
            .position(|item| item.drag_id() == event.item_id);

        let Some(from_index) = from_index else {
            return false;
        };

        if self.items[from_index].is_group_header() {
            return self.reorder_group_header(event, from_index);
        }

        self.reorder_member(event, from_index)
    }

    fn reorder_group_header(&mut self, event: &ReorderEvent, from_index: usize) -> bool {
        let Some(group_id) = self.items[from_index].group_id().cloned() else {
            return false;
        };

        let mut group_items = Vec::new();
        let mut indices_to_remove = Vec::new();

        for (index, item) in self.items.iter().enumerate() {
            if item.group_id() == Some(&group_id) {
                group_items.push(item.clone());
                indices_to_remove.push(index);
            }
        }

        for index in indices_to_remove.into_iter().rev() {
            self.items.remove(index);
        }

        // Derive container items from the remaining flat list to map to_index
        // to a flat list position. Container items are top-level entries
        // (group IDs + standalone IDs).
        let container_items: Vec<DragId> = partition_grouped_items(self.items)
            .iter()
            .map(|e| e.drag_id())
            .collect();
        let mut to_index = find_flat_insert_position(self.items, &container_items, event.to_index)
            .min(self.items.len());

        // Snap to group block boundary if the target position lands inside
        // another group's block.
        if let Some((target_group_id, range)) = self.group_block_for_insert_index(to_index)
            && target_group_id != group_id
        {
            // Snap to start of the group block (before the group)
            to_index = *range.start();
        }

        for (offset, item) in group_items.into_iter().enumerate() {
            self.items.insert(to_index + offset, item);
        }

        true
    }

    fn reorder_member(&mut self, event: &ReorderEvent, from_index: usize) -> bool {
        let item = self.items.remove(from_index);
        let mut item = item;
        let was_in_group = item.group_id().is_some();

        // Map container-level to_index to flat list position.
        // If within a group container, derive that group's member IDs.
        // If parent-level, derive top-level entries.
        let within_group_id = group_id_from_container(&event.container_id);
        let is_within_group = within_group_id.is_some();
        let to_index = if let Some(gid) = within_group_id {
            // Within a group container: container items are that group's members
            let container_items: Vec<DragId> = self
                .items
                .iter()
                .filter(|i| i.group_id().map(|g| g.as_str()) == Some(gid.as_str()))
                .map(|i| i.drag_id())
                .collect();
            find_flat_insert_position(self.items, &container_items, event.to_index)
        } else {
            // Parent-level: container items are top-level entries
            let container_items: Vec<DragId> = partition_grouped_items(self.items)
                .iter()
                .map(|e| e.drag_id())
                .collect();
            find_flat_insert_position(self.items, &container_items, event.to_index)
        }
        .min(self.items.len());

        // Only auto-assign group membership when reordering WITHIN a group's
        // inner container (container_id ends with CONTAINER_SUFFIX, e.g.
        // "superset-1-container"). Parent-level reorders (container_id = "workout")
        // should never auto-assign — items dropped before/after a group via edge
        // zones or normalized collision targets must stay standalone. Group
        // membership only changes via on_move (cross-container moves).
        if is_within_group {
            let target_group = self.group_for_insert_index(to_index);
            if let Some(target_group) = target_group {
                item.set_group_id(Some(target_group));
            } else if was_in_group {
                item.set_group_id(None);
            }
        } else if was_in_group {
            item.set_group_id(None);
        }

        self.items.insert(to_index, item);
        true
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;
    use crate::types::MergeEvent;

    #[derive(Clone, Debug, PartialEq)]
    enum TestItem {
        Exercise { id: String, group: Option<String> },
        Header { id: String },
    }

    impl GroupedItem for TestItem {
        type GroupId = String;

        fn drag_id(&self) -> DragId {
            match self {
                TestItem::Exercise { id, .. } => DragId::new(id),
                TestItem::Header { id } => DragId::new(format!("header-{}", id)),
            }
        }

        fn group_id(&self) -> Option<&String> {
            match self {
                TestItem::Exercise { group, .. } => group.as_ref(),
                TestItem::Header { id } => Some(id),
            }
        }

        fn is_group_header(&self) -> bool {
            matches!(self, TestItem::Header { .. })
        }

        fn set_group_id(&mut self, group_id: Option<String>) {
            if let TestItem::Exercise { group, .. } = self {
                *group = group_id;
            }
        }

        fn make_group_header(group_id: String) -> Self {
            TestItem::Header { id: group_id }
        }
    }

    #[test]
    fn test_merge_two_standalone_creates_superset() {
        let mut items = vec![
            TestItem::Exercise {
                id: "1".into(),
                group: None,
            },
            TestItem::Exercise {
                id: "2".into(),
                group: None,
            },
            TestItem::Exercise {
                id: "3".into(),
                group: None,
            },
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = MergeEvent {
            item_id: DragId::new("2"),
            target_id: DragId::new("1"),
            from_container: DragId::new("list"),
            to_container: DragId::new("list"),
        };
        let changed = grouped.merge_with(&event, || "ss1".to_string());
        assert!(changed);
        assert_eq!(items.len(), 4); // header + 3 exercises
        assert!(items[0].is_group_header());
        assert_eq!(items[1].group_id(), Some(&"ss1".to_string()));
        assert_eq!(items[2].group_id(), Some(&"ss1".to_string()));
        assert_eq!(items[3].group_id(), None);
    }

    #[test]
    fn test_merge_into_existing_superset() {
        // Start with existing superset [header, ex1, ex2] + standalone ex3
        let mut items = vec![
            TestItem::Header { id: "ss1".into() },
            TestItem::Exercise {
                id: "1".into(),
                group: Some("ss1".into()),
            },
            TestItem::Exercise {
                id: "2".into(),
                group: Some("ss1".into()),
            },
            TestItem::Exercise {
                id: "3".into(),
                group: None,
            },
            TestItem::Exercise {
                id: "4".into(),
                group: None,
            },
        ];
        let mut grouped = GroupedList::new(&mut items);

        // Merge ex3 onto ex2 (which is in superset ss1)
        let event = MergeEvent {
            item_id: DragId::new("3"),
            target_id: DragId::new("2"),
            from_container: DragId::new("list"),
            to_container: DragId::new("list"),
        };
        let changed = grouped.merge_with(&event, || "ss2".to_string());

        assert!(changed, "merge_with should return true");
        // Ex3 should now be in the superset after ex2
        assert_eq!(items.len(), 5); // no new header
        assert_eq!(items[0].drag_id(), DragId::new("header-ss1"));
        assert_eq!(items[1].drag_id(), DragId::new("1"));
        assert_eq!(items[2].drag_id(), DragId::new("2"));
        assert_eq!(items[3].drag_id(), DragId::new("3"));
        assert_eq!(
            items[3].group_id(),
            Some(&"ss1".to_string()),
            "ex3 should join ss1"
        );
        assert_eq!(items[4].drag_id(), DragId::new("4"));
        assert_eq!(items[4].group_id(), None, "ex4 should remain standalone");
    }
}

#[cfg(test)]
mod partition_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum TestItem {
        Exercise { id: String, group: Option<String> },
        Header { id: String },
    }

    impl GroupedItem for TestItem {
        type GroupId = String;

        fn drag_id(&self) -> DragId {
            match self {
                TestItem::Exercise { id, .. } => DragId::new(id),
                TestItem::Header { id } => DragId::new(format!("header-{}", id)),
            }
        }

        fn group_id(&self) -> Option<&String> {
            match self {
                TestItem::Exercise { group, .. } => group.as_ref(),
                TestItem::Header { id } => Some(id),
            }
        }

        fn is_group_header(&self) -> bool {
            matches!(self, TestItem::Header { .. })
        }

        fn set_group_id(&mut self, group_id: Option<String>) {
            if let TestItem::Exercise { group, .. } = self {
                *group = group_id;
            }
        }

        fn make_group_header(group_id: String) -> Self {
            TestItem::Header { id: group_id }
        }
    }

    fn ex(id: &str) -> TestItem {
        TestItem::Exercise {
            id: id.into(),
            group: None,
        }
    }

    fn ex_g(id: &str, group: &str) -> TestItem {
        TestItem::Exercise {
            id: id.into(),
            group: Some(group.into()),
        }
    }

    fn hdr(id: &str) -> TestItem {
        TestItem::Header { id: id.into() }
    }

    // ---- partition_grouped_items ----

    #[test]
    fn test_partition_empty() {
        let items: Vec<TestItem> = vec![];
        let entries = partition_grouped_items(&items);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_partition_standalone_only() {
        let items = vec![ex("1"), ex("2"), ex("3")];
        let entries = partition_grouped_items(&items);
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| e.as_standalone().is_some()));
    }

    #[test]
    fn test_partition_single_group() {
        let items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1")];
        let entries = partition_grouped_items(&items);
        assert_eq!(entries.len(), 1);
        let (gid, group_items) = entries[0].as_group().unwrap();
        assert_eq!(gid, "g1");
        assert_eq!(group_items.len(), 3); // header + 2 members
    }

    #[test]
    fn test_partition_mixed() {
        let items = vec![
            ex("1"),
            hdr("g1"),
            ex_g("2", "g1"),
            ex_g("3", "g1"),
            ex("4"),
        ];
        let entries = partition_grouped_items(&items);
        assert_eq!(entries.len(), 3);
        assert!(entries[0].as_standalone().is_some());
        assert!(entries[1].is_group());
        assert!(entries[2].as_standalone().is_some());
    }

    #[test]
    fn test_partition_drag_ids() {
        let items = vec![ex("1"), hdr("g1"), ex_g("2", "g1")];
        let entries = partition_grouped_items(&items);
        assert_eq!(entries[0].drag_id(), DragId::new("1"));
        assert_eq!(entries[1].drag_id(), DragId::new("g1"));
    }

    #[test]
    fn test_partition_group_item_ids() {
        let items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1")];
        let entries = partition_grouped_items(&items);
        let ids = entries[0].group_item_ids();
        assert_eq!(
            ids,
            vec![DragId::new("header-g1"), DragId::new("1"), DragId::new("2")]
        );
    }

    // ---- find_flat_insert_position ----

    #[test]
    fn test_flat_insert_at_index_0() {
        // container_items: ["1", "2", "3"], index 0 → flat position of "1" = 0
        let items = vec![ex("1"), ex("2"), ex("3")];
        let container_items = vec![DragId::new("1"), DragId::new("2"), DragId::new("3")];
        let pos = find_flat_insert_position(&items, &container_items, 0);
        assert_eq!(pos, 0);
    }

    #[test]
    fn test_flat_insert_at_index_1() {
        // container_items: ["1", "2", "3"], index 1 → flat position of "2" = 1
        let items = vec![ex("1"), ex("2"), ex("3")];
        let container_items = vec![DragId::new("1"), DragId::new("2"), DragId::new("3")];
        let pos = find_flat_insert_position(&items, &container_items, 1);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_flat_insert_group_at_index() {
        // flat: [ex1, hdr-g1, ex2(g1), ex3(g1)]
        // container_items: ["1", "g1"], index 1 → flat position of g1's header = 1
        let items = vec![ex("1"), hdr("g1"), ex_g("2", "g1"), ex_g("3", "g1")];
        let container_items = vec![DragId::new("1"), DragId::new("g1")];
        let pos = find_flat_insert_position(&items, &container_items, 1);
        assert_eq!(pos, 1); // position of the group's header
    }

    #[test]
    fn test_flat_insert_past_end() {
        // index >= container_items.len() → append
        let items = vec![ex("1"), ex("2"), ex("3")];
        let container_items = vec![DragId::new("1"), DragId::new("2"), DragId::new("3")];
        let pos = find_flat_insert_position(&items, &container_items, 3);
        assert_eq!(pos, items.len());
    }

    #[test]
    fn test_flat_insert_after_group() {
        // flat: [ex1, hdr-g1, ex2(g1), ex3(g1), ex4]
        // container_items: ["1", "g1", "4"], index 2 → flat position of "4" = 4
        let items = vec![
            ex("1"),
            hdr("g1"),
            ex_g("2", "g1"),
            ex_g("3", "g1"),
            ex("4"),
        ];
        let container_items = vec![DragId::new("1"), DragId::new("g1"), DragId::new("4")];
        let pos = find_flat_insert_position(&items, &container_items, 2);
        assert_eq!(pos, 4); // position of "4" in flat list
    }

    // ---- group_id_from_container ----

    #[test]
    fn test_group_id_from_container_valid() {
        let id = DragId::new("superset-1-container");
        assert_eq!(group_id_from_container(&id), Some("superset-1".to_string()));
    }

    #[test]
    fn test_group_id_from_container_no_suffix() {
        let id = DragId::new("workout");
        assert_eq!(group_id_from_container(&id), None);
    }

    // ---- grouped_move (via direct list manipulation) ----

    #[test]
    fn test_move_regular_item_into_group() {
        // Move standalone item into a group container
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3]
        // g1-container items: [header-g1, 1, 2] — moving ex3 to index 3 (append)
        let mut items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1"), ex("3")];

        let event = MoveEvent {
            item_id: DragId::new("3"),
            from_container: DragId::new("workout"),
            from_index: 0,
            to_container: DragId::new("g1-container"),
            to_index: 3, // after all current group members (header + 2 members)
        };

        // Simulate grouped_move logic directly
        let from = items
            .iter()
            .position(|i| i.drag_id() == event.item_id)
            .unwrap();
        let mut item = items.remove(from);
        let to_group = group_id_from_container(&event.to_container);
        item.set_group_id(to_group);
        // Derive container items for the group (after removing dragged item)
        let container_items: Vec<DragId> = items
            .iter()
            .filter(|i| i.group_id().map(|g| g.as_str()) == Some("g1"))
            .map(|i| i.drag_id())
            .collect();
        let to = find_flat_insert_position(&items, &container_items, event.to_index);
        items.insert(to.min(items.len()), item);

        assert_eq!(items[3].drag_id(), DragId::new("3"));
        assert_eq!(items[3].group_id(), Some(&"g1".to_string()));
    }

    #[test]
    fn test_move_item_out_of_group() {
        // Move grouped item to parent container (no -container suffix)
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3]
        // parent container items: ["g1", "3"]
        // Moving ex2 to parent at index 2 (past end = append)
        let mut items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1"), ex("3")];

        let event = MoveEvent {
            item_id: DragId::new("2"),
            from_container: DragId::new("g1-container"),
            from_index: 0,
            to_container: DragId::new("workout"),
            to_index: 2, // after "3" in parent's items ["g1", "3"]
        };

        let from = items
            .iter()
            .position(|i| i.drag_id() == event.item_id)
            .unwrap();
        let mut item = items.remove(from);
        let to_group = group_id_from_container(&event.to_container);
        item.set_group_id(to_group);
        // Derive top-level container items (after removing dragged item)
        let container_items: Vec<DragId> = partition_grouped_items(&items)
            .iter()
            .map(|e| e.drag_id())
            .collect();
        let to = find_flat_insert_position(&items, &container_items, event.to_index);
        items.insert(to.min(items.len()), item);

        // Item should now be standalone after ex3
        assert_eq!(items[3].drag_id(), DragId::new("2"));
        assert_eq!(items[3].group_id(), None);
    }

    #[test]
    fn test_move_group_header_block() {
        // Move entire group to a new position
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3, ex4]
        // parent container items: ["g1", "3", "4"]
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            ex("3"),
            ex("4"),
        ];

        // Simulate header block move
        let from = items
            .iter()
            .position(|i| i.drag_id() == DragId::new("header-g1"))
            .unwrap();
        let group_id = items[from].group_id().cloned().unwrap();

        let mut group_items = Vec::new();
        let mut indices_to_remove = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if item.group_id() == Some(&group_id) {
                group_items.push(item.clone());
                indices_to_remove.push(i);
            }
        }
        for idx in indices_to_remove.into_iter().rev() {
            items.remove(idx);
        }

        // After removing g1 group, remaining: [ex3, ex4]
        // container items: ["3", "4"], index 2 = past end = append
        let container_items: Vec<DragId> = partition_grouped_items(&items)
            .iter()
            .map(|e| e.drag_id())
            .collect();
        let to = find_flat_insert_position(&items, &container_items, 2);
        for (offset, item) in group_items.into_iter().enumerate() {
            items.insert(to + offset, item);
        }

        // Group should be after ex3 and ex4
        assert_eq!(items[0].drag_id(), DragId::new("3"));
        assert_eq!(items[1].drag_id(), DragId::new("4"));
        assert_eq!(items[2].drag_id(), DragId::new("header-g1"));
        assert_eq!(items[3].drag_id(), DragId::new("1"));
        assert_eq!(items[4].drag_id(), DragId::new("2"));
    }
}

#[cfg(test)]
mod reorder_group_boundary_tests {
    use super::*;
    use crate::types::ReorderEvent;

    #[derive(Clone, Debug, PartialEq)]
    enum TestItem {
        Exercise { id: String, group: Option<String> },
        Header { id: String },
    }

    impl GroupedItem for TestItem {
        type GroupId = String;

        fn drag_id(&self) -> DragId {
            match self {
                TestItem::Exercise { id, .. } => DragId::new(id),
                TestItem::Header { id } => DragId::new(format!("header-{}", id)),
            }
        }

        fn group_id(&self) -> Option<&String> {
            match self {
                TestItem::Exercise { group, .. } => group.as_ref(),
                TestItem::Header { id } => Some(id),
            }
        }

        fn is_group_header(&self) -> bool {
            matches!(self, TestItem::Header { .. })
        }

        fn set_group_id(&mut self, group_id: Option<String>) {
            if let TestItem::Exercise { group, .. } = self {
                *group = group_id;
            }
        }

        fn make_group_header(group_id: String) -> Self {
            TestItem::Header { id: group_id }
        }
    }

    fn ex(id: &str) -> TestItem {
        TestItem::Exercise {
            id: id.into(),
            group: None,
        }
    }

    fn ex_g(id: &str, group: &str) -> TestItem {
        TestItem::Exercise {
            id: id.into(),
            group: Some(group.into()),
        }
    }

    fn hdr(id: &str) -> TestItem {
        TestItem::Header { id: id.into() }
    }

    /// Reorder standalone item to BEFORE a group.
    /// Simulates AtIndex at position 0 in parent container [g1, 3],
    /// which resolves to before the group's header.
    #[test]
    fn test_reorder_standalone_before_group() {
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3]
        // parent container items: [g1, 3]
        // Dragging ex3. After excluding: [g1] → to_index=0 → before g1
        let mut items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1"), ex("3")];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 1, // ex3 was at container index 1
            to_index: 0,   // insert before g1
            item_id: DragId::new("3"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex3 should be inserted BEFORE the group header, remaining standalone
        assert_eq!(items[0].drag_id(), DragId::new("3"));
        assert_eq!(items[0].group_id(), None, "should remain standalone");
        assert_eq!(items[1].drag_id(), DragId::new("header-g1"));
    }

    /// Reorder standalone item to AFTER a group.
    /// Simulates AtIndex at position 1 in parent container [g1, 3]
    /// (after removing ex4), which resolves to after the group.
    #[test]
    fn test_reorder_standalone_after_group() {
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3, ex4]
        // parent container items: [g1, 3, 4]
        // Dragging ex4. After excluding: [g1, 3] → to_index=1 → before "3" → after group
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            ex("3"),
            ex("4"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 2, // ex4 was at container index 2
            to_index: 1,   // insert before "3" in filtered list [g1, 3]
            item_id: DragId::new("4"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex4 should be right after last group member, remaining standalone
        assert_eq!(items[3].drag_id(), DragId::new("4"));
        assert_eq!(items[3].group_id(), None, "should remain standalone");
        // Group should be intact
        assert_eq!(items[0].drag_id(), DragId::new("header-g1"));
        assert_eq!(items[1].drag_id(), DragId::new("1"));
        assert_eq!(items[2].drag_id(), DragId::new("2"));
    }

    /// Reorder standalone to after group when group is NOT last.
    #[test]
    fn test_reorder_after_group_not_at_end() {
        // flat: [ex0, hdr-g1, ex1(g1), ex2(g1), ex3]
        // parent container items: [0, g1, 3]
        // Dragging ex0. After excluding: [g1, 3] → to_index=1 → before "3" → after group
        let mut items = vec![
            ex("0"),
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            ex("3"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 0, // ex0 was at container index 0
            to_index: 1,   // insert before "3" in filtered list [g1, 3]
            item_id: DragId::new("0"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex0 should be between the group and ex3
        assert_eq!(items[0].drag_id(), DragId::new("header-g1"));
        assert_eq!(items[1].drag_id(), DragId::new("1"));
        assert_eq!(items[2].drag_id(), DragId::new("2"));
        assert_eq!(items[3].drag_id(), DragId::new("0"));
        assert_eq!(items[3].group_id(), None, "should remain standalone");
        assert_eq!(items[4].drag_id(), DragId::new("3"));
    }

    /// Reorder a group header past another group.
    /// After removing g1's block, container items become [g2],
    /// to_index=1 (past end) → append after g2.
    #[test]
    fn test_reorder_group_header_after_another_group() {
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            hdr("g2"),
            ex_g("3", "g2"),
            ex_g("4", "g2"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 0,
            to_index: 1, // past end of filtered [g2] → append
            item_id: DragId::new("header-g1"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // g2 should come first, then g1
        assert_eq!(items[0].drag_id(), DragId::new("header-g2"));
        assert_eq!(items[1].drag_id(), DragId::new("3"));
        assert_eq!(items[2].drag_id(), DragId::new("4"));
        assert_eq!(items[3].drag_id(), DragId::new("header-g1"));
        assert_eq!(items[4].drag_id(), DragId::new("1"));
        assert_eq!(items[5].drag_id(), DragId::new("2"));
    }

    /// Reorder within a group's inner container still assigns group membership.
    /// The container_id ends with CONTAINER_SUFFIX to indicate a within-group reorder.
    #[test]
    fn test_reorder_within_group_still_assigns_membership() {
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3]
        // g1-container items: [header-g1, 1, 2, 3] (ex3 happens to be in this container)
        // Dragging ex3. After excluding: [header-g1, 1, 2]
        // to_index=2 → before "2" → flat position of "2" = 2 → insert ex3 between ex1 and ex2
        let mut items = vec![hdr("g1"), ex_g("1", "g1"), ex_g("2", "g1"), ex("3")];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("g1-container"),
            from_index: 3, // ex3 was at container index 3
            to_index: 2,   // insert before "2" in filtered [header-g1, 1, 2]
            item_id: DragId::new("3"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex3 should be after ex1, inside the group
        assert_eq!(items[2].drag_id(), DragId::new("3"));
        assert_eq!(
            items[2].group_id(),
            Some(&"g1".to_string()),
            "should join group"
        );
    }

    /// Parent-level reorder adjacent to a group should NOT auto-assign
    /// group membership. AtIndex at group boundary stays standalone.
    #[test]
    fn test_reorder_parent_level_near_group_stays_standalone() {
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3, ex4]
        // parent container items: [g1, 3, 4]
        // Dragging ex4. After excluding: [g1, 3] → to_index=1 → before "3" → after group
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            ex("3"),
            ex("4"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 2, // ex4 was at container index 2
            to_index: 1,   // insert before "3" in filtered [g1, 3]
            item_id: DragId::new("4"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex4 should be before ex3, right after the group, and remain standalone
        assert_eq!(items[3].drag_id(), DragId::new("4"));
        assert_eq!(
            items[3].group_id(),
            None,
            "should NOT be absorbed into group"
        );
        assert_eq!(items[4].drag_id(), DragId::new("3"));
    }

    /// Parent-level reorder where item lands right after group's last member
    /// should not auto-assign group membership.
    #[test]
    fn test_reorder_after_group_last_member_stays_standalone() {
        // Same scenario as above: ex4 lands at position after the group
        // flat: [hdr-g1, ex1(g1), ex2(g1), ex3, ex4]
        // parent container items: [g1, 3, 4]
        // Dragging ex4. After excluding: [g1, 3] → to_index=1 → after group
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            ex("3"),
            ex("4"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 2, // ex4 at container index 2
            to_index: 1,   // insert before "3" in filtered [g1, 3]
            item_id: DragId::new("4"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        // ex4 should be right after the group, before ex3, and remain standalone
        assert_eq!(items[3].drag_id(), DragId::new("4"));
        assert_eq!(
            items[3].group_id(),
            None,
            "should NOT be absorbed into group"
        );
    }

    /// Grouped item being reordered to before a group at parent level
    /// should leave the group (clear group_id).
    #[test]
    fn test_reorder_grouped_item_to_boundary_clears_group() {
        // flat: [hdr-g1, ex1(g1), ex2(g1), hdr-g2, ex3(g2)]
        // parent container items: [g1, g2]
        // Dragging ex3. After excluding: [g1, g2] → to_index=0 → before g1
        let mut items = vec![
            hdr("g1"),
            ex_g("1", "g1"),
            ex_g("2", "g1"),
            hdr("g2"),
            ex_g("3", "g2"),
        ];
        let mut grouped = GroupedList::new(&mut items);
        let event = ReorderEvent {
            container_id: DragId::new("workout"),
            from_index: 1, // ex3 at container index 1 in [g1, g2]
            to_index: 0,   // insert before g1
            item_id: DragId::new("3"),
        };
        let changed = grouped.reorder(&event);
        assert!(changed);
        assert_eq!(items[0].drag_id(), DragId::new("3"));
        assert_eq!(
            items[0].group_id(),
            None,
            "should leave group and become standalone"
        );
    }
}
