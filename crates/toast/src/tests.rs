//! Tests for dioxus-nox-toast.

use crate::types::ToastId;

#[test]
fn toast_id_unique() {
    let id1 = ToastId::new();
    let id2 = ToastId::new();
    assert_ne!(id1, id2);
}

#[test]
fn toast_id_equality() {
    let id = ToastId::new();
    let id_copy = id;
    assert_eq!(id, id_copy);
}

#[test]
fn toast_id_default_is_unique() {
    let id1 = ToastId::default();
    let id2 = ToastId::default();
    assert_ne!(id1, id2);
}

#[test]
fn toast_id_as_u64() {
    let id = ToastId::new();
    let _ = id.as_u64(); // Should not panic.
}

#[test]
fn toast_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let id1 = ToastId::new();
    let id2 = ToastId::new();
    set.insert(id1);
    set.insert(id2);
    assert_eq!(set.len(), 2);
    set.insert(id1); // Duplicate
    assert_eq!(set.len(), 2);
}
