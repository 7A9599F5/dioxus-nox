//! Shared date segment spinbutton component.
//!
//! Three segments (year, month, day) compose to form a complete date input.
//! Each segment is a `role="spinbutton"` with arrow key increment/decrement,
//! digit typing, and auto-advance on max length.

use dioxus::prelude::*;

/// Which part of the date this segment represents.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    Year,
    Month,
    Day,
}

impl SegmentKind {
    /// Maximum number of digits for this segment.
    pub fn max_digits(self) -> usize {
        match self {
            Self::Year => 4,
            Self::Month | Self::Day => 2,
        }
    }

    /// Human-readable label for aria.
    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Year => "Year",
            Self::Month => "Month",
            Self::Day => "Day",
        }
    }

    /// Placeholder text (e.g., "YYYY", "MM", "DD").
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Year => "YYYY",
            Self::Month => "MM",
            Self::Day => "DD",
        }
    }
}

/// Clamp and wrap a segment value within valid bounds.
pub fn clamp_segment(kind: SegmentKind, value: i32, year: i32, month: u8) -> i32 {
    match kind {
        SegmentKind::Year => value.clamp(1, 9999),
        SegmentKind::Month => {
            if value < 1 {
                12
            } else if value > 12 {
                1
            } else {
                value
            }
        }
        SegmentKind::Day => {
            let max = time::Month::try_from(month)
                .map(|m| m.length(year))
                .unwrap_or(31) as i32;
            if value < 1 {
                max
            } else if value > max {
                1
            } else {
                value
            }
        }
    }
}

/// Format a segment value with leading zeros.
pub fn format_segment(kind: SegmentKind, value: i32) -> String {
    match kind {
        SegmentKind::Year => format!("{value:04}"),
        SegmentKind::Month | SegmentKind::Day => format!("{value:02}"),
    }
}

/// A single date segment spinbutton (year, month, or day).
///
/// ## Data attributes
/// - `data-placeholder` — present when no value has been entered
/// - `data-segment` — `"year"` / `"month"` / `"day"`
#[component]
pub fn DateSegment(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Which segment this is.
    kind: SegmentKind,
    /// Current value of this segment.
    value: Signal<Option<i32>>,
    /// Called when the value changes.
    on_change: EventHandler<Option<i32>>,
    /// Called when the user wants to move to the next segment.
    #[props(default)]
    on_advance: Option<EventHandler<()>>,
    /// Called when the user wants to move to the previous segment.
    #[props(default)]
    on_retreat: Option<EventHandler<()>>,
    /// Year value (needed for day max calculation).
    #[props(default = 2026)]
    context_year: i32,
    /// Month value (needed for day max calculation).
    #[props(default = 1)]
    context_month: u8,
    /// Whether the segment is disabled.
    #[props(default)]
    disabled: bool,
    /// Whether the segment is read-only.
    #[props(default)]
    read_only: bool,
    /// Callback when the segment element is mounted (for focus management).
    #[props(default)]
    on_mounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    let mut typed_digits = use_signal(String::new);

    let display = match (value)() {
        Some(v) => format_segment(kind, v),
        None => kind.placeholder().to_string(),
    };

    let is_placeholder = (value)().is_none();

    let (min, max) = match kind {
        SegmentKind::Year => (1, 9999),
        SegmentKind::Month => (1, 12),
        SegmentKind::Day => {
            let m = time::Month::try_from(context_month)
                .map(|m| m.length(context_year))
                .unwrap_or(31);
            (1, m as i32)
        }
    };

    let onkeydown = move |e: KeyboardEvent| {
        if disabled || read_only {
            return;
        }
        match e.key() {
            Key::ArrowUp => {
                e.prevent_default();
                let current = (value)().unwrap_or(min);
                let new_val = clamp_segment(kind, current + 1, context_year, context_month);
                on_change.call(Some(new_val));
                typed_digits.set(String::new());
            }
            Key::ArrowDown => {
                e.prevent_default();
                let current = (value)().unwrap_or(min);
                let new_val = clamp_segment(kind, current - 1, context_year, context_month);
                on_change.call(Some(new_val));
                typed_digits.set(String::new());
            }
            Key::ArrowRight => {
                e.prevent_default();
                if let Some(handler) = &on_advance {
                    handler.call(());
                }
                typed_digits.set(String::new());
            }
            Key::ArrowLeft => {
                e.prevent_default();
                if let Some(handler) = &on_retreat {
                    handler.call(());
                }
                typed_digits.set(String::new());
            }
            Key::Backspace => {
                e.prevent_default();
                let mut digits = (typed_digits)();
                if digits.pop().is_some() {
                    typed_digits.set(digits.clone());
                    if digits.is_empty() {
                        on_change.call(None);
                    } else if let Ok(v) = digits.parse::<i32>() {
                        on_change.call(Some(v.clamp(min, max)));
                    }
                } else {
                    on_change.call(None);
                    if let Some(handler) = &on_retreat {
                        handler.call(());
                    }
                }
            }
            Key::Character(ref c)
                if c.len() == 1 && c.chars().next().is_some_and(|ch| ch.is_ascii_digit()) =>
            {
                e.prevent_default();
                let mut digits = (typed_digits)();
                digits.push_str(c);

                // If digits exceed max length, start fresh
                if digits.len() > kind.max_digits() {
                    digits = c.clone();
                }

                typed_digits.set(digits.clone());

                if let Ok(v) = digits.parse::<i32>() {
                    let clamped = v.clamp(min, max);
                    on_change.call(Some(clamped));

                    // Auto-advance when max digits reached
                    if digits.len() >= kind.max_digits() {
                        typed_digits.set(String::new());
                        if let Some(handler) = &on_advance {
                            handler.call(());
                        }
                    }
                }
            }
            _ => {}
        }
    };

    let segment_name = match kind {
        SegmentKind::Year => "year",
        SegmentKind::Month => "month",
        SegmentKind::Day => "day",
    };

    rsx! {
        span {
            role: "spinbutton",
            tabindex: if disabled { "-1" } else { "0" },
            aria_label: kind.aria_label(),
            aria_valuemin: "{min}",
            aria_valuemax: "{max}",
            aria_valuenow: if let Some(v) = (value)() { format!("{v}") } else { String::new() },
            aria_disabled: disabled.then_some("true"),
            aria_readonly: read_only.then_some("true"),
            "data-segment": segment_name,
            "data-placeholder": is_placeholder.then_some("true"),
            onkeydown,
            onmounted: move |e: MountedEvent| {
                if let Some(handler) = &on_mounted {
                    handler.call(e);
                }
            },
            ..attributes,
            {display}
        }
    }
}
