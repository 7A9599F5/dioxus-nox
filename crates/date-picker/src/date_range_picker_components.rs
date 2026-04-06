//! DateRangePicker: popover calendar + two segmented inputs.
//!
//! ```text
//! date_range_picker::Root {
//!     date_range_picker::Trigger { "Open" }
//!     date_range_picker::InputStart {}
//!     date_range_picker::InputEnd {}
//!     date_range_picker::Popover {
//!         date_range_picker::Calendar {}
//!     }
//! }
//! ```

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_nox_calendar::DateRange;
use time::Date;

use crate::segment::{DateSegment, SegmentKind};

// ── Context ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub(crate) struct DateRangePickerContext {
    pub open: Signal<bool>,
    pub selected_range: Signal<Option<DateRange>>,
    pub controlled_range: Option<Signal<Option<DateRange>>>,
    pub on_range_change: Option<EventHandler<Option<DateRange>>>,
    pub disabled: bool,
    pub read_only: bool,
    // Start segments
    pub start_year: Signal<Option<i32>>,
    pub start_month: Signal<Option<i32>>,
    pub start_day: Signal<Option<i32>>,
    // End segments
    pub end_year: Signal<Option<i32>>,
    pub end_month: Signal<Option<i32>>,
    pub end_day: Signal<Option<i32>>,
}

impl DateRangePickerContext {
    pub fn current_range(&self) -> Option<DateRange> {
        match self.controlled_range {
            Some(sig) => (sig)(),
            None => (self.selected_range)(),
        }
    }

    pub fn set_range(&mut self, range: Option<DateRange>) {
        if let Some(mut controlled) = self.controlled_range {
            controlled.set(range);
        } else {
            self.selected_range.set(range);
        }
        // Sync segments
        match range {
            Some(r) => {
                self.start_year.set(Some(r.start().year()));
                self.start_month.set(Some(r.start().month() as i32));
                self.start_day.set(Some(r.start().day() as i32));
                self.end_year.set(Some(r.end().year()));
                self.end_month.set(Some(r.end().month() as i32));
                self.end_day.set(Some(r.end().day() as i32));
            }
            None => {
                self.start_year.set(None);
                self.start_month.set(None);
                self.start_day.set(None);
                self.end_year.set(None);
                self.end_month.set(None);
                self.end_day.set(None);
            }
        }
        if let Some(handler) = &self.on_range_change {
            handler.call(range);
        }
    }

    fn try_start(&self) -> Option<Date> {
        let y = (self.start_year)()?;
        let m = (self.start_month)()? as u8;
        let d = (self.start_day)()? as u8;
        Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
    }

    fn try_end(&self) -> Option<Date> {
        let y = (self.end_year)()?;
        let m = (self.end_month)()? as u8;
        let d = (self.end_day)()? as u8;
        Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
    }

    pub fn try_range_from_segments(&self) -> Option<DateRange> {
        let start = self.try_start()?;
        let end = self.try_end()?;
        Some(DateRange::new(start, end))
    }
}

// ── Root ────────────────────────────────────────────────────────────

/// Context provider for a date range picker.
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial range (uncontrolled).
    #[props(default)]
    default_value: Option<DateRange>,
    /// Controlled range signal.
    #[props(default)]
    value: Option<Signal<Option<DateRange>>>,
    /// Fires when the range changes.
    #[props(default)]
    on_range_change: Option<EventHandler<Option<DateRange>>>,
    /// Disable the entire picker.
    #[props(default)]
    disabled: bool,
    /// Read-only mode.
    #[props(default)]
    read_only: bool,
    children: Element,
) -> Element {
    let open = use_signal(|| false);
    let selected_range = use_signal(|| default_value);

    let start_year = use_signal(|| default_value.map(|r| r.start().year()));
    let start_month = use_signal(|| default_value.map(|r| r.start().month() as i32));
    let start_day = use_signal(|| default_value.map(|r| r.start().day() as i32));
    let end_year = use_signal(|| default_value.map(|r| r.end().year()));
    let end_month = use_signal(|| default_value.map(|r| r.end().month() as i32));
    let end_day = use_signal(|| default_value.map(|r| r.end().day() as i32));

    let ctx = DateRangePickerContext {
        open,
        selected_range,
        controlled_range: value,
        on_range_change,
        disabled,
        read_only,
        start_year,
        start_month,
        start_day,
        end_year,
        end_month,
        end_day,
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
    let ctx: DateRangePickerContext = use_context();

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

/// Conditional container for the range calendar.
#[component]
pub fn Popover(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DateRangePickerContext = use_context();
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

// ── Shared segment input helper ────────────────────────────────────

#[component]
fn SegmentedInput(
    year: Signal<Option<i32>>,
    month: Signal<Option<i32>>,
    day: Signal<Option<i32>>,
    on_change: EventHandler<()>,
    disabled: bool,
    read_only: bool,
    slot_name: &'static str,
    #[props(default)] class: Option<String>,
) -> Element {
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

    let context_year = (year)().unwrap_or(2026);
    let context_month = (month)().unwrap_or(1) as u8;

    rsx! {
        div {
            role: "group",
            class,
            "data-slot": slot_name,

            DateSegment {
                kind: SegmentKind::Year,
                value: year,
                context_year,
                context_month,
                disabled,
                read_only,
                on_change: move |v| {
                    let mut y = year;
                    y.set(v);
                    on_change.call(());
                },
                on_advance: move |_| focus_segment(month_ref),
                on_mounted: move |e: MountedEvent| year_ref.set(Some(e.data())),
            }

            span { aria_hidden: "true", "-" }

            DateSegment {
                kind: SegmentKind::Month,
                value: month,
                context_year,
                context_month,
                disabled,
                read_only,
                on_change: move |v| {
                    let mut m = month;
                    m.set(v);
                    on_change.call(());
                },
                on_advance: move |_| focus_segment(day_ref),
                on_retreat: move |_| focus_segment(year_ref),
                on_mounted: move |e: MountedEvent| month_ref.set(Some(e.data())),
            }

            span { aria_hidden: "true", "-" }

            DateSegment {
                kind: SegmentKind::Day,
                value: day,
                context_year,
                context_month,
                disabled,
                read_only,
                on_change: move |v| {
                    let mut d = day;
                    d.set(v);
                    on_change.call(());
                },
                on_retreat: move |_| focus_segment(month_ref),
                on_mounted: move |e: MountedEvent| day_ref.set(Some(e.data())),
            }
        }
    }
}

// ── InputStart ──────────────────────────────────────────────────────

/// Segmented input for the range start date.
#[component]
pub fn InputStart(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: DateRangePickerContext = use_context();

    let on_change = move |_: ()| {
        let mut ctx: DateRangePickerContext = consume_context();
        if let Some(range) = ctx.try_range_from_segments() {
            ctx.set_range(Some(range));
        } else if let Some(handler) = &ctx.on_range_change {
            handler.call(None);
        }
    };

    rsx! {
        SegmentedInput {
            year: ctx.start_year,
            month: ctx.start_month,
            day: ctx.start_day,
            on_change,
            disabled: ctx.disabled,
            read_only: ctx.read_only,
            slot_name: "range-start-input",
            class,
        }
    }
}

// ── InputEnd ────────────────────────────────────────────────────────

/// Segmented input for the range end date.
#[component]
pub fn InputEnd(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: DateRangePickerContext = use_context();

    let on_change = move |_: ()| {
        let mut ctx: DateRangePickerContext = consume_context();
        if let Some(range) = ctx.try_range_from_segments() {
            ctx.set_range(Some(range));
        } else if let Some(handler) = &ctx.on_range_change {
            handler.call(None);
        }
    };

    rsx! {
        SegmentedInput {
            year: ctx.end_year,
            month: ctx.end_month,
            day: ctx.end_day,
            on_change,
            disabled: ctx.disabled,
            read_only: ctx.read_only,
            slot_name: "range-end-input",
            class,
        }
    }
}

// ── Calendar ────────────────────────────────────────────────────────

/// Pre-wired range_calendar::Root that auto-closes on committed range.
#[component]
pub fn Calendar(
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let ctx: DateRangePickerContext = use_context();

    let on_range_change = move |range: Option<DateRange>| {
        let mut ctx: DateRangePickerContext = consume_context();
        ctx.set_range(range);
        // Auto-close after a complete range is committed
        if range.is_some() {
            let mut open = ctx.open;
            open.set(false);
        }
    };

    rsx! {
        dioxus_nox_calendar::range_calendar::Root {
            class,
            default_value: ctx.current_range(),
            on_range_change: on_range_change,
            disabled: ctx.disabled,
            {children}
        }
    }
}
