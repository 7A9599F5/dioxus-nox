use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

/// Returns a debounced copy of `active_id` that trails changes by
/// `debounce_ms` milliseconds.
///
/// Internally cancels any in-flight timer task when `active_id` changes,
/// matching the pattern used by `dioxus-nox-cmdk`'s debounced query.
///
/// On **non-wasm targets** the value is set immediately (no browser timer is
/// available; local operations are instant anyway).
///
/// # Dioxus 0.7 gotcha
/// The signal subscription (`active_id.read()`) happens *before* any early
/// return so the effect re-runs on every change.
pub fn use_debounced_active(
    active_id: ReadSignal<Option<String>>,
    debounce_ms: u32,
) -> ReadSignal<Option<String>> {
    let mut debounced = use_signal(|| active_id.peek().clone());
    let task_ref: Rc<RefCell<Option<dioxus_core::Task>>> = use_hook(|| Rc::new(RefCell::new(None)));

    use_effect(move || {
        // Subscribe before any early-return so the effect re-runs on change.
        let current = active_id.read().clone();

        // Cancel any in-flight debounce task.
        if let Some(old_task) = task_ref.borrow_mut().take() {
            old_task.cancel();
        }

        if debounce_ms == 0 {
            debounced.set(current);
        } else {
            // Clone for the async block; keep `task_ref` for the outer store.
            let task_ref_inner = task_ref.clone();
            let new_task = spawn(async move {
                // web_sys used here: confirmed no Dioxus 0.7 native API for
                // sub-millisecond timers as of 2026-02-26.
                // Non-WASM targets: fires immediately (instant local ops).
                #[cfg(target_arch = "wasm32")]
                TimeoutFuture::new(debounce_ms).await;

                debounced.set(current);
                *task_ref_inner.borrow_mut() = None;
            });
            *task_ref.borrow_mut() = Some(new_task);
        }
    });

    debounced.into()
}
