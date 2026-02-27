use std::collections::HashSet;
use std::rc::Rc;

use crate::types::{GroupRegistration, ItemRegistration};

/// Find the next non-disabled item index.
/// When `loop_navigation` is `true`, wraps from last to first.
/// When `false`, stops at the end of the list and returns `None`.
/// Returns `None` if all items are disabled or the list is empty.
pub fn find_next(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    let len = visible.len();

    if loop_navigation {
        let mut next_idx = (current_idx + 1) % len;
        let start = next_idx;
        loop {
            let nid = &visible[next_idx];
            let is_disabled = items
                .iter()
                .find(|it| &it.id == nid)
                .is_some_and(|it| it.disabled);
            if !is_disabled {
                return Some(next_idx);
            }
            next_idx = (next_idx + 1) % len;
            if next_idx == start {
                return None; // All disabled
            }
        }
    } else {
        // No wrapping: iterate from current_idx + 1 to end
        for (idx, nid) in visible.iter().enumerate().skip(current_idx + 1) {
            let is_disabled = items
                .iter()
                .find(|it| it.id == *nid)
                .is_some_and(|it| it.disabled);
            if !is_disabled {
                return Some(idx);
            }
        }
        None
    }
}

/// Find the previous non-disabled item index.
/// When `loop_navigation` is `true`, wraps from first to last.
/// When `false`, stops at the start of the list and returns `None`.
/// Returns `None` if all items are disabled or the list is empty.
pub fn find_prev(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    let len = visible.len();

    if loop_navigation {
        let mut prev_idx = if current_idx == 0 { len - 1 } else { current_idx - 1 };
        let start = prev_idx;
        loop {
            let pid = &visible[prev_idx];
            let is_disabled = items
                .iter()
                .find(|it| &it.id == pid)
                .is_some_and(|it| it.disabled);
            if !is_disabled {
                return Some(prev_idx);
            }
            prev_idx = if prev_idx == 0 { len - 1 } else { prev_idx - 1 };
            if prev_idx == start {
                return None;
            }
        }
    } else {
        // No wrapping: iterate from current_idx - 1 down to 0
        if current_idx == 0 {
            return None;
        }
        for (idx, pid) in visible.iter().enumerate().take(current_idx).rev() {
            let is_disabled = items
                .iter()
                .find(|it| it.id == *pid)
                .is_some_and(|it| it.disabled);
            if !is_disabled {
                return Some(idx);
            }
        }
        None
    }
}

/// Advance by `steps` non-disabled items in the forward direction.
/// Each step calls `find_next` once. Stops at the list boundary when
/// `loop_navigation` is `false`, or wraps when `true`.
/// Returns the final index reached (may be the same as `current_idx`
/// if no forward movement is possible).
pub(crate) fn find_next_by(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    steps: usize,
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() || steps == 0 {
        return Some(current_idx);
    }
    let mut idx = current_idx;
    for _ in 0..steps {
        match find_next(visible, idx, items, loop_navigation) {
            Some(next) => idx = next,
            None => break, // hit boundary, stop here
        }
    }
    Some(idx)
}

// ---------------------------------------------------------------------------
// P-021: Group-level navigation
// ---------------------------------------------------------------------------

/// Jump to the first item of the next visible group after the active item's group.
///
/// `items` is the full registration list (in registration order).
/// `groups` is in registration order — this defines the group traversal order.
/// Returns the id of the first visible item in the next group with visible items.
///
/// With `loop_nav = true`, wraps from the last group back to the first.
/// With `loop_nav = false`, returns `None` if already in the last group.
///
/// Items with no `group_id` are ignored for group navigation purposes.
///
/// # Key conflict note
/// Group navigation is bound to `Alt+Shift+Arrow` in `CommandInput`.
/// Plain `Alt+Arrow` is reserved for history navigation (Phase 8 / Wave 2).
pub(crate) fn find_next_group(
    items: &[Rc<ItemRegistration>],
    groups: &[GroupRegistration],
    active_id: Option<&str>,
    visible_set: &HashSet<String>,
    loop_nav: bool,
) -> Option<String> {
    if groups.is_empty() || items.is_empty() {
        return None;
    }

    // Determine the group of the active item (None if ungrouped)
    let active_group = active_id.and_then(|aid| {
        items.iter().find(|i| i.id == aid).and_then(|i| i.group_id.as_deref())
    });

    // Find index of the active group in the groups list
    let active_group_idx = active_group
        .and_then(|gid| groups.iter().position(|g| g.id == gid));

    let start_idx = match active_group_idx {
        Some(idx) => idx + 1, // start searching from the next group
        None => 0,             // no group → search from the first group
    };

    let n = groups.len();

    // Search forward for the next group with visible items
    let search_range: Vec<usize> = if loop_nav {
        (0..n).map(|i| (start_idx + i) % n).collect()
    } else {
        (start_idx..n).collect()
    };

    for gidx in search_range {
        // Skip the current group when looping
        if loop_nav && Some(gidx) == active_group_idx {
            continue;
        }
        let gid = &groups[gidx].id;
        // Find first visible item in this group (in items registration order)
        let first = items.iter().find(|i| {
            i.group_id.as_deref() == Some(gid.as_str()) && visible_set.contains(&i.id)
        });
        if let Some(item) = first {
            return Some(item.id.clone());
        }
    }

    None
}

/// Jump to the last item of the previous visible group before the active item's group.
///
/// `items` is the full registration list (in registration order).
/// `groups` is in registration order — this defines the group traversal order.
/// Returns the id of the last visible item in the previous group with visible items.
///
/// With `loop_nav = true`, wraps from the first group back to the last.
/// With `loop_nav = false`, returns `None` if already in the first group.
///
/// Items with no `group_id` are ignored for group navigation purposes.
///
/// # Key conflict note
/// Group navigation is bound to `Alt+Shift+Arrow` in `CommandInput`.
/// Plain `Alt+Arrow` is reserved for history navigation (Phase 8 / Wave 2).
pub(crate) fn find_prev_group(
    items: &[Rc<ItemRegistration>],
    groups: &[GroupRegistration],
    active_id: Option<&str>,
    visible_set: &HashSet<String>,
    loop_nav: bool,
) -> Option<String> {
    if groups.is_empty() || items.is_empty() {
        return None;
    }

    // Determine the group of the active item
    let active_group = active_id.and_then(|aid| {
        items.iter().find(|i| i.id == aid).and_then(|i| i.group_id.as_deref())
    });

    // No group → can't navigate to a previous group
    let active_group_idx = active_group
        .and_then(|gid| groups.iter().position(|g| g.id == gid))?;

    let n = groups.len();

    // Build search range going backwards from (active_group_idx - 1)
    let search_range: Vec<usize> = if loop_nav {
        (1..=n).map(|i| (active_group_idx + n - i) % n).collect()
    } else {
        (0..active_group_idx).rev().collect()
    };

    for gidx in search_range {
        if gidx == active_group_idx {
            continue;
        }
        let gid = &groups[gidx].id;
        // Find last visible item in this group (in items registration order)
        let last = items.iter().rev().find(|i| {
            i.group_id.as_deref() == Some(gid.as_str()) && visible_set.contains(&i.id)
        });
        if let Some(item) = last {
            return Some(item.id.clone());
        }
    }

    None
}

/// Move back by `steps` non-disabled items in the backward direction.
/// Each step calls `find_prev` once. Stops at the list boundary when
/// `loop_navigation` is `false`, or wraps when `true`.
/// Returns the final index reached (may be the same as `current_idx`
/// if no backward movement is possible).
pub(crate) fn find_prev_by(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    steps: usize,
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() || steps == 0 {
        return Some(current_idx);
    }
    let mut idx = current_idx;
    for _ in 0..steps {
        match find_prev(visible, idx, items, loop_navigation) {
            Some(prev) => idx = prev,
            None => break, // hit boundary, stop here
        }
    }
    Some(idx)
}
