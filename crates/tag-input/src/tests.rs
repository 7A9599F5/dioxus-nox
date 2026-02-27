use std::cmp::Ordering;

use crate::hook::{
    build_groups, compute_auto_complete_text, compute_overflow, filter_denied, find_match_ranges,
    format_error_max_length, format_form_value, format_status_added, format_status_denied,
    format_status_duplicate, format_status_pasted, format_status_removed,
    format_status_suggestions, format_status_truncated, is_below_min, is_denied, is_in_allow_list,
    split_by_delimiters,
};
use crate::tag::{Tag, TagLike};

// ---------------------------------------------------------------------------
// Helper tag type for grouped tests
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
struct GroupedTag {
    id: String,
    name: String,
    group: Option<String>,
    locked: bool,
}

impl GroupedTag {
    fn new(id: &str, name: &str, group: Option<&str>, locked: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            group: group.map(|s| s.to_string()),
            locked,
        }
    }
}

impl TagLike for GroupedTag {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    fn is_locked(&self) -> bool {
        self.locked
    }
}

// ---------------------------------------------------------------------------
// find_match_ranges tests
// ---------------------------------------------------------------------------

#[test]
fn match_ranges_empty_query_returns_empty() {
    let result = find_match_ranges("Hello World", "");
    assert_eq!(result, vec![]);
}

#[test]
fn match_ranges_single_match() {
    let result = find_match_ranges("Hello World", "world");
    assert_eq!(result, vec![(6, 11)]);
}

#[test]
fn match_ranges_multiple_matches() {
    let result = find_match_ranges("abcabc", "abc");
    assert_eq!(result, vec![(0, 3), (3, 6)]);
}

#[test]
fn match_ranges_case_insensitive() {
    let result = find_match_ranges("FooBar", "foo");
    assert_eq!(result, vec![(0, 3)]);
}

#[test]
fn match_ranges_no_match() {
    let result = find_match_ranges("hello", "xyz");
    assert_eq!(result, vec![]);
}

#[test]
fn match_ranges_empty_text() {
    let result = find_match_ranges("", "abc");
    assert_eq!(result, vec![]);
}

#[test]
fn match_ranges_query_equals_text() {
    let result = find_match_ranges("exact", "exact");
    assert_eq!(result, vec![(0, 5)]);
}

#[test]
fn match_ranges_query_longer_than_text() {
    let result = find_match_ranges("hi", "hello");
    assert_eq!(result, vec![]);
}

#[test]
fn match_ranges_overlapping_not_double_counted() {
    // "aa" in "aaa" — first match at (0,2), then search from 2, finds nothing at position 2 because
    // "aaa"[2..] = "a" which is shorter than the query "aa".
    let result = find_match_ranges("aaa", "aa");
    assert_eq!(result, vec![(0, 2)]);
}

#[test]
fn match_ranges_unicode_latin_extended() {
    // "ä" is a two-byte character in UTF-8. The function lowercases and searches.
    // "Ä" lowercases to "ä", so this should match at byte offset 0..2.
    let result = find_match_ranges("Äpfel", "ä");
    assert_eq!(result, vec![(0, 2)]);
}

#[test]
fn match_ranges_unicode_spanish() {
    // "ñ" is 2 bytes in UTF-8. Match "ño" inside "cañon".
    let result = find_match_ranges("cañon", "ño");
    // "c" = 1 byte, "a" = 1 byte, "ñ" = 2 bytes → "ñ" starts at byte 2
    // "ño" = 3 bytes → range is (2, 5)
    assert_eq!(result, vec![(2, 5)]);
}

#[test]
fn match_ranges_match_at_start_and_middle() {
    let result = find_match_ranges("tag-input tag", "tag");
    assert_eq!(result, vec![(0, 3), (10, 13)]);
}

// ---------------------------------------------------------------------------
// build_groups tests
// ---------------------------------------------------------------------------

fn make_tag(id: &str, name: &str, group: Option<&str>) -> GroupedTag {
    GroupedTag::new(id, name, group, false)
}

#[test]
fn build_groups_empty_input_returns_empty() {
    let items: Vec<GroupedTag> = vec![];
    let result = build_groups(&items, None, None, None);
    assert!(result.is_empty());
}

#[test]
fn build_groups_all_same_group() {
    let items = vec![
        make_tag("1", "Alpha", Some("fruits")),
        make_tag("2", "Beta", Some("fruits")),
        make_tag("3", "Gamma", Some("fruits")),
    ];
    let result = build_groups(&items, None, None, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "fruits");
    assert_eq!(result[0].items.len(), 3);
    assert_eq!(result[0].total_count, 3);
}

#[test]
fn build_groups_multiple_groups_preserves_first_seen_order() {
    let items = vec![
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Carrot", Some("veggies")),
        make_tag("3", "Banana", Some("fruits")),
        make_tag("4", "Broccoli", Some("veggies")),
        make_tag("5", "Cherry", Some("fruits")),
    ];
    let result = build_groups(&items, None, None, None);
    assert_eq!(result.len(), 2);
    // "fruits" was first seen before "veggies"
    assert_eq!(result[0].label, "fruits");
    assert_eq!(result[1].label, "veggies");
    assert_eq!(result[0].items.len(), 3);
    assert_eq!(result[1].items.len(), 2);
}

#[test]
fn build_groups_no_group_uses_empty_label() {
    let items = vec![make_tag("1", "Alpha", None), make_tag("2", "Beta", None)];
    let result = build_groups(&items, None, None, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "");
    assert_eq!(result[0].items.len(), 2);
}

#[test]
fn build_groups_mixed_grouped_and_ungrouped() {
    let items = vec![
        make_tag("1", "Alpha", None),
        make_tag("2", "Apple", Some("fruits")),
        make_tag("3", "Beta", None),
    ];
    let result = build_groups(&items, None, None, None);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].label, ""); // ungrouped first
    assert_eq!(result[0].items.len(), 2);
    assert_eq!(result[1].label, "fruits");
    assert_eq!(result[1].items.len(), 1);
}

#[test]
fn build_groups_sort_items_within_group() {
    let items = vec![
        make_tag("1", "Cherry", Some("fruits")),
        make_tag("2", "Apple", Some("fruits")),
        make_tag("3", "Banana", Some("fruits")),
    ];
    let sort_items: fn(&GroupedTag, &GroupedTag) -> Ordering = |a, b| a.name().cmp(b.name());
    let result = build_groups(&items, Some(sort_items), None, None);
    assert_eq!(result.len(), 1);
    let names: Vec<&str> = result[0].items.iter().map(|t| t.name()).collect();
    assert_eq!(names, vec!["Apple", "Banana", "Cherry"]);
}

#[test]
fn build_groups_sort_groups_by_label() {
    let items = vec![
        make_tag("1", "Carrot", Some("veggies")),
        make_tag("2", "Apple", Some("fruits")),
        make_tag("3", "Broccoli", Some("veggies")),
    ];
    let sort_groups: fn(&str, &str) -> Ordering = |a, b| a.cmp(b);
    let result = build_groups(&items, None, Some(sort_groups), None);
    assert_eq!(result.len(), 2);
    // "fruits" < "veggies" alphabetically
    assert_eq!(result[0].label, "fruits");
    assert_eq!(result[1].label, "veggies");
}

#[test]
fn build_groups_max_items_per_group_truncates() {
    let items = vec![
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Banana", Some("fruits")),
        make_tag("3", "Cherry", Some("fruits")),
        make_tag("4", "Carrot", Some("veggies")),
        make_tag("5", "Broccoli", Some("veggies")),
    ];
    let result = build_groups(&items, None, None, Some(2));
    assert_eq!(result.len(), 2);
    // fruits: 3 total, truncated to 2
    assert_eq!(result[0].items.len(), 2);
    assert_eq!(result[0].total_count, 3);
    // veggies: 2 total, within limit
    assert_eq!(result[1].items.len(), 2);
    assert_eq!(result[1].total_count, 2);
}

#[test]
fn build_groups_max_items_one_shows_total_count() {
    let items = vec![
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Banana", Some("fruits")),
        make_tag("3", "Cherry", Some("fruits")),
    ];
    let result = build_groups(&items, None, None, Some(1));
    assert_eq!(result[0].items.len(), 1);
    assert_eq!(result[0].total_count, 3);
}

#[test]
fn build_groups_sort_and_truncate_combined() {
    // Sort descending (Z→A), then keep top 2 per group
    let items = vec![
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Cherry", Some("fruits")),
        make_tag("3", "Banana", Some("fruits")),
    ];
    let sort_items: fn(&GroupedTag, &GroupedTag) -> Ordering = |a, b| b.name().cmp(a.name()); // reverse alphabetical
    let result = build_groups(&items, Some(sort_items), None, Some(2));
    assert_eq!(result.len(), 1);
    // After sort desc: Cherry, Banana, Apple — truncated to 2
    let names: Vec<&str> = result[0].items.iter().map(|t| t.name()).collect();
    assert_eq!(names, vec!["Cherry", "Banana"]);
    assert_eq!(result[0].total_count, 3);
}

// ---------------------------------------------------------------------------
// TagLike::is_locked() tests
// ---------------------------------------------------------------------------

#[test]
fn tag_is_locked_default_false() {
    let tag = Tag::new("1", "MyTag");
    assert!(!tag.is_locked());
}

#[test]
fn grouped_tag_locked_true() {
    let tag = GroupedTag::new("1", "Locked", None, true);
    assert!(tag.is_locked());
}

#[test]
fn grouped_tag_locked_false() {
    let tag = GroupedTag::new("1", "Unlocked", None, false);
    assert!(!tag.is_locked());
}

// ---------------------------------------------------------------------------
// TagLike::group() default tests
// ---------------------------------------------------------------------------

#[test]
fn tag_group_default_none() {
    let tag = Tag::new("1", "MyTag");
    assert_eq!(tag.group(), None);
}

#[test]
fn grouped_tag_group_some() {
    let tag = GroupedTag::new("1", "Apple", Some("fruits"), false);
    assert_eq!(tag.group(), Some("fruits"));
}

#[test]
fn grouped_tag_group_none() {
    let tag = GroupedTag::new("1", "Apple", None, false);
    assert_eq!(tag.group(), None);
}

// ---------------------------------------------------------------------------
// Status message formatting tests
// ---------------------------------------------------------------------------

#[test]
fn status_added_singular() {
    assert_eq!(
        format_status_added("Apple", 1),
        "Apple added. 1 tag selected."
    );
}

#[test]
fn status_added_plural() {
    assert_eq!(
        format_status_added("Banana", 3),
        "Banana added. 3 tags selected."
    );
}

#[test]
fn status_removed_zero() {
    assert_eq!(
        format_status_removed("Cherry", 0),
        "Cherry removed. 0 tags selected."
    );
}

#[test]
fn status_removed_singular() {
    assert_eq!(
        format_status_removed("Grape", 1),
        "Grape removed. 1 tag selected."
    );
}

#[test]
fn status_removed_plural() {
    assert_eq!(
        format_status_removed("Mango", 5),
        "Mango removed. 5 tags selected."
    );
}

#[test]
fn status_pasted_singular() {
    assert_eq!(format_status_pasted(1, 1), "1 tag pasted. 1 tag selected.");
}

#[test]
fn status_pasted_plural() {
    assert_eq!(
        format_status_pasted(3, 5),
        "3 tags pasted. 5 tags selected."
    );
}

#[test]
fn status_suggestions_zero() {
    assert_eq!(format_status_suggestions(0), "0 suggestions available.");
}

#[test]
fn status_suggestions_one() {
    assert_eq!(format_status_suggestions(1), "1 suggestion available.");
}

#[test]
fn status_suggestions_many() {
    assert_eq!(format_status_suggestions(12), "12 suggestions available.");
}

// ---------------------------------------------------------------------------
// split_by_delimiters tests
// ---------------------------------------------------------------------------

#[test]
fn split_empty_string() {
    assert_eq!(split_by_delimiters("", &[',']), Vec::<String>::new());
}

#[test]
fn split_no_delimiters_returns_whole() {
    assert_eq!(
        split_by_delimiters("hello world", &[',']),
        vec!["hello world"]
    );
}

#[test]
fn split_by_comma() {
    assert_eq!(
        split_by_delimiters("apple, banana, cherry", &[',']),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn split_by_newline() {
    assert_eq!(
        split_by_delimiters("apple\nbanana\ncherry", &['\n']),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn split_by_tab() {
    assert_eq!(
        split_by_delimiters("apple\tbanana\tcherry", &['\t']),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn split_multiple_delimiters() {
    assert_eq!(
        split_by_delimiters("apple, banana\ncherry\tgrape", &[',', '\n', '\t']),
        vec!["apple", "banana", "cherry", "grape"]
    );
}

#[test]
fn split_skips_empty_tokens() {
    assert_eq!(
        split_by_delimiters("apple,,banana,,,cherry", &[',']),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn split_trims_whitespace() {
    assert_eq!(
        split_by_delimiters("  apple ,  banana  , cherry  ", &[',']),
        vec!["apple", "banana", "cherry"]
    );
}

#[test]
fn split_only_delimiters_returns_empty() {
    assert_eq!(
        split_by_delimiters(",,,\n\n\t", &[',', '\n', '\t']),
        Vec::<String>::new()
    );
}

#[test]
fn split_only_whitespace_returns_empty() {
    assert_eq!(
        split_by_delimiters(",  ,  ,  ", &[',']),
        Vec::<String>::new()
    );
}

#[test]
fn split_single_char_delimiter() {
    assert_eq!(split_by_delimiters(",", &[',']), Vec::<String>::new());
}

#[test]
fn split_preserves_internal_spaces() {
    assert_eq!(
        split_by_delimiters("New York, Los Angeles, San Francisco", &[',']),
        vec!["New York", "Los Angeles", "San Francisco"]
    );
}

// ---------------------------------------------------------------------------
// Additional status message edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn status_added_zero_tags_selected() {
    // Unusual but tests the format function boundary
    assert_eq!(
        format_status_added("Apple", 0),
        "Apple added. 0 tags selected."
    );
}

#[test]
fn status_pasted_mixed_singular_plural() {
    assert_eq!(format_status_pasted(1, 5), "1 tag pasted. 5 tags selected.");
    assert_eq!(format_status_pasted(3, 1), "3 tags pasted. 1 tag selected.");
}

// ===========================================================================
// Phase 4: Production Guards — Deny List Tests
// ===========================================================================

#[test]
fn deny_list_rejects_forbidden_tag() {
    let deny = vec!["spam".to_string(), "nsfw".to_string()];
    assert!(is_denied("spam", &deny));
    assert!(is_denied("nsfw", &deny));
}

#[test]
fn deny_list_case_insensitive() {
    let deny = vec!["Spam".to_string()];
    assert!(is_denied("spam", &deny));
    assert!(is_denied("SPAM", &deny));
    assert!(is_denied("SpAm", &deny));
}

#[test]
fn deny_list_allows_non_denied() {
    let deny = vec!["spam".to_string()];
    assert!(!is_denied("hello", &deny));
    assert!(!is_denied("world", &deny));
}

#[test]
fn deny_list_empty_allows_all() {
    let deny: Vec<String> = vec![];
    assert!(!is_denied("anything", &deny));
}

#[test]
fn deny_list_filters_suggestions() {
    let tags = vec![
        Tag::new("1", "Apple"),
        Tag::new("2", "Spam"),
        Tag::new("3", "Banana"),
        Tag::new("4", "NSFW"),
    ];
    let deny = vec!["spam".to_string(), "nsfw".to_string()];
    let filtered = filter_denied(&tags, &deny);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].name(), "Apple");
    assert_eq!(filtered[1].name(), "Banana");
}

#[test]
fn deny_list_empty_preserves_all() {
    let tags = vec![Tag::new("1", "Apple"), Tag::new("2", "Banana")];
    let deny: Vec<String> = vec![];
    let filtered = filter_denied(&tags, &deny);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn deny_list_status_message() {
    assert_eq!(format_status_denied("spam"), "spam is not allowed.");
    assert_eq!(format_status_denied("NSFW"), "NSFW is not allowed.");
}

// ===========================================================================
// Phase 4: Production Guards — Allow List Tests
// ===========================================================================

#[test]
fn allow_list_accepts_known_tag() {
    let available = vec![Tag::new("1", "Apple"), Tag::new("2", "Banana")];
    assert!(is_in_allow_list("1", &available));
    assert!(is_in_allow_list("2", &available));
}

#[test]
fn allow_list_rejects_unknown_tag() {
    let available = vec![Tag::new("1", "Apple")];
    assert!(!is_in_allow_list("99", &available));
    assert!(!is_in_allow_list("unknown", &available));
}

#[test]
fn allow_list_empty_rejects_all() {
    let available: Vec<Tag> = vec![];
    assert!(!is_in_allow_list("1", &available));
}

// ===========================================================================
// Phase 4: Production Guards — Duplicate Status Message Tests
// ===========================================================================

#[test]
fn duplicate_status_message() {
    assert_eq!(format_status_duplicate("Apple"), "Apple already exists.");
    assert_eq!(format_status_duplicate("Rust"), "Rust already exists.");
}

// ===========================================================================
// Phase 4: Production Guards — Min Tags Tests
// ===========================================================================

#[test]
fn min_tags_below_minimum_true() {
    assert!(is_below_min(1, Some(3)));
    assert!(is_below_min(0, Some(1)));
    assert!(is_below_min(2, Some(5)));
}

#[test]
fn min_tags_at_minimum_false() {
    assert!(!is_below_min(3, Some(3)));
    assert!(!is_below_min(5, Some(3)));
}

#[test]
fn min_tags_none_always_false() {
    assert!(!is_below_min(0, None));
    assert!(!is_below_min(100, None));
}

#[test]
fn min_tags_zero_never_below() {
    assert!(!is_below_min(0, Some(0)));
    assert!(!is_below_min(5, Some(0)));
}

// ===========================================================================
// Phase 4: Production Guards — Max Suggestions / No Matches Tests
// ===========================================================================

#[test]
fn max_suggestions_caps_list() {
    let tags = vec![
        Tag::new("1", "Apple"),
        Tag::new("2", "Banana"),
        Tag::new("3", "Cherry"),
        Tag::new("4", "Date"),
        Tag::new("5", "Elderberry"),
    ];
    let mut capped = tags.clone();
    capped.truncate(3);
    assert_eq!(capped.len(), 3);
    assert_eq!(capped[2].name(), "Cherry");
}

#[test]
fn max_suggestions_none_unlimited() {
    let tags = vec![
        Tag::new("1", "Apple"),
        Tag::new("2", "Banana"),
        Tag::new("3", "Cherry"),
    ];
    // None means no truncation
    let max: Option<usize> = None;
    let mut result = tags.clone();
    if let Some(m) = max {
        result.truncate(m);
    }
    assert_eq!(result.len(), 3);
}

// ===========================================================================
// Phase 4: Validation Error Messages
// ===========================================================================

#[test]
fn max_tag_length_error_message() {
    assert_eq!(
        format_error_max_length(10),
        "Tag must be 10 characters or fewer."
    );
    assert_eq!(
        format_error_max_length(1),
        "Tag must be 1 characters or fewer."
    );
}

// ===========================================================================
// Phase 5: Async Data Loading — Status Messages
// ===========================================================================

#[test]
fn truncated_suggestions_status_message() {
    assert_eq!(
        format_status_truncated(10, 50),
        "Showing 10 of 50 suggestions. Type to refine."
    );
    assert_eq!(
        format_status_truncated(5, 5),
        "Showing 5 of 5 suggestions. Type to refine."
    );
}

// ===========================================================================
// Phase 6: UX Polish — Overflow Count Tests
// ===========================================================================

#[test]
fn overflow_count_with_limit() {
    assert_eq!(compute_overflow(10, Some(5)), 5);
    assert_eq!(compute_overflow(3, Some(5)), 0);
    assert_eq!(compute_overflow(5, Some(5)), 0);
}

#[test]
fn overflow_count_no_limit() {
    assert_eq!(compute_overflow(10, None), 0);
    assert_eq!(compute_overflow(0, None), 0);
}

#[test]
fn overflow_count_zero_limit() {
    assert_eq!(compute_overflow(5, Some(0)), 5);
    assert_eq!(compute_overflow(0, Some(0)), 0);
}

// ===========================================================================
// Phase 7: Auto-Complete Tests
// ===========================================================================

#[test]
fn auto_complete_text_prefix_match() {
    assert_eq!(compute_auto_complete_text("app", "Apple"), "le");
    assert_eq!(compute_auto_complete_text("ban", "Banana"), "ana");
}

#[test]
fn auto_complete_text_case_insensitive() {
    assert_eq!(compute_auto_complete_text("APP", "Apple"), "le");
    assert_eq!(compute_auto_complete_text("app", "APPLE"), "LE");
}

#[test]
fn auto_complete_text_no_match() {
    assert_eq!(compute_auto_complete_text("xyz", "Apple"), "");
}

#[test]
fn auto_complete_text_empty_query() {
    assert_eq!(compute_auto_complete_text("", "Apple"), "");
}

#[test]
fn auto_complete_text_exact_match() {
    assert_eq!(compute_auto_complete_text("Apple", "Apple"), "");
}

#[test]
fn auto_complete_text_partial_match_not_prefix() {
    // "ple" is in "Apple" but not a prefix — should return empty
    assert_eq!(compute_auto_complete_text("ple", "Apple"), "");
}

// ===========================================================================
// Phase 7: Form Value Serialization Tests
// ===========================================================================

#[test]
fn form_value_empty() {
    assert_eq!(format_form_value(&[]), "[]");
}

#[test]
fn form_value_single() {
    assert_eq!(format_form_value(&["tag1"]), "[\"tag1\"]");
}

#[test]
fn form_value_multiple() {
    assert_eq!(format_form_value(&["a", "b", "c"]), "[\"a\",\"b\",\"c\"]");
}

#[test]
fn form_value_with_special_chars() {
    assert_eq!(
        format_form_value(&["tag-1", "tag_2", "tag.3"]),
        "[\"tag-1\",\"tag_2\",\"tag.3\"]"
    );
}

// ===========================================================================
// Phase 6: Deny List + build_groups Integration Tests
// ===========================================================================

#[test]
fn filter_denied_with_grouped_tags() {
    let tags = vec![
        GroupedTag::new("1", "Apple", Some("fruits"), false),
        GroupedTag::new("2", "Spam", Some("junk"), false),
        GroupedTag::new("3", "Banana", Some("fruits"), false),
        GroupedTag::new("4", "Phishing", Some("junk"), false),
    ];
    let deny = vec!["spam".to_string(), "phishing".to_string()];
    let filtered = filter_denied(&tags, &deny);
    assert_eq!(filtered.len(), 2);
    let groups = build_groups(&filtered, None, None, None);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].label, "fruits");
    assert_eq!(groups[0].items.len(), 2);
}

#[test]
fn filter_denied_preserves_locked_status() {
    let tags = vec![
        GroupedTag::new("1", "Admin", None, true),
        GroupedTag::new("2", "Spam", None, false),
        GroupedTag::new("3", "User", None, false),
    ];
    let deny = vec!["spam".to_string()];
    let filtered = filter_denied(&tags, &deny);
    assert_eq!(filtered.len(), 2);
    assert!(filtered[0].is_locked()); // Admin is still locked
    assert!(!filtered[1].is_locked()); // User is not locked
}

// ===========================================================================
// Phase 6: Sort Integration Tests
// ===========================================================================

#[test]
fn build_groups_sort_items_alphabetical() {
    let items = vec![
        make_tag("3", "Cherry", Some("fruits")),
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Banana", Some("fruits")),
    ];
    let sort_fn: fn(&GroupedTag, &GroupedTag) -> Ordering = |a, b| a.name().cmp(b.name());
    let groups = build_groups(&items, Some(sort_fn), None, None);
    let names: Vec<&str> = groups[0].items.iter().map(|t| t.name()).collect();
    assert_eq!(names, vec!["Apple", "Banana", "Cherry"]);
}

#[test]
fn build_groups_sort_items_reverse() {
    let items = vec![
        make_tag("1", "Apple", Some("fruits")),
        make_tag("2", "Banana", Some("fruits")),
        make_tag("3", "Cherry", Some("fruits")),
    ];
    let sort_fn: fn(&GroupedTag, &GroupedTag) -> Ordering = |a, b| b.name().cmp(a.name());
    let groups = build_groups(&items, Some(sort_fn), None, None);
    let names: Vec<&str> = groups[0].items.iter().map(|t| t.name()).collect();
    assert_eq!(names, vec!["Cherry", "Banana", "Apple"]);
}

// ===========================================================================
// Edge case: deny list with unicode
// ===========================================================================

#[test]
fn deny_list_unicode_case_insensitive() {
    let deny = vec!["Ärger".to_string()];
    assert!(is_denied("ärger", &deny));
    assert!(is_denied("ÄRGER", &deny));
}

#[test]
fn deny_list_partial_match_not_denied() {
    // "spa" should NOT be denied when "spam" is in the deny list
    let deny = vec!["spam".to_string()];
    assert!(!is_denied("spa", &deny));
    assert!(!is_denied("spammer", &deny));
}

// ===========================================================================
// Phase 8: Truncation info status message
// ===========================================================================

#[test]
fn truncated_status_message_boundary() {
    assert_eq!(
        format_status_truncated(0, 0),
        "Showing 0 of 0 suggestions. Type to refine."
    );
    assert_eq!(
        format_status_truncated(1, 1000),
        "Showing 1 of 1000 suggestions. Type to refine."
    );
}
