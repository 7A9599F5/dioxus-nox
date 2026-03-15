//! Keyboard-driven drag navigation.
//!
//! This module implements keyboard-accessible drag-and-drop, allowing users
//! to navigate items with arrow keys, switch containers with Tab, toggle
//! merge with M, and drop with Space/Enter.

use std::collections::HashMap;

#[allow(unused_imports)]
use dioxus::prelude::*;

use super::{DragContext, DropZoneState, sorted_items_in_container};
#[cfg(target_arch = "wasm32")]
use crate::patterns::sortable::item::NextAnimationFrame;
use crate::types::{AnnouncementEvent, DragData, DragId, DropEvent, DropLocation};

impl DragContext {
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
        if let Some(zone) = zones.get(&target_item_id)
            && !zone.accepts_data(drag_data)
        {
            return None;
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

/// Focus a SortableItem element by its DragId after a keyboard drop.
///
/// Spawns an async task that waits two animation frames (for DOM re-render)
/// then focuses the element with the matching `data-dnd-id` attribute.
#[cfg(target_arch = "wasm32")]
pub(super) fn keyboard_focus_item(item_id: DragId) {
    use wasm_bindgen::JsCast;
    spawn(async move {
        NextAnimationFrame::new().await;
        NextAnimationFrame::new().await;
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Ok(Some(el)) =
                document.query_selector(&format!("[data-dnd-id=\"{}\"]", item_id.0))
            && let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>()
        {
            let _ = html_el.focus();
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
