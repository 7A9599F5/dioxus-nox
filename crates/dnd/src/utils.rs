//! Utility functions and helpers
//!
//! This module provides utilities for working with drag-and-drop operations:
//! - CSS style constants (`FUNCTIONAL_STYLES`, `THEME_STYLES`, etc.)
//! - Attribute merging utilities for component composition

use dioxus::prelude::Attribute;
use std::ops::RangeInclusive;

/// Find a contiguous block of items that share the same key.
///
/// This is useful when list items are grouped by an identifier and appear as
/// consecutive blocks (e.g., grouped lists, sections, supersets).
///
/// Returns the block's key and the inclusive range of indices that belong to
/// that block. If the index is out of bounds or the key function returns
/// `None`, this returns `None`.
pub fn find_contiguous_block<T, K: Clone + PartialEq>(
    items: &[T],
    index: usize,
    key: impl Fn(&T) -> Option<K>,
) -> Option<(K, RangeInclusive<usize>)> {
    if index >= items.len() {
        return None;
    }

    let target_key = key(&items[index])?;

    let mut start = index;
    while start > 0 {
        let prev_index = start - 1;
        if key(&items[prev_index]) != Some(target_key.clone()) {
            break;
        }
        start = prev_index;
    }

    let mut end = index;
    while end + 1 < items.len() {
        let next_index = end + 1;
        if key(&items[next_index]) != Some(target_key.clone()) {
            break;
        }
        end = next_index;
    }

    Some((target_key, start..=end))
}

// ============================================================================
// CSS Styles — Three-Tier Architecture
// ============================================================================
//
// dx-dnd CSS is split into three tiers:
//
//   Tier 0: FUNCTIONAL_STYLES          (required)  — mechanics, positioning, transforms
//   Tier 1: FEEDBACK_STYLES            (new)       — essential DnD visual feedback
//   Tier 2: THEME_STYLES               (unchanged) — opinionated card/container defaults
//
// Consumer usage patterns:
//
//   Headless (Tailwind):  FUNCTIONAL + FEEDBACK
//   Full defaults:        FUNCTIONAL + THEME       (theme is a superset of feedback)
//   Truly headless:       FUNCTIONAL only           (rare — no visual feedback)
//
// Grouped variants follow the same pattern:
//   GROUPED_FUNCTIONAL + GROUPED_FEEDBACK  or  GROUPED_FUNCTIONAL + GROUPED_THEME
// ============================================================================

/// Functional CSS required for drag-and-drop mechanics.
///
/// These styles handle layout, positioning, visibility states, pointer behavior,
/// displacement animations, and reduced-motion accessibility. Without these,
/// drag-and-drop will not work correctly.
///
/// # Usage
///
/// ```ignore
/// use dioxus_nox_dnd::prelude::*;
///
/// rsx! {
///     style { {FUNCTIONAL_STYLES} }
///     style { {THEME_STYLES} }  // optional — omit for custom styling
///     // ... your components
/// }
/// ```
pub const FUNCTIONAL_STYLES: &str = include_str!("styles-functional.css");

/// Feedback CSS providing essential visual indicators for drag-and-drop perception.
///
/// This is the **recommended** layer for consumers using custom design systems
/// (e.g., Tailwind). It provides:
/// - Drop indicator visibility (line color, endpoint dots, glow)
/// - Merge target highlighting (outline + background tint)
/// - Edge glow on items adjacent to the drop position
/// - Container flex layout with gap
/// - Accessibility (focus outlines, high-contrast mode)
///
/// Override `--dxdnd-primary` and related variables to match your design tokens.
/// See [`THEME_STYLES`] for full opinionated defaults (superset of feedback).
pub const FEEDBACK_STYLES: &str = include_str!("styles-feedback.css");

/// Theme CSS providing beautiful visual defaults (optional).
///
/// Colors, shadows, cursors, hover effects, borders, decorative indicator
/// circles, edge glow, focus outlines, and high-contrast/dark-mode overrides.
///
/// This is a **superset** of [`FEEDBACK_STYLES`] — loading both is unnecessary.
/// Load THEME for quick prototyping, or FEEDBACK for custom design systems.
///
/// Only [`FUNCTIONAL_STYLES`] is strictly required for drag-and-drop mechanics.
///
/// Requires the `styles` feature (enabled by default). Disable with
/// `default-features = false` to exclude opinionated theme CSS.
#[cfg(feature = "styles")]
pub const THEME_STYLES: &str = include_str!("styles-theme.css");

/// Functional CSS for grouped/nested container drag mechanics.
///
/// Handles visibility during drag-out and indicator suppression at group
/// boundaries. Required when using nested `SortableContext` (groups/supersets).
pub const GROUPED_FUNCTIONAL_STYLES: &str = include_str!("grouped-functional.css");

/// Feedback CSS for grouped/nested containers (essential visual indicators only).
///
/// Provides group boundary indicators (external and internal) and adjacent
/// item glow near group edges. Recommended alongside [`FEEDBACK_STYLES`] for
/// consumers using custom design systems.
///
/// See [`GROUPED_THEME_STYLES`] for full opinionated defaults (superset).
pub const GROUPED_FEEDBACK_STYLES: &str = include_str!("grouped-feedback.css");

/// Theme CSS for grouped/nested container visuals (optional).
///
/// Provides header/member styling, group container appearance, boundary
/// indicators, collapse animations, and merge target highlighting.
///
/// This is a **superset** of [`GROUPED_FEEDBACK_STYLES`].
///
/// # CSS Variable Tokens
///
/// - `--dxdnd-grouped-header-bg`, `--dxdnd-grouped-header-color`
/// - `--dxdnd-grouped-member-bg`, `--dxdnd-grouped-border`
/// - `--dxdnd-grouped-radius`, `--dxdnd-grouped-merge-color`
/// - `--dxdnd-grouped-collapse-duration`, `--dxdnd-grouped-collapse-ease`
///
/// Requires the `styles` feature (enabled by default).
#[cfg(feature = "styles")]
pub const GROUPED_THEME_STYLES: &str = include_str!("grouped-theme.css");

// ============================================================================
// Attribute Merging Utilities (Attribute Forwarding Support)
// ============================================================================

/// Extract the value of a named attribute from a list of attributes
///
/// Searches for an attribute with the given name and returns its text value.
///
/// # Arguments
///
/// * `attrs` - The list of attributes to search
/// * `name` - The attribute name to find (e.g., "class", "style")
///
/// # Returns
///
/// `Some(String)` if the attribute exists and has a text value, `None` otherwise
///
/// # Example
///
/// ```ignore
/// let class_value = extract_attribute(&props.attributes, "class");
/// ```
pub fn extract_attribute(attrs: &[Attribute], name: &str) -> Option<String> {
    attrs.iter().find(|a| a.name == name).and_then(|a| {
        // AttributeValue::Text variant contains the text string
        if let dioxus::dioxus_core::AttributeValue::Text(s) = &a.value {
            Some(s.clone())
        } else {
            None
        }
    })
}

/// Merge library styles with consumer styles
///
/// Library styles (like touch-action: none) are placed first to ensure
/// functionality. Consumer styles are appended and can override individual
/// CSS properties via the cascade.
///
/// # Arguments
///
/// * `library` - The library's required functional styles
/// * `consumer` - Optional consumer-provided styles
///
/// # Returns
///
/// A merged style string, or just library styles if consumer is None/empty
///
/// # Example
///
/// ```ignore
/// let style = merge_styles("touch-action: none;", Some("background: red;"));
/// // Result: "touch-action: none; background: red;"
/// ```
pub fn merge_styles(library: &str, consumer: Option<&str>) -> String {
    match consumer {
        Some(s) if !s.is_empty() => format!("{} {}", library, s),
        _ => library.to_string(),
    }
}

/// Filter out class and style attributes from a list
///
/// Used to prepare attributes for spreading after class/style have been
/// merged separately.
///
/// # Arguments
///
/// * `attrs` - The attributes to filter
///
/// # Returns
///
/// A new Vec containing all attributes except "class" and "style"
///
/// # Example
///
/// ```ignore
/// let other_attrs = filter_class_style(props.attributes);
/// // Spread these after handling class/style separately
/// ```
pub fn filter_class_style(attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs
        .into_iter()
        .filter(|a| a.name != "class" && a.name != "style")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::find_contiguous_block;

    #[derive(Clone, Debug, PartialEq)]
    struct Item(Option<&'static str>);

    #[test]
    fn finds_block_for_matching_keys() {
        let items = vec![
            Item(None),
            Item(Some("a")),
            Item(Some("a")),
            Item(Some("b")),
            Item(Some("b")),
            Item(None),
        ];

        let (key, range) = find_contiguous_block(&items, 2, |item| item.0).unwrap();
        assert_eq!(key, "a");
        assert_eq!(range, 1..=2);

        let (key, range) = find_contiguous_block(&items, 4, |item| item.0).unwrap();
        assert_eq!(key, "b");
        assert_eq!(range, 3..=4);
    }

    #[test]
    fn returns_none_when_key_missing_or_index_oob() {
        let items = vec![Item(None), Item(Some("a"))];
        assert!(find_contiguous_block(&items, 0, |item| item.0).is_none());
        assert!(find_contiguous_block(&items, 10, |item| item.0).is_none());
    }
}
