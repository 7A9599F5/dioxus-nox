//! Toast viewport component.

use dioxus::prelude::*;

use crate::manager::ToastManager;
use crate::types::Toast;

/// Headless toast viewport — renders toasts with auto-dismiss.
///
/// Consumes the [`ToastManager`] from context and renders each active toast
/// via the `render_toast` callback. Automatically removes expired toasts.
#[component]
pub fn ToastViewport<T: Clone + PartialEq + 'static>(
    /// Render function for each toast.
    render_toast: Callback<Toast<T>, Element>,
) -> Element {
    let mut manager: ToastManager<T> = use_context();

    // Tick loop to remove expired toasts.
    use_effect(move || {
        spawn(async move {
            loop {
                crate::time::sleep_ms(1000).await;
                manager.remove_expired();
            }
        });
    });

    let toasts = manager.toasts.read();

    rsx! {
        div {
            role: "region",
            aria_label: "Notifications",
            aria_live: "polite",
            "data-toast-viewport": "",
            for toast in toasts.iter() {
                div {
                    key: "{toast.id.as_u64()}",
                    role: "status",
                    aria_atomic: "true",
                    "data-toast-state": "active",
                    {render_toast(toast.clone())}
                }
            }
        }
    }
}
