//! DatePicker: popover calendar + segmented input.
//!
//! ```text
//! date_picker::Root {
//!     date_picker::Trigger { "Open" }
//!     date_picker::Input {}
//!     date_picker::Popover {
//!         date_picker::Calendar {}
//!     }
//! }
//! ```

use std::rc::Rc;

use dioxus::prelude::*;
use time::Date;

use crate::segment::{DateSegment, SegmentKind};

// ── Context ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub(crate) struct DatePickerContext {
    pub open: Signal<bool>,
    pub selected_date: Signal<Option<Date>>,
    pub controlled_date: Option<Signal<Option<Date>>>,
    pub on_value_change: Option<EventHandler<Option<Date>>>,
    pub disabled: bool,
    pub read_only: bool,
    // Internal segment signals
    pub year: Signal<Option<i32>>,
    pub month: Signal<Option<i32>>,
    pub day: Signal<Option<i32>>,
}

impl DatePickerContext {
    pub fn current_date(&self) -> Option<Date> {
        match self.controlled_date {
            Some(sig) => (sig)(),
            None => (self.selected_date)(),
        }
    }

    pub fn set_date(&mut self, date: Option<Date>) {
        if let Some(mut controlled) = self.controlled_date {
            controlled.set(date);
        } else {
            self.selected_date.set(date);
        }
        // Sync segments
        match date {
            Some(d) => {
                self.year.set(Some(d.year()));
                self.month.set(Some(d.month() as i32));
                self.day.set(Some(d.day() as i32));
            }
            None => {
                self.year.set(None);
                self.month.set(None);
                self.day.set(None);
            }
        }
        if let Some(handler) = &self.on_value_change {
            handler.call(date);
        }
    }

    pub fn try_date_from_segments(&self) -> Option<Date> {
        let y = (self.year)()?;
        let m = (self.month)()? as u8;
        let d = (self.day)()? as u8;
        Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
    }
}

// ── Root ────────────────────────────────────────────────────────────

/// Context provider for a date picker.
///
/// ## Data attributes
/// - `data-state` — `"open"` / `"closed"`
/// - `data-disabled` — present when disabled
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial date (uncontrolled).
    #[props(default)]
    default_value: Option<Date>,
    /// Controlled date signal.
    #[props(default)]
    value: Option<Signal<Option<Date>>>,
    /// Fires when the date changes.
    #[props(default)]
    on_value_change: Option<EventHandler<Option<Date>>>,
    /// Disable the entire picker.
    #[props(default)]
    disabled: bool,
    /// Read-only mode.
    #[props(default)]
    read_only: bool,
    children: Element,
) -> Element {
    let open = use_signal(|| false);
    let selected_date = use_signal(|| default_value);
    let year = use_signal(|| default_value.map(|d| d.year()));
    let month = use_signal(|| default_value.map(|d| d.month() as i32));
    let day = use_signal(|| default_value.map(|d| d.day() as i32));

    let ctx = DatePickerContext {
        open,
        selected_date,
        controlled_date: value,
        on_value_change,
        disabled,
        read_only,
        year,
        month,
        day,
    };

    use_context_provider(|| ctx);

    let state = if (open)() { "open" } else { "closed" };

    rsx! {
        div {
            "data-state": state,
            "data-disabled": disabled.then_some("true"),
            ..attributes,
            {children}
        }
    }
}

// ── Trigger ─────────────────────────────────────────────────────────

/// Button that toggles the popover open/closed.
#[component]
pub fn Trigger(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DatePickerContext = use_context();

    let onclick = move |_: MouseEvent| {
        if ctx.disabled || ctx.read_only {
            return;
        }
        let mut open = ctx.open;
        open.set(!(ctx.open)());
    };

    rsx! {
        button {
            r#type: "button",
            aria_haspopup: "dialog",
            aria_expanded: if (ctx.open)() { "true" } else { "false" },
            disabled: ctx.disabled,
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Popover ─────────────────────────────────────────────────────────

/// Conditional container for the calendar. Renders children only when open.
///
/// ## Data attributes
/// - `data-state` — `"open"` / `"closed"`
#[component]
pub fn Popover(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DatePickerContext = use_context();
    let is_open = (ctx.open)();
    let state = if is_open { "open" } else { "closed" };

    let onkeydown = move |e: KeyboardEvent| {
        if e.key() == Key::Escape {
            let mut open = ctx.open;
            open.set(false);
        }
    };

    rsx! {
        div {
            "data-state": state,
            role: "dialog",
            aria_modal: "true",
            onkeydown,
            ..attributes,
            if is_open {
                {children}
            }
        }
    }
}

// ── Input ───────────────────────────────────────────────────────────

/// Segmented YYYY-MM-DD input wired to the DatePickerContext.
#[component]
pub fn Input(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let ctx: DatePickerContext = use_context();

    let mut year_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut month_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut day_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    let focus_segment = move |target: Signal<Option<Rc<MountedData>>>| {
        spawn(async move {
            if let Some(el) = (target)() {
                _ = el.set_focus(true).await;
            }
        });
    };

    let context_year = (ctx.year)().unwrap_or(2026);
    let context_month = (ctx.month)().unwrap_or(1) as u8;

    let notify = move || {
        let mut ctx: DatePickerContext = consume_context();
        if let Some(date) = ctx.try_date_from_segments() {
            ctx.set_date(Some(date));
        } else if let Some(handler) = &ctx.on_value_change {
            handler.call(None);
        }
    };

    rsx! {
        div {
            role: "group",
            "data-slot": "date-picker-input",
            ..attributes,

            DateSegment {
                kind: SegmentKind::Year,
                value: ctx.year,
                context_year,
                context_month,
                disabled: ctx.disabled,
                read_only: ctx.read_only,
                on_change: move |v| {
                    let mut year = ctx.year;
                    year.set(v);
                    notify();
                },
                on_advance: move |_| focus_segment(month_ref),
                on_mounted: move |e: MountedEvent| year_ref.set(Some(e.data())),
            }

            span { aria_hidden: "true", "-" }

            DateSegment {
                kind: SegmentKind::Month,
                value: ctx.month,
                context_year,
                context_month,
                disabled: ctx.disabled,
                read_only: ctx.read_only,
                on_change: move |v| {
                    let mut month = ctx.month;
                    month.set(v);
                    notify();
                },
                on_advance: move |_| focus_segment(day_ref),
                on_retreat: move |_| focus_segment(year_ref),
                on_mounted: move |e: MountedEvent| month_ref.set(Some(e.data())),
            }

            span { aria_hidden: "true", "-" }

            DateSegment {
                kind: SegmentKind::Day,
                value: ctx.day,
                context_year,
                context_month,
                disabled: ctx.disabled,
                read_only: ctx.read_only,
                on_change: move |v| {
                    let mut day = ctx.day;
                    day.set(v);
                    notify();
                },
                on_retreat: move |_| focus_segment(month_ref),
                on_mounted: move |e: MountedEvent| day_ref.set(Some(e.data())),
            }
        }
    }
}

// ── Calendar ────────────────────────────────────────────────────────

/// Pre-wired calendar::Root that auto-closes the popover on select.
///
/// Wraps the calendar with auto-close behavior.
#[component]
pub fn Calendar(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let ctx: DatePickerContext = use_context();

    let on_value_change = move |date: Option<Date>| {
        let mut ctx: DatePickerContext = consume_context();
        ctx.set_date(date);
        // Auto-close on selection
        if date.is_some() {
            let mut open = ctx.open;
            open.set(false);
        }
    };

    rsx! {
        dioxus_nox_calendar::calendar::Root {
            class,
            default_value: ctx.current_date(),
            on_value_change: on_value_change,
            disabled: ctx.disabled,
            {children}
        }
    }
}
