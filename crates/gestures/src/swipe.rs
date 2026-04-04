use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

use crate::math::{self, SwipeDecision};
use crate::types::{SwipeConfig, SwipeHandle, SwipePhase};

/// Minimum horizontal movement in px before entering Dragging state.
const DEADZONE_PX: f64 = 5.0;

/// Duration of the Closing → Idle transition in ms (matches CSS transition).
#[cfg(target_arch = "wasm32")]
const CLOSING_DURATION_MS: u32 = 300;

/// High-resolution timestamp in ms.
fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

fn do_transition_to_closing(
    phase: &mut Signal<SwipePhase>,
    offset_px: &mut Signal<f64>,
    closing_task: &Rc<RefCell<Option<dioxus_core::Task>>>,
) {
    phase.set(SwipePhase::Closing);
    offset_px.set(0.0);

    if let Some(old) = closing_task.borrow_mut().take() {
        old.cancel();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        phase.set(SwipePhase::Idle);
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut phase = *phase;
        let closing_task_inner = closing_task.clone();
        let new_task = spawn(async move {
            TimeoutFuture::new(CLOSING_DURATION_MS).await;
            phase.set(SwipePhase::Idle);
            *closing_task_inner.borrow_mut() = None;
        });
        *closing_task.borrow_mut() = Some(new_task);
    }
}

fn do_reset(
    phase: &mut Signal<SwipePhase>,
    offset_px: &mut Signal<f64>,
    active_pointer: &Rc<Cell<Option<i32>>>,
    cancelled: &Rc<Cell<bool>>,
) {
    active_pointer.set(None);
    cancelled.set(false);
    phase.set(SwipePhase::Idle);
    offset_px.set(0.0);
}

/// Hook that detects swipe-to-reveal gestures.
///
/// Returns a [`SwipeHandle`] whose event handlers should be wired to the
/// swipeable element's pointer events. Swiping left reveals an action area;
/// when the swipe crosses the commit threshold (by distance or velocity),
/// `on_commit` fires.
///
/// # Example
/// ```rust,ignore
/// let sw = use_swipe_gesture(SwipeConfig::default(), move |_| { delete_item(); });
/// rsx! {
///     div {
///         style: "touch-action: none;",
///         onpointerdown: sw.onpointerdown,
///         onpointermove: sw.onpointermove,
///         onpointerup: sw.onpointerup,
///         onpointercancel: sw.onpointercancel,
///         "Swipe me"
///     }
/// }
/// ```
pub fn use_swipe_gesture(config: SwipeConfig, on_commit: EventHandler<()>) -> SwipeHandle {
    let mut phase = use_signal(|| SwipePhase::Idle);
    let mut offset_px = use_signal(|| 0.0_f64);

    let start_x: Rc<Cell<f64>> = use_hook(|| Rc::new(Cell::new(0.0)));
    let start_y: Rc<Cell<f64>> = use_hook(|| Rc::new(Cell::new(0.0)));
    let start_time: Rc<Cell<f64>> = use_hook(|| Rc::new(Cell::new(0.0)));
    let active_pointer: Rc<Cell<Option<i32>>> = use_hook(|| Rc::new(Cell::new(None)));
    let cancelled: Rc<Cell<bool>> = use_hook(|| Rc::new(Cell::new(false)));
    let closing_task: Rc<RefCell<Option<dioxus_core::Task>>> =
        use_hook(|| Rc::new(RefCell::new(None)));

    // Wrap in Rc so closures can share without requiring Copy.
    // Fresh each render so config updates from the parent are always picked up.
    let config_rc: Rc<SwipeConfig> = Rc::new(config);

    let onpointerdown = {
        let active_pointer = active_pointer.clone();
        let cancelled = cancelled.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let start_time = start_time.clone();
        let closing_task = closing_task.clone();
        EventHandler::new(move |e: PointerEvent| {
            let current_phase = *phase.peek();
            // If already open, close on new pointer down (tap-to-dismiss).
            if current_phase == SwipePhase::Open {
                do_transition_to_closing(&mut phase, &mut offset_px, &closing_task);
                return;
            }
            if current_phase != SwipePhase::Idle {
                return;
            }
            if active_pointer.get().is_some() {
                return;
            }

            let pid = e.data().pointer_id();
            active_pointer.set(Some(pid));
            cancelled.set(false);
            start_x.set(e.client_coordinates().x);
            start_y.set(e.client_coordinates().y);
            start_time.set(now_ms());
        })
    };

    let onpointermove = {
        let config_rc = config_rc.clone();
        let start_x = start_x.clone();
        let start_y = start_y.clone();
        let active_pointer = active_pointer.clone();
        let cancelled = cancelled.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            if cancelled.get() {
                return;
            }

            let cfg = &config_rc;
            let dx = e.client_coordinates().x - start_x.get();
            let dy = e.client_coordinates().y - start_y.get();
            let current_phase = *phase.peek();

            // Cross-axis cancellation
            if dy.abs() > cfg.max_cross_axis_px {
                cancelled.set(true);
                if current_phase == SwipePhase::Dragging {
                    do_reset(&mut phase, &mut offset_px, &active_pointer, &cancelled);
                }
                return;
            }

            match current_phase {
                SwipePhase::Idle => {
                    // Enter dragging once past the deadzone (only leftward).
                    if dx.abs() > DEADZONE_PX && dx < 0.0 {
                        phase.set(SwipePhase::Dragging);
                        let clamped = dx.max(-cfg.action_width_px).min(0.0);
                        offset_px.set(clamped);
                    }
                }
                SwipePhase::Dragging => {
                    let clamped = dx.max(-cfg.action_width_px).min(0.0);
                    offset_px.set(clamped);
                }
                _ => {}
            }
        })
    };

    let onpointerup = {
        let config_rc = config_rc.clone();
        let start_x = start_x.clone();
        let start_time = start_time.clone();
        let active_pointer = active_pointer.clone();
        let cancelled = cancelled.clone();
        let closing_task = closing_task.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            active_pointer.set(None);

            if cancelled.get() {
                cancelled.set(false);
                return;
            }

            let current_phase = *phase.peek();
            if current_phase != SwipePhase::Dragging {
                return;
            }

            let cfg = &config_rc;
            let dx = e.client_coordinates().x - start_x.get();
            let elapsed = now_ms() - start_time.get();
            let vel = math::velocity(dx, elapsed);

            let decision = math::next_swipe_phase(
                dx,
                cfg.action_width_px,
                vel,
                cfg.commit_ratio,
                cfg.velocity_threshold,
            );

            match decision {
                SwipeDecision::Commit => {
                    phase.set(SwipePhase::Open);
                    offset_px.set(-cfg.action_width_px);
                    on_commit.call(());
                }
                SwipeDecision::SpringBack => {
                    do_transition_to_closing(&mut phase, &mut offset_px, &closing_task);
                }
            }
        })
    };

    let onpointercancel = {
        let active_pointer = active_pointer.clone();
        let cancelled = cancelled.clone();
        EventHandler::new(move |e: PointerEvent| {
            if active_pointer.get() != Some(e.data().pointer_id()) {
                return;
            }
            do_reset(&mut phase, &mut offset_px, &active_pointer, &cancelled);
        })
    };

    let close = {
        let closing_task = closing_task.clone();
        EventHandler::new(move |_: ()| {
            if *phase.peek() == SwipePhase::Open {
                do_transition_to_closing(&mut phase, &mut offset_px, &closing_task);
            }
        })
    };

    let open = {
        let config_rc = config_rc.clone();
        EventHandler::new(move |_: ()| {
            phase.set(SwipePhase::Open);
            offset_px.set(-config_rc.action_width_px);
        })
    };

    SwipeHandle {
        phase: phase.into(),
        offset_px: offset_px.into(),
        onpointerdown,
        onpointermove,
        onpointerup,
        onpointercancel,
        close,
        open,
    }
}
