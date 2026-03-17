use crate::types::{Direction, ListItem};

/// Navigate to the next/previous non-disabled item among the filtered set.
///
/// `items` is the full registration list (needed for disabled/label checks).
/// `filtered` is the ordered list of visible item values.
/// `current` is the currently highlighted value (`None` means nothing highlighted).
/// `loop_navigation` controls wrapping at list boundaries.
pub fn navigate<T: ListItem>(
    items: &[T],
    filtered: &[String],
    current: Option<&str>,
    direction: Direction,
    loop_navigation: bool,
) -> Option<String> {
    if filtered.is_empty() {
        return None;
    }

    let cur_idx = current.and_then(|val| filtered.iter().position(|v| v == val));
    let len = filtered.len();

    if !loop_navigation {
        return navigate_no_loop(items, filtered, cur_idx, direction);
    }

    let start = match (cur_idx, direction) {
        (Some(idx), _) => idx,
        (None, Direction::Forward) => len.wrapping_sub(1),
        (None, Direction::Backward) => 0,
    };

    let step: isize = match direction {
        Direction::Forward => 1,
        Direction::Backward => -1,
    };

    for i in 1..=len {
        let idx = ((start as isize + step * i as isize).rem_euclid(len as isize)) as usize;
        let val = &filtered[idx];
        let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
        if !is_disabled {
            return Some(val.clone());
        }
    }

    None
}

fn navigate_no_loop<T: ListItem>(
    items: &[T],
    filtered: &[String],
    cur_idx: Option<usize>,
    direction: Direction,
) -> Option<String> {
    match direction {
        Direction::Forward => {
            let start = cur_idx.map(|i| i + 1).unwrap_or(0);
            for val in filtered.iter().skip(start) {
                let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
                if !is_disabled {
                    return Some(val.clone());
                }
            }
            None
        }
        Direction::Backward => {
            let start = match cur_idx {
                Some(0) | None => return None,
                Some(i) => i,
            };
            for val in filtered[..start].iter().rev() {
                let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
                if !is_disabled {
                    return Some(val.clone());
                }
            }
            None
        }
    }
}

/// Navigate by `steps` items in the given direction.
///
/// Each step calls `navigate` once. Stops at boundary when `loop_navigation`
/// is false. Returns `None` if no movement is possible.
pub fn navigate_by<T: ListItem>(
    items: &[T],
    filtered: &[String],
    current: Option<&str>,
    steps: usize,
    direction: Direction,
    loop_navigation: bool,
) -> Option<String> {
    if filtered.is_empty() || steps == 0 {
        return current.map(|s| s.to_string());
    }

    let mut cur = current.map(|s| s.to_string());
    for _ in 0..steps {
        match navigate(items, filtered, cur.as_deref(), direction, loop_navigation) {
            Some(next) => cur = Some(next),
            None => break,
        }
    }
    cur
}

/// First non-disabled item in the filtered list.
pub fn first<T: ListItem>(items: &[T], filtered: &[String]) -> Option<String> {
    filtered.iter().find_map(|val| {
        let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
        if !is_disabled {
            Some(val.clone())
        } else {
            None
        }
    })
}

/// Last non-disabled item in the filtered list.
pub fn last<T: ListItem>(items: &[T], filtered: &[String]) -> Option<String> {
    filtered.iter().rev().find_map(|val| {
        let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
        if !is_disabled {
            Some(val.clone())
        } else {
            None
        }
    })
}

/// Type-ahead: find the first item whose label starts with `prefix` (case-insensitive),
/// searching from the item after `current` and wrapping around.
pub fn type_ahead<T: ListItem>(
    items: &[T],
    filtered: &[String],
    current: Option<&str>,
    prefix: &str,
) -> Option<String> {
    if filtered.is_empty() || prefix.is_empty() {
        return None;
    }

    let prefix_lower = prefix.to_lowercase();
    let len = filtered.len();

    let start = match current {
        Some(val) => filtered
            .iter()
            .position(|v| v == val)
            .map(|i| i + 1)
            .unwrap_or(0),
        None => 0,
    };

    for i in 0..len {
        let idx = (start + i) % len;
        let val = &filtered[idx];
        let is_disabled = items.iter().any(|e| e.value() == val && e.disabled());
        if is_disabled {
            continue;
        }
        if let Some(entry) = items.iter().find(|e| e.value() == val)
            && entry.label().to_lowercase().starts_with(&prefix_lower)
        {
            return Some(val.clone());
        }
    }

    None
}
