use std::rc::Rc;

use crate::types::ItemRegistration;

use dioxus_nox_collection::Direction;

/// Find the next non-disabled item index.
/// When `loop_navigation` is `true`, wraps from last to first.
/// When `false`, stops at the end of the list and returns `None`.
pub fn find_next(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    let current = visible.get(current_idx).map(|s| s.as_str());
    let items_ref: Vec<&ItemRegistration> = items.iter().map(|i| i.as_ref()).collect();
    let result = dioxus_nox_collection::navigate(
        &items_ref,
        visible,
        current,
        Direction::Forward,
        loop_navigation,
    );
    result.and_then(|val| visible.iter().position(|v| v == &val))
}

/// Find the previous non-disabled item index.
/// When `loop_navigation` is `true`, wraps from first to last.
/// When `false`, stops at the start of the list and returns `None`.
pub fn find_prev(
    visible: &[String],
    current_idx: usize,
    items: &[Rc<ItemRegistration>],
    loop_navigation: bool,
) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    let current = visible.get(current_idx).map(|s| s.as_str());
    let items_ref: Vec<&ItemRegistration> = items.iter().map(|i| i.as_ref()).collect();
    let result = dioxus_nox_collection::navigate(
        &items_ref,
        visible,
        current,
        Direction::Backward,
        loop_navigation,
    );
    result.and_then(|val| visible.iter().position(|v| v == &val))
}

/// Advance by `steps` non-disabled items in the forward direction.
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
            None => break,
        }
    }
    Some(idx)
}

/// Move back by `steps` non-disabled items in the backward direction.
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
            None => break,
        }
    }
    Some(idx)
}

// ---------------------------------------------------------------------------
// P-021: Group-level navigation
// ---------------------------------------------------------------------------

use std::collections::HashSet;

use crate::types::GroupRegistration;

/// Jump to the first item of the next visible group after the active item's group.
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

    let active_group = active_id.and_then(|aid| {
        items
            .iter()
            .find(|i| i.id == aid)
            .and_then(|i| i.group_id.as_deref())
    });

    let active_group_idx = active_group.and_then(|gid| groups.iter().position(|g| g.id == gid));

    let start_idx = match active_group_idx {
        Some(idx) => idx + 1,
        None => 0,
    };

    let n = groups.len();

    let search_range: Vec<usize> = if loop_nav {
        (0..n).map(|i| (start_idx + i) % n).collect()
    } else {
        (start_idx..n).collect()
    };

    for gidx in search_range {
        if loop_nav && Some(gidx) == active_group_idx {
            continue;
        }
        let gid = &groups[gidx].id;
        let first = items
            .iter()
            .find(|i| i.group_id.as_deref() == Some(gid.as_str()) && visible_set.contains(&i.id));
        if let Some(item) = first {
            return Some(item.id.clone());
        }
    }

    None
}

/// Jump to the last item of the previous visible group before the active item's group.
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

    let active_group = active_id.and_then(|aid| {
        items
            .iter()
            .find(|i| i.id == aid)
            .and_then(|i| i.group_id.as_deref())
    });

    let active_group_idx =
        active_group.and_then(|gid| groups.iter().position(|g| g.id == gid))?;

    let n = groups.len();

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
        let last = items
            .iter()
            .rev()
            .find(|i| i.group_id.as_deref() == Some(gid.as_str()) && visible_set.contains(&i.id));
        if let Some(item) = last {
            return Some(item.id.clone());
        }
    }

    None
}
