//! Inline confirm state hook.

use dioxus::prelude::*;

use crate::types::{ConfirmState, InlineConfirmHandle};

/// Create an inline confirm state handle.
///
/// # Parameters
///
/// - `auto_cancel_ms`: Auto-cancel after this many milliseconds (None = no timeout).
///   When set, the state automatically transitions from Confirming → Idle after the timeout.
pub fn use_inline_confirm(auto_cancel_ms: Option<u64>) -> InlineConfirmHandle {
    let mut state = use_signal(|| ConfirmState::Idle);
    let mut cancel_generation = use_signal(|| 0u32);

    // Auto-cancel effect.
    let gen_value = *cancel_generation.read();
    use_effect(move || {
        let current_gen = gen_value;
        if *state.read() == ConfirmState::Confirming
            && let Some(timeout_ms) = auto_cancel_ms
        {
            spawn(async move {
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(timeout_ms as u32).await;
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)).await;

                // Only cancel if generation hasn't changed.
                if *cancel_generation.read() == current_gen
                    && *state.read() == ConfirmState::Confirming
                {
                    state.set(ConfirmState::Idle);
                }
            });
        }
    });

    InlineConfirmHandle {
        state,
        request: Callback::new(move |()| {
            if *state.read() == ConfirmState::Idle {
                state.set(ConfirmState::Confirming);
                let next_gen = cancel_generation.read().wrapping_add(1);
                cancel_generation.set(next_gen);
            }
        }),
        confirm: Callback::new(move |()| {
            state.set(ConfirmState::Idle);
            let next_gen = cancel_generation.read().wrapping_add(1);
            cancel_generation.set(next_gen);
        }),
        cancel: Callback::new(move |()| {
            state.set(ConfirmState::Idle);
            let next_gen = cancel_generation.read().wrapping_add(1);
            cancel_generation.set(next_gen);
        }),
    }
}
