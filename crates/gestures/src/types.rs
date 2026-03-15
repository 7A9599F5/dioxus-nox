use dioxus::prelude::*;

// ── Swipe ────────────────────────────────────────────────────────────────────

/// Configuration for [`use_swipe_gesture`](crate::use_swipe_gesture).
#[derive(Clone, Debug, PartialEq)]
pub struct SwipeConfig {
    /// Ratio of `action_width_px` that triggers commit (default 0.40).
    pub commit_ratio: f64,
    /// Velocity threshold in px/ms to commit via fast swipe (default 0.5).
    pub velocity_threshold: f64,
    /// Maximum vertical drift in px before cancelling a horizontal swipe (default 30.0).
    pub max_cross_axis_px: f64,
    /// Width of the revealed action area in px (default 80.0).
    pub action_width_px: f64,
}

impl Default for SwipeConfig {
    fn default() -> Self {
        Self {
            commit_ratio: 0.40,
            velocity_threshold: 0.5,
            max_cross_axis_px: 30.0,
            action_width_px: 80.0,
        }
    }
}

/// State machine for swipe gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwipePhase {
    #[default]
    Idle,
    /// Pointer is down and moving horizontally.
    Dragging,
    /// Swipe committed — actions are fully revealed.
    Open,
    /// Transitioning back to Idle (CSS transition handles animation).
    Closing,
}

impl SwipePhase {
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Dragging => "dragging",
            Self::Open => "open",
            Self::Closing => "closing",
        }
    }
}

/// Handle returned by [`use_swipe_gesture`](crate::use_swipe_gesture).
///
/// Wire the event handlers into your swipeable element's pointer events.
#[derive(Clone)]
pub struct SwipeHandle {
    /// Current phase of the swipe gesture.
    pub phase: ReadSignal<SwipePhase>,
    /// Current horizontal offset in px (negative = swiped left).
    pub offset_px: ReadSignal<f64>,
    /// Wire to `onpointerdown`.
    pub onpointerdown: EventHandler<PointerEvent>,
    /// Wire to `onpointermove`.
    pub onpointermove: EventHandler<PointerEvent>,
    /// Wire to `onpointerup`.
    pub onpointerup: EventHandler<PointerEvent>,
    /// Wire to `onpointercancel`.
    pub onpointercancel: EventHandler<PointerEvent>,
    /// Programmatically close the swipe (Open → Closing → Idle).
    pub close: EventHandler<()>,
    /// Programmatically open the swipe.
    pub open: EventHandler<()>,
}

// ── Long Press ───────────────────────────────────────────────────────────────

/// State machine for long-press gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LongPressPhase {
    #[default]
    Idle,
    /// Pointer is down, timer is running.
    Pending,
    /// Timer fired — long press activated.
    Fired,
}

impl LongPressPhase {
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Pending => "pending",
            Self::Fired => "fired",
        }
    }
}

/// Handle returned by [`use_long_press`](crate::use_long_press).
///
/// Wire the event handlers into your target element's pointer events.
#[derive(Clone)]
pub struct LongPressHandle {
    /// Current phase.
    pub phase: ReadSignal<LongPressPhase>,
    /// Wire to `onpointerdown`.
    pub onpointerdown: EventHandler<PointerEvent>,
    /// Wire to `onpointerup`.
    pub onpointerup: EventHandler<PointerEvent>,
    /// Wire to `onpointermove` (cancels if pointer drifts too far).
    pub onpointermove: EventHandler<PointerEvent>,
    /// Wire to `onpointercancel`.
    pub onpointercancel: EventHandler<PointerEvent>,
}
