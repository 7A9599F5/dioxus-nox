use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use dioxus::prelude::*;

/// LRU cache backed by `VecDeque`.
///
/// Most-recently-used entries sit at the back; the front is evicted when
/// `capacity` is exceeded. O(n) operations are acceptable at ≤ 20 entries.
pub(crate) struct PreviewCache {
    entries: VecDeque<(String, Rc<dyn Fn() -> Element>)>,
    capacity: usize,
}

impl PreviewCache {
    pub(crate) fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            entries: VecDeque::with_capacity(cap),
            capacity: cap,
        }
    }

    /// Insert or promote `id`. If the key already exists it is removed first
    /// (so the new entry always lands at the back / most-recently-used slot).
    /// Evicts the front entry when the cache exceeds `capacity`.
    pub(crate) fn insert(&mut self, id: impl Into<String>, render: Rc<dyn Fn() -> Element>) {
        let id = id.into();
        self.entries.retain(|(k, _)| k != &id);
        self.entries.push_back((id, render));
        if self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// Returns a clone of the cached render closure for `id`, or `None`.
    pub(crate) fn get(&self, id: &str) -> Option<Rc<dyn Fn() -> Element>> {
        self.entries
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| Rc::clone(v))
    }

    /// Remove the entry for `id` if present.
    pub(crate) fn invalidate(&mut self, id: &str) {
        self.entries.retain(|(k, _)| k != id);
    }

    /// Remove all entries.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── Public handle ────────────────────────────────────────────────────────────

/// Cheaply-clonable handle to a shared [`PreviewCache`].
///
/// Obtain one via [`use_preview_cache`].
#[derive(Clone)]
pub struct PreviewCacheHandle {
    pub(crate) inner: Rc<RefCell<PreviewCache>>,
}

impl PartialEq for PreviewCacheHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl PreviewCacheHandle {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(PreviewCache::new(capacity))),
        }
    }

    /// Returns the cached render closure for `id`, or `None`.
    pub fn get(&self, id: &str) -> Option<Rc<dyn Fn() -> Element>> {
        self.inner.borrow().get(id)
    }

    /// Insert or promote `id` with the given render closure.
    pub fn insert(&self, id: impl Into<String>, render: Rc<dyn Fn() -> Element>) {
        self.inner.borrow_mut().insert(id, render);
    }

    /// Remove the cached entry for `id`.
    pub fn invalidate(&self, id: &str) {
        self.inner.borrow_mut().invalidate(id);
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        self.inner.borrow_mut().clear();
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}

// ── Hook ─────────────────────────────────────────────────────────────────────

/// Allocates a stable [`PreviewCacheHandle`] for the lifetime of the
/// calling component.
///
/// `capacity` is the maximum number of entries retained; the oldest entry
/// is evicted when the limit is exceeded. Minimum effective capacity is 1.
pub fn use_preview_cache(capacity: usize) -> PreviewCacheHandle {
    use_hook(|| PreviewCacheHandle::new(capacity))
}
