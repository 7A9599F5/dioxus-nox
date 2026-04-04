//! Toast queue manager.

use std::collections::VecDeque;
use std::time::Duration;

use dioxus::prelude::*;

use crate::time;
use crate::types::{Toast, ToastId};

/// Toast manager — manages a queue of toasts with auto-dismiss.
///
/// Provide via `use_context_provider` at app root. The manager is generic
/// over user-defined toast data type `T`.
///
/// All fields are `Copy` (`Signal` is `Copy`, `usize` is `Copy`),
/// so we manually implement `Copy` + `Clone` without requiring `T: Copy`.
pub struct ToastManager<T: Clone + 'static> {
    /// Active toasts signal.
    pub toasts: Signal<VecDeque<Toast<T>>>,
    /// Maximum simultaneous toasts (oldest removed when exceeded).
    pub max_toasts: usize,
}

impl<T: Clone + 'static> Clone for ToastManager<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Clone + 'static> Copy for ToastManager<T> {}

impl<T: Clone + 'static> ToastManager<T> {
    /// Create a new toast manager.
    pub fn new(max_toasts: usize) -> Self {
        Self {
            toasts: Signal::new(VecDeque::new()),
            max_toasts,
        }
    }

    /// Show a toast with auto-dismiss duration.
    pub fn show(&mut self, data: T, duration: Duration) {
        let now = time::now_ms();
        let toast = Toast {
            id: ToastId::new(),
            data,
            expires_at_ms: now + duration.as_millis() as i64,
            undoable: false,
        };
        self.push_toast(toast);
    }

    /// Show a toast that supports undo (consumer handles undo logic).
    pub fn show_undoable(&mut self, data: T, duration: Duration) {
        let now = time::now_ms();
        let toast = Toast {
            id: ToastId::new(),
            data,
            expires_at_ms: now + duration.as_millis() as i64,
            undoable: true,
        };
        self.push_toast(toast);
    }

    /// Dismiss a specific toast by ID.
    pub fn dismiss(&mut self, id: ToastId) {
        let mut toasts = self.toasts.write();
        toasts.retain(|t| t.id != id);
    }

    /// Get a toast by ID.
    pub fn get(&self, id: ToastId) -> Option<Toast<T>> {
        self.toasts.read().iter().find(|t| t.id == id).cloned()
    }

    /// Remove expired toasts (called by ToastViewport tick loop).
    pub fn remove_expired(&mut self) {
        let now = time::now_ms();
        let mut toasts = self.toasts.write();
        toasts.retain(|t| t.expires_at_ms > now);
    }

    fn push_toast(&mut self, toast: Toast<T>) {
        let mut toasts = self.toasts.write();
        toasts.push_back(toast);
        // Evict oldest if over max.
        while toasts.len() > self.max_toasts {
            toasts.pop_front();
        }
    }
}
