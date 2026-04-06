//! Standalone segmented date input (no popover).
//!
//! ```text
//! date_field::Root {
//!     date_field::Input {}
//! }
//! ```

use std::rc::Rc;

use dioxus::prelude::*;
use time::Date;

use crate::segment::{DateSegment, SegmentKind};

// ── Context ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub(crate) struct DateFieldContext {
    pub year: Signal<Option<i32>>,
    pub month: Signal<Option<i32>>,
    pub day: Signal<Option<i32>>,
    pub disabled: bool,
    pub read_only: bool,
    pub on_value_change: Option<EventHandler<Option<Date>>>,
}

impl DateFieldContext {
    /// Try to build a valid Date from the three segments.
    pub fn try_date(&self) -> Option<Date> {
        let y = (self.year)()?;
        let m = (self.month)()? as u8;
        let d = (self.day)()? as u8;
        Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
    }

    /// Fire the on_value_change callback if all segments are set.
    pub fn notify(&self) {
        if let Some(handler) = &self.on_value_change {
            handler.call(self.try_date());
        }
    }
}

// ── Root ────────────────────────────────────────────────────────────

/// Context provider for a standalone date field.
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial date value.
    #[props(default)]
    default_value: Option<Date>,
    /// Fires when the date changes (or becomes invalid/empty).
    #[props(default)]
    on_value_change: Option<EventHandler<Option<Date>>>,
    /// Disable the field.
    #[props(default)]
    disabled: bool,
    /// Read-only mode.
    #[props(default)]
    read_only: bool,
    children: Element,
) -> Element {
    let year = use_signal(|| default_value.map(|d| d.year()));
    let month = use_signal(|| default_value.map(|d| d.month() as i32));
    let day = use_signal(|| default_value.map(|d| d.day() as i32));

    let ctx = DateFieldContext {
        year,
        month,
        day,
        disabled,
        read_only,
        on_value_change,
    };

    use_context_provider(|| ctx);

    rsx! {
        div {
            role: "group",
            aria_label: "Date",
            "data-disabled": disabled.then_some("true"),
            "data-readonly": read_only.then_some("true"),
            ..attributes,
            {children}
        }
    }
}

// ── Input ───────────────────────────────────────────────────────────

/// The segmented YYYY-MM-DD date input.
///
/// Contains three `DateSegment` spinbuttons with separator characters.
#[component]
pub fn Input(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let ctx: DateFieldContext = use_context();

    // Focus refs for the three segments
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

    rsx! {
        div {
            role: "group",
            "data-slot": "date-field-input",
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
                    ctx.notify();
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
                    ctx.notify();
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
                    ctx.notify();
                },
                on_retreat: move |_| focus_segment(month_ref),
                on_mounted: move |e: MountedEvent| day_ref.set(Some(e.data())),
            }
        }
    }
}
