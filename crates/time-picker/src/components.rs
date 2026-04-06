use dioxus::prelude::*;
use time::Time;

use crate::context::*;

// ── Root ────────────────────────────────────────────────────────────

/// Context provider for the time picker.
///
/// ## Data attributes
/// - `data-disabled` — present when disabled
/// - `data-readonly` — present when read-only
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial time value (uncontrolled).
    #[props(default)]
    default_value: Option<Time>,
    /// Fires when the time changes.
    #[props(default)]
    on_change: Option<EventHandler<Option<Time>>>,
    /// Use 12-hour mode with AM/PM.
    #[props(default)]
    use_12_hour: bool,
    /// Show seconds segment.
    #[props(default)]
    show_seconds: bool,
    /// Disable the entire picker.
    #[props(default)]
    disabled: bool,
    /// Read-only mode.
    #[props(default)]
    read_only: bool,
    children: Element,
) -> Element {
    let hour = use_signal(|| default_value.map(|t| t.hour()));
    let minute = use_signal(|| default_value.map(|t| t.minute()));
    let second = use_signal(|| default_value.map(|t| t.second()));

    let ctx = TimePickerContext {
        hour,
        minute,
        second,
        use_12_hour,
        show_seconds,
        disabled,
        read_only,
        on_change,
    };

    use_context_provider(|| ctx);

    rsx! {
        div {
            role: "group",
            aria_label: "Time",
            "data-disabled": disabled.then_some("true"),
            "data-readonly": read_only.then_some("true"),
            ..attributes,
            {children}
        }
    }
}

// ── Time segment spinbutton ────────────────────────────────────────

#[component]
fn TimeSpinbutton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    kind: TimeSegmentKind,
    value: Signal<Option<u8>>,
    on_value_change: EventHandler<Option<u8>>,
    disabled: bool,
    read_only: bool,
    /// Called to advance focus to next segment.
    #[props(default)]
    on_advance: Option<EventHandler<()>>,
    /// Called to retreat focus to previous segment.
    #[props(default)]
    on_retreat: Option<EventHandler<()>>,
) -> Element {
    let mut typed_digits = use_signal(String::new);

    let display = match (value)() {
        Some(v) => format!("{v:02}"),
        None => kind.placeholder().to_string(),
    };
    let is_placeholder = (value)().is_none();
    let min = kind.min_value();
    let max = kind.max_value();

    let onkeydown = move |e: KeyboardEvent| {
        if disabled || read_only {
            return;
        }
        match e.key() {
            Key::ArrowUp => {
                e.prevent_default();
                let current = (value)().unwrap_or(min) as i32;
                on_value_change.call(Some(clamp_time_segment(kind, current + 1)));
                typed_digits.set(String::new());
            }
            Key::ArrowDown => {
                e.prevent_default();
                let current = (value)().unwrap_or(min) as i32;
                on_value_change.call(Some(clamp_time_segment(kind, current - 1)));
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
                        on_value_change.call(None);
                    } else if let Ok(v) = digits.parse::<i32>() {
                        on_value_change.call(Some(clamp_time_segment(kind, v)));
                    }
                } else {
                    on_value_change.call(None);
                    if let Some(handler) = &on_retreat {
                        handler.call(());
                    }
                }
            }
            Key::Character(ref c) if c.len() == 1 && c.chars().next().is_some_and(|ch| ch.is_ascii_digit()) => {
                e.prevent_default();
                let mut digits = (typed_digits)();
                digits.push_str(c);

                if digits.len() > kind.max_digits() {
                    digits = c.clone();
                }

                typed_digits.set(digits.clone());

                if let Ok(v) = digits.parse::<i32>() {
                    on_value_change.call(Some(clamp_time_segment(kind, v)));

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
            "data-segment": kind.aria_label().to_lowercase(),
            "data-placeholder": is_placeholder.then_some("true"),
            onkeydown,
            ..attributes,
            {display}
        }
    }
}

// ── Hour ────────────────────────────────────────────────────────────

/// Hour spinbutton (0-23 in 24h mode, 1-12 in 12h mode).
#[component]
pub fn Hour(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: TimePickerContext = use_context();
    let kind = if ctx.use_12_hour {
        TimeSegmentKind::Hour12
    } else {
        TimeSegmentKind::Hour24
    };

    // For 12-hour display, convert from 24h internal value
    let display_value = use_memo(move || {
        (ctx.hour)().map(|h| {
            if ctx.use_12_hour {
                match h % 12 {
                    0 => 12,
                    other => other,
                }
            } else {
                h
            }
        })
    });

    #[allow(clippy::redundant_closure)]
    let mut display_sig = use_signal(|| (display_value)());
    use_effect(move || {
        let v = (display_value)();
        display_sig.set(v);
    });

    let on_value_change = move |v: Option<u8>| {
        let mut hour = ctx.hour;
        let stored = v.map(|h| {
            if ctx.use_12_hour {
                let is_pm = ctx.hour().is_some_and(|curr| curr >= 12);
                let base = if h == 12 { 0 } else { h };
                if is_pm { base + 12 } else { base }
            } else {
                h
            }
        });
        hour.set(stored);
        ctx.notify();
    };

    rsx! {
        TimeSpinbutton {
            kind,
            value: display_sig,
            disabled: ctx.disabled,
            read_only: ctx.read_only,
            on_value_change,
        }
    }
}

// ── Minute ──────────────────────────────────────────────────────────

/// Minute spinbutton (0-59).
#[component]
pub fn Minute(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: TimePickerContext = use_context();

    rsx! {
        TimeSpinbutton {
            kind: TimeSegmentKind::Minute,
            value: ctx.minute,
            disabled: ctx.disabled,
            read_only: ctx.read_only,
            on_value_change: move |v: Option<u8>| {
                let mut minute = ctx.minute;
                minute.set(v);
                ctx.notify();
            },
        }
    }
}

// ── Second ──────────────────────────────────────────────────────────

/// Second spinbutton (0-59). Only visible when `show_seconds` is true on Root.
#[component]
pub fn Second(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: TimePickerContext = use_context();

    rsx! {
        TimeSpinbutton {
            kind: TimeSegmentKind::Second,
            value: ctx.second,
            disabled: ctx.disabled,
            read_only: ctx.read_only,
            on_value_change: move |v: Option<u8>| {
                let mut second = ctx.second;
                second.set(v);
                ctx.notify();
            },
        }
    }
}

// ── Period ──────────────────────────────────────────────────────────

/// AM/PM toggle button (12-hour mode).
///
/// ## Data attributes
/// - `data-period` — `"am"` or `"pm"`
#[component]
pub fn Period(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: TimePickerContext = use_context();
    let is_am = ctx.is_am();
    let label = if is_am { "AM" } else { "PM" };
    let period = if is_am { "am" } else { "pm" };

    let onclick = move |_: MouseEvent| {
        if ctx.disabled || ctx.read_only {
            return;
        }
        let ctx: TimePickerContext = consume_context();
        ctx.toggle_period();
    };

    let onkeydown = move |e: KeyboardEvent| {
        if ctx.disabled || ctx.read_only {
            return;
        }
        match e.key() {
            Key::ArrowUp | Key::ArrowDown => {
                e.prevent_default();
                let ctx: TimePickerContext = consume_context();
                ctx.toggle_period();
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("a") => {
                e.prevent_default();
                if !ctx.is_am() {
                    let ctx: TimePickerContext = consume_context();
                    ctx.toggle_period();
                }
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("p") => {
                e.prevent_default();
                if ctx.is_am() {
                    let ctx: TimePickerContext = consume_context();
                    ctx.toggle_period();
                }
            }
            _ => {}
        }
    };

    rsx! {
        button {
            r#type: "button",
            role: "spinbutton",
            tabindex: if ctx.disabled { "-1" } else { "0" },
            aria_label: "AM/PM",
            aria_valuetext: label,
            "data-period": period,
            disabled: ctx.disabled,
            onclick,
            onkeydown,
            ..attributes,
            {label}
        }
    }
}

// ── Separator ───────────────────────────────────────────────────────

/// Decorative separator between time segments (typically ":").
#[component]
pub fn Separator(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    #[props(default = ":".to_string())]
    #[props(into)]
    text: String,
) -> Element {
    rsx! {
        span {
            aria_hidden: "true",
            "data-slot": "separator",
            ..attributes,
            {text}
        }
    }
}
