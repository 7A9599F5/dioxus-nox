use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

use crate::math;
use crate::types::{LongPressHandle, LongPressPhase};

/// Maximum pointer drift in px before the long-press is cancelled.
const MAX_DRIFT_PX: f64 = 10.0;

fn do_cancel(
    phase: &mut Signal<LongPressPhase>,
    task_ref: &Rc<RefCell<Option<dioxus_core::Task>>>,
    active_pointer: &Rc<Cell<Option<i32>>>,
) {
    if let Some(old_task) = task_ref.borrow_mut().take() {
        old_task.cancel();
    }
    active_pointer.set(None);
    phase.set(LongPressPhase::Idle);
}

/// Hook that detects long-press gestures.
///
/// Returns a [`LongPressHandle`] whose event handlers should be wired to the
/// target element's pointer events. When the pointer is held for `duration_ms`
/// without drifting more than 10px, `on_press` fires.
///
/// # Example
/// ```rust,ignore
/// let lp = use_long_press(500, move |_| { show_menu.set(true); });
/// rsx! {
///     div {
///         onpointerdown: lp.onpointerdown,
///         onpointerup: lp.onpointerup,
///         onpointermove: lp.onpointermove,
///         onpointercancel: lp.onpointercancel,
///         "Hold me"
///     }
/// }
/// ```
pub fn use_long_press(duration_ms: u32, on_press: EventHandler<()>) -> LongPressHandle {
    let mut phase = use_signal(|| LongPressPhase::Idle);
    let task_ref: Rc<RefCell<Option<dioxus_core::Task>>> = use_hook(|| Rc::new(RefCell::new(None)));
    let start_x: Rc<Cell<f64>> = use_hook(|| Rc::new(Cell::new(0.0)));
    let start_y: Rc<Cell<f64>> = use_hook(|| Rc::new(Cell::new(0.0)));
    let active_pointer: Rc<Cell<Option<i32>>> = use_hook(|| Rc::new(Cell::new(None)));

    let onpointerdown = {
        let task_ref = task_ref.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let active_pointer = active_pointer.clone();
        EventHandler::new(move |e: PointerEvent| {
            // Ignore if another pointer is already active.
            if active_pointer.get().is_some() {
                return;
            }

            let pid = e.data().pointer_id();
            active_pointer.set(Some(pid));
            start_x.set(e.client_coordinates().x);
            start_y.set(e.client_coordinates().y);
            phase.set(LongPressPhase::Pending);

            // Cancel any stale timer.
            if let Some(old) = task_ref.borrow_mut().take() {
                old.cancel();
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = duration_ms;
                phase.set(LongPressPhase::Fired);
                on_press.call(());
            }

            #[cfg(target_arch = "wasm32")]
            {
                let mut phase = phase;
                let task_ref_inner = task_ref.clone();
                let new_task = spawn(async move {
                    TimeoutFuture::new(duration_ms).await;
                    phase.set(LongPressPhase::Fired);
                    on_press.call(());
                    *task_ref_inner.borrow_mut() = None;
                });
                *task_ref.borrow_mut() = Some(new_task);
            }
        })
    };

    let onpointermove = {
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let active_pointer = active_pointer.clone();
        let task_ref = task_ref.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            let dx = e.client_coordinates().x - start_x.get();
            let dy = e.client_coordinates().y - start_y.get();
            if math::distance(0.0, 0.0, dx, dy) > MAX_DRIFT_PX {
                do_cancel(&mut phase, &task_ref, &active_pointer);
            }
        })
    };

    let onpointerup = {
        let active_pointer = active_pointer.clone();
        let task_ref = task_ref.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            do_cancel(&mut phase, &task_ref, &active_pointer);
        })
    };

    let onpointercancel = {
        let active_pointer = active_pointer.clone();
        let task_ref = task_ref.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            do_cancel(&mut phase, &task_ref, &active_pointer);
        })
    };

    LongPressHandle {
        phase: phase.into(),
        onpointerdown,
        onpointerup,
        onpointermove,
        onpointercancel,
    }
}
