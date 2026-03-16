//! Modal state hook.

use dioxus::prelude::*;

use crate::types::ModalHandle;

/// Create a modal state handle.
///
/// Returns a [`ModalHandle`] with reactive `open` signal and `show`/`close`/`toggle` callbacks.
///
/// # Parameters
///
/// - `initial_open`: Whether the modal starts in the open state.
pub fn use_modal(initial_open: bool) -> ModalHandle {
    let mut open = use_signal(|| initial_open);

    ModalHandle {
        open,
        show: Callback::new(move |()| {
            open.set(true);
        }),
        close: Callback::new(move |()| {
            open.set(false);
        }),
        toggle: Callback::new(move |()| {
            let current = *open.read();
            open.set(!current);
        }),
    }
}
