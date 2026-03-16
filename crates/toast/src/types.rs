//! Core types for dioxus-nox-toast.

use std::sync::atomic::{AtomicU64, Ordering};

/// Unique toast identifier.
///
/// By default uses an incrementing `u64` counter. Enable the `uuid` feature
/// for `From<uuid::Uuid>` support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

static TOAST_COUNTER: AtomicU64 = AtomicU64::new(0);

impl ToastId {
    /// Create a new unique toast ID via atomic counter.
    pub fn new() -> Self {
        Self(TOAST_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the inner numeric value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ToastId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for ToastId {
    fn from(uuid: uuid::Uuid) -> Self {
        // Use lower 64 bits of the UUID.
        Self(uuid.as_u128() as u64)
    }
}

/// A toast item in the queue.
#[derive(Clone)]
pub struct Toast<T: Clone + 'static> {
    /// Unique identifier.
    pub id: ToastId,
    /// User-provided data (message, variant, action — whatever the consumer needs).
    pub data: T,
    /// When this toast expires (epoch ms, auto-dismiss).
    pub expires_at_ms: i64,
    /// Whether this toast supports an undo action.
    pub undoable: bool,
}
