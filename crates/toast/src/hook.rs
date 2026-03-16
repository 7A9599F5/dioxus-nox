//! Toast manager hook.

use dioxus::prelude::*;

use crate::manager::ToastManager;

/// Create and provide a toast manager via Dioxus context.
///
/// Call this at the app root to make the manager available via `use_context()`.
///
/// # Parameters
///
/// - `max_toasts`: Maximum toasts displayed at once (default 3).
pub fn use_toast_manager<T: Clone + 'static>(max_toasts: usize) -> ToastManager<T> {
    use_context_provider(|| ToastManager::<T>::new(max_toasts))
}
