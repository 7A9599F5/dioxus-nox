use crate::types::*;

// ── ID generation tests ─────────────────────────────────────────────────────

#[test]
fn instance_id_increments() {
    let a = next_instance_id();
    let b = next_instance_id();
    assert!(b > a);
}

#[test]
fn autocomplete_aria_attrs() {
    assert_eq!(AutoComplete::None.as_aria_attr(), "none");
    assert_eq!(AutoComplete::List.as_aria_attr(), "list");
    assert_eq!(AutoComplete::Both.as_aria_attr(), "both");
}

#[test]
fn autocomplete_default_is_none() {
    assert_eq!(AutoComplete::default(), AutoComplete::None);
}

#[test]
fn custom_filter_never_equal() {
    let a = CustomFilter::new(|_, _| Some(1));
    let b = CustomFilter::new(|_, _| Some(1));
    assert_ne!(a, b);
}

#[test]
fn item_entry_equality() {
    let a = ItemEntry {
        value: "x".into(),
        label: "X".into(),
        keywords: String::new(),
        disabled: false,
        group_id: None,
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn group_entry_equality() {
    let a = GroupEntry {
        id: "g1".into(),
        label: Some("Group 1".into()),
    };
    let b = a.clone();
    assert_eq!(a, b);
}

// NOTE: Navigation and filter tests live in their respective modules
// (navigation.rs and filter.rs) alongside the implementation for locality.
