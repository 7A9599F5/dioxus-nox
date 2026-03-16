use crate::types::ItemEntry;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

/// Navigate to the next/previous non-disabled item among the filtered set, wrapping.
///
/// `items` is the full registration list (needed for disabled checks).
/// `filtered` is the ordered list of visible item values.
/// `current` is the currently highlighted value (`None` means nothing highlighted).
pub fn navigate(
    items: &[ItemEntry],
    filtered: &[String],
    current: Option<&str>,
    direction: Direction,
) -> Option<String> {
    if filtered.is_empty() {
        return None;
    }

    let cur_idx = match current {
        Some(val) => filtered.iter().position(|v| v == val),
        None => None,
    };

    let len = filtered.len();
    let start = match (cur_idx, direction) {
        (Some(idx), _) => idx,
        (None, Direction::Forward) => len.wrapping_sub(1), // so first step lands on 0
        (None, Direction::Backward) => 0,                  // so first step lands on last
    };

    let step: isize = match direction {
        Direction::Forward => 1,
        Direction::Backward => -1,
    };

    for i in 1..=len {
        let idx = ((start as isize + step * i as isize).rem_euclid(len as isize)) as usize;
        let val = &filtered[idx];
        let is_disabled = items.iter().any(|e| e.value == *val && e.disabled);
        if !is_disabled {
            return Some(val.clone());
        }
    }

    None
}

/// First non-disabled item in the filtered list.
pub fn first(items: &[ItemEntry], filtered: &[String]) -> Option<String> {
    filtered.iter().find_map(|val| {
        let is_disabled = items.iter().any(|e| e.value == *val && e.disabled);
        if !is_disabled {
            Some(val.clone())
        } else {
            None
        }
    })
}

/// Last non-disabled item in the filtered list.
pub fn last(items: &[ItemEntry], filtered: &[String]) -> Option<String> {
    filtered.iter().rev().find_map(|val| {
        let is_disabled = items.iter().any(|e| e.value == *val && e.disabled);
        if !is_disabled {
            Some(val.clone())
        } else {
            None
        }
    })
}

/// Type-ahead: find the first item whose label starts with `prefix` (case-insensitive),
/// searching from the item after `current` and wrapping around.
pub fn type_ahead(
    items: &[ItemEntry],
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
        let is_disabled = items.iter().any(|e| e.value == *val && e.disabled);
        if is_disabled {
            continue;
        }
        if let Some(entry) = items.iter().find(|e| e.value == *val)
            && entry.label.to_lowercase().starts_with(&prefix_lower)
        {
            return Some(val.clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(specs: &[(&str, &str, bool)]) -> Vec<ItemEntry> {
        specs
            .iter()
            .map(|(v, l, d)| ItemEntry {
                value: v.to_string(),
                label: l.to_string(),
                keywords: String::new(),
                disabled: *d,
                group_id: None,
            })
            .collect()
    }

    fn vals(specs: &[&str]) -> Vec<String> {
        specs.iter().map(|s| s.to_string()).collect()
    }

    // ── navigate ────────────────────────────────────────────────

    #[test]
    fn navigate_forward_wraps() {
        let items = entries(&[("a", "A", false), ("b", "B", false), ("c", "C", false)]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(
            navigate(&items, &filtered, Some("c"), Direction::Forward),
            Some("a".into())
        );
    }

    #[test]
    fn navigate_backward_wraps() {
        let items = entries(&[("a", "A", false), ("b", "B", false), ("c", "C", false)]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(
            navigate(&items, &filtered, Some("a"), Direction::Backward),
            Some("c".into())
        );
    }

    #[test]
    fn navigate_skips_disabled() {
        let items = entries(&[("a", "A", false), ("b", "B", true), ("c", "C", false)]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(
            navigate(&items, &filtered, Some("a"), Direction::Forward),
            Some("c".into())
        );
    }

    #[test]
    fn navigate_all_disabled_returns_none() {
        let items = entries(&[("a", "A", true), ("b", "B", true)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(
            navigate(&items, &filtered, Some("a"), Direction::Forward),
            None
        );
    }

    #[test]
    fn navigate_empty_list_returns_none() {
        let items: Vec<ItemEntry> = vec![];
        let filtered: Vec<String> = vec![];
        assert_eq!(navigate(&items, &filtered, None, Direction::Forward), None);
    }

    #[test]
    fn navigate_no_current_forward_selects_first() {
        let items = entries(&[("a", "A", false), ("b", "B", false)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(
            navigate(&items, &filtered, None, Direction::Forward),
            Some("a".into())
        );
    }

    #[test]
    fn navigate_no_current_backward_selects_last() {
        let items = entries(&[("a", "A", false), ("b", "B", false)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(
            navigate(&items, &filtered, None, Direction::Backward),
            Some("b".into())
        );
    }

    #[test]
    fn navigate_single_element() {
        let items = entries(&[("a", "A", false)]);
        let filtered = vals(&["a"]);
        assert_eq!(
            navigate(&items, &filtered, Some("a"), Direction::Forward),
            Some("a".into())
        );
    }

    // ── first / last ────────────────────────────────────────────

    #[test]
    fn first_returns_first_non_disabled() {
        let items = entries(&[("a", "A", true), ("b", "B", false), ("c", "C", false)]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(first(&items, &filtered), Some("b".into()));
    }

    #[test]
    fn last_returns_last_non_disabled() {
        let items = entries(&[("a", "A", false), ("b", "B", false), ("c", "C", true)]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(last(&items, &filtered), Some("b".into()));
    }

    #[test]
    fn first_empty_returns_none() {
        let items: Vec<ItemEntry> = vec![];
        let filtered: Vec<String> = vec![];
        assert_eq!(first(&items, &filtered), None);
    }

    // ── type_ahead ──────────────────────────────────────────────

    #[test]
    fn type_ahead_finds_match() {
        let items = entries(&[
            ("a", "Apple", false),
            ("b", "Banana", false),
            ("c", "Cherry", false),
        ]);
        let filtered = vals(&["a", "b", "c"]);
        assert_eq!(type_ahead(&items, &filtered, None, "b"), Some("b".into()));
    }

    #[test]
    fn type_ahead_case_insensitive() {
        let items = entries(&[("a", "Apple", false), ("b", "Banana", false)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(type_ahead(&items, &filtered, None, "BAN"), Some("b".into()));
    }

    #[test]
    fn type_ahead_wraps_from_current() {
        let items = entries(&[("a", "Apple", false), ("b", "Avocado", false)]);
        let filtered = vals(&["a", "b"]);
        // Starting after "a", should find "b" (Avocado starts with A)
        assert_eq!(
            type_ahead(&items, &filtered, Some("a"), "a"),
            Some("b".into())
        );
    }

    #[test]
    fn type_ahead_no_match_returns_none() {
        let items = entries(&[("a", "Apple", false), ("b", "Banana", false)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(type_ahead(&items, &filtered, None, "z"), None);
    }

    #[test]
    fn type_ahead_skips_disabled() {
        let items = entries(&[("a", "Apple", true), ("b", "Apricot", false)]);
        let filtered = vals(&["a", "b"]);
        assert_eq!(type_ahead(&items, &filtered, None, "a"), Some("b".into()));
    }

    #[test]
    fn type_ahead_empty_prefix_returns_none() {
        let items = entries(&[("a", "Apple", false)]);
        let filtered = vals(&["a"]);
        assert_eq!(type_ahead(&items, &filtered, None, ""), None);
    }
}
