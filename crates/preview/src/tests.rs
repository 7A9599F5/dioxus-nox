use std::rc::Rc;

use dioxus::prelude::Element;

use crate::cache::{PreviewCache, PreviewCacheHandle};
use crate::position::PreviewPosition;

// Helper: a closure that satisfies `Fn() -> Element`.
// `panic!()` returns `!` which coerces to any type — the closure is never
// called in cache tests, only stored and retrieved by key.
fn stub() -> Rc<dyn Fn() -> Element> {
    Rc::new(|| panic!("stub render: not intended to be called in tests"))
}

// ── PreviewCache ──────────────────────────────────────────────────────────────

#[test]
fn test_insert_and_get() {
    let mut cache = PreviewCache::new(5);
    cache.insert("a", stub());
    assert!(cache.get("a").is_some());
    assert!(cache.get("b").is_none());
}

#[test]
fn test_capacity_eviction() {
    let mut cache = PreviewCache::new(2);
    cache.insert("a", stub());
    cache.insert("b", stub());
    cache.insert("c", stub()); // "a" should be evicted
    assert_eq!(cache.len(), 2);
    assert!(cache.get("a").is_none(), "oldest entry should be evicted");
    assert!(cache.get("b").is_some());
    assert!(cache.get("c").is_some());
}

#[test]
fn test_reinsertion_promotes() {
    // Re-inserting an existing key removes the old entry and pushes to back,
    // so "a" becomes the newest and "b" is now the oldest candidate for eviction.
    let mut cache = PreviewCache::new(2);
    cache.insert("a", stub());
    cache.insert("b", stub());
    cache.insert("a", stub()); // re-insert "a" — promotes it; no eviction yet
    assert_eq!(cache.len(), 2);

    // Now insert "c": the oldest entry is "b" (a was just promoted).
    cache.insert("c", stub());
    assert_eq!(cache.len(), 2);
    assert!(
        cache.get("b").is_none(),
        "b should be evicted after a was promoted"
    );
    assert!(cache.get("a").is_some());
    assert!(cache.get("c").is_some());
}

#[test]
fn test_invalidate() {
    let mut cache = PreviewCache::new(5);
    cache.insert("a", stub());
    cache.insert("b", stub());
    cache.invalidate("a");
    assert!(cache.get("a").is_none());
    assert!(cache.get("b").is_some());
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_clear() {
    let mut cache = PreviewCache::new(5);
    cache.insert("a", stub());
    cache.insert("b", stub());
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_empty_on_construction() {
    let cache = PreviewCache::new(5);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_len_increases_after_insert() {
    let mut cache = PreviewCache::new(5);
    cache.insert("x", stub());
    assert!(!cache.is_empty());
    assert_eq!(cache.len(), 1);

    cache.insert("y", stub());
    assert_eq!(cache.len(), 2);
}

#[test]
fn test_len_decreases_after_invalidate() {
    let mut cache = PreviewCache::new(5);
    cache.insert("x", stub());
    cache.insert("y", stub());
    cache.invalidate("x");
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_cache_zero_capacity_clamps_to_one() {
    let mut cache = PreviewCache::new(0);
    cache.insert("a", stub());
    cache.insert("b", stub()); // must evict "a"
    assert_eq!(cache.len(), 1, "capacity 0 must clamp to 1");
    assert!(cache.get("a").is_none(), "a should be evicted");
    assert!(cache.get("b").is_some());
}

// ── PreviewCacheHandle ────────────────────────────────────────────────────────

#[test]
fn test_handle_insert_and_get() {
    let handle = PreviewCacheHandle::new(5);
    handle.insert("a", stub());
    assert!(handle.get("a").is_some());
    assert!(handle.get("z").is_none());
}

#[test]
fn test_handle_invalidate() {
    let handle = PreviewCacheHandle::new(5);
    handle.insert("a", stub());
    handle.insert("b", stub());
    handle.invalidate("a");
    assert!(handle.get("a").is_none());
    assert!(handle.get("b").is_some());
}

#[test]
fn test_handle_clear() {
    let handle = PreviewCacheHandle::new(5);
    handle.insert("a", stub());
    handle.insert("b", stub());
    handle.clear();
    assert!(handle.is_empty());
    assert_eq!(handle.len(), 0);
}

// ── PreviewPosition ───────────────────────────────────────────────────────────

#[test]
fn test_position_data_attr_none() {
    assert_eq!(PreviewPosition::None.as_data_attr(), None);
}

#[test]
fn test_position_data_attr_right() {
    assert_eq!(PreviewPosition::Right.as_data_attr(), Some("right"));
}

#[test]
fn test_position_data_attr_bottom() {
    assert_eq!(PreviewPosition::Bottom.as_data_attr(), Some("bottom"));
}

#[test]
fn test_position_default_is_none() {
    assert_eq!(PreviewPosition::default(), PreviewPosition::None);
}
