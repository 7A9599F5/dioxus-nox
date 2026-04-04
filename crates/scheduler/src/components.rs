use chrono::{Datelike, NaiveDate, Timelike};
use dioxus::prelude::*;

use crate::navigation::week_dates;
use crate::types::{EventDropData, EventResizeData, SchedulerContext, SchedulerView, TimeSlotData};

// ── Root ────────────────────────────────────────────────────────────────────

/// Context provider for the scheduler compound component.
///
/// ## Data attributes
/// - `data-scheduler-view="day|week|agenda"`
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial view mode.
    #[props(default)]
    view: SchedulerView,
    /// Initial date to display (defaults to today).
    #[props(default)]
    initial_date: Option<NaiveDate>,
    /// Time slot granularity in minutes (default: 30).
    #[props(default = 30)]
    slot_minutes: u32,
    /// First visible hour (default: 6).
    #[props(default = 6)]
    day_start_hour: u32,
    /// Last visible hour (default: 22).
    #[props(default = 22)]
    day_end_hour: u32,
    /// Callback when an event is clicked.
    #[props(default)]
    on_event_click: Option<EventHandler<String>>,
    /// Callback when a time slot is clicked.
    #[props(default)]
    on_slot_click: Option<EventHandler<TimeSlotData>>,
    /// Callback when an event is resized.
    #[props(default)]
    on_event_resize: Option<EventHandler<EventResizeData>>,
    /// Callback when an event is dropped.
    #[props(default)]
    on_event_drop: Option<EventHandler<EventDropData>>,
    /// Callback when view mode changes.
    #[props(default)]
    on_view_change: Option<EventHandler<SchedulerView>>,
    /// Callback when displayed date changes.
    #[props(default)]
    on_date_change: Option<EventHandler<NaiveDate>>,
    children: Element,
) -> Element {
    let today = chrono::Local::now().date_naive();
    let date = initial_date.unwrap_or(today);

    let ctx = SchedulerContext {
        view: use_signal(|| view),
        current_date: use_signal(|| date),
        events: use_signal(Vec::new),
        selected_event: use_signal(|| None),
        slot_height_minutes: slot_minutes,
        day_start_hour,
        day_end_hour,
        on_event_click,
        on_slot_click,
        on_event_resize,
        on_event_drop,
        on_view_change,
        on_date_change,
    };

    use_context_provider(|| ctx);

    let view_attr = ctx.view().as_data_attr();

    rsx! {
        div {
            role: "application",
            "aria-label": "Scheduler",
            "data-scheduler-view": view_attr,
            ..attributes,
            {children}
        }
    }
}

// ── Header ──────────────────────────────────────────────────────────────────

/// Navigation bar container for the scheduler.
#[component]
pub fn Header(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "toolbar",
            "aria-label": "Scheduler navigation",
            ..attributes,
            {children}
        }
    }
}

// ── PrevButton ──────────────────────────────────────────────────────────────

/// Navigate to the previous period (day or week depending on view).
#[component]
pub fn PrevButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let onclick = move |_: MouseEvent| {
        let mut ctx: SchedulerContext = consume_context();
        ctx.go_prev();
    };

    rsx! {
        button {
            "type": "button",
            "aria-label": "Previous",
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── NextButton ──────────────────────────────────────────────────────────────

/// Navigate to the next period.
#[component]
pub fn NextButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let onclick = move |_: MouseEvent| {
        let mut ctx: SchedulerContext = consume_context();
        ctx.go_next();
    };

    rsx! {
        button {
            "type": "button",
            "aria-label": "Next",
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── TodayButton ─────────────────────────────────────────────────────────────

/// Navigate to today.
#[component]
pub fn TodayButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let onclick = move |_: MouseEvent| {
        let mut ctx: SchedulerContext = consume_context();
        ctx.go_today();
    };

    rsx! {
        button {
            "type": "button",
            "aria-label": "Go to today",
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Title ───────────────────────────────────────────────────────────────────

/// Displays the current date range as a heading.
///
/// Renders the date based on the current view:
/// - Day: "April 4, 2026"
/// - Week: "Mar 30 – Apr 5, 2026"
/// - Agenda: "April 4, 2026"
#[component]
pub fn Title(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();
    let date = ctx.current_date();

    let title = match ctx.view() {
        SchedulerView::Day | SchedulerView::Agenda => {
            format!(
                "{} {}, {}",
                month_name(date.month()),
                date.day(),
                date.year()
            )
        }
        SchedulerView::Week => {
            let dates = week_dates(date);
            let start = dates[0];
            let end = dates[6];
            if start.month() == end.month() {
                format!(
                    "{} {} – {}, {}",
                    month_name(start.month()),
                    start.day(),
                    end.day(),
                    end.year()
                )
            } else {
                format!(
                    "{} {} – {} {}, {}",
                    month_name(start.month()),
                    start.day(),
                    month_name(end.month()),
                    end.day(),
                    end.year()
                )
            }
        }
    };

    rsx! {
        div {
            role: "heading",
            "aria-level": "2",
            "aria-live": "polite",
            ..attributes,
            if children == VNode::empty() {
                "{title}"
            } else {
                {children}
            }
        }
    }
}

// ── DayView ─────────────────────────────────────────────────────────────────

/// Single-day timeline view container.
///
/// Contains [`AllDayRow`] and [`TimeGrid`].
#[component]
pub fn DayView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();
    let date = ctx.current_date();
    let is_today = ctx.is_today(date);

    rsx! {
        div {
            "data-today": is_today.then_some("true"),
            "data-weekday": date.weekday().num_days_from_monday().to_string(),
            ..attributes,
            {children}
        }
    }
}

// ── WeekView ────────────────────────────────────────────────────────────────

/// 7-day side-by-side view container.
///
/// Renders column headers for each day of the week.
#[component]
pub fn WeekView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "grid",
            "aria-label": "Week view",
            ..attributes,
            {children}
        }
    }
}

// ── AgendaView ──────────────────────────────────────────────────────────────

/// Flat chronological event list.
#[component]
pub fn AgendaView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "list",
            "aria-label": "Agenda",
            ..attributes,
            {children}
        }
    }
}

// ── AgendaDay ───────────────────────────────────────────────────────────────

/// Day grouping header in agenda view.
#[component]
pub fn AgendaDay(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// The date this group represents.
    date: NaiveDate,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();
    let is_today = ctx.is_today(date);

    rsx! {
        div {
            role: "listitem",
            "data-today": is_today.then_some("true"),
            ..attributes,
            {children}
        }
    }
}

// ── AgendaEvent ─────────────────────────────────────────────────────────────

/// Event row in agenda view.
#[component]
pub fn AgendaEvent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Event identifier.
    event_id: String,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();
    let is_selected = ctx.selected_event().as_deref() == Some(event_id.as_str());

    let eid = event_id.clone();
    let onclick = move |_: MouseEvent| {
        let mut ctx: SchedulerContext = consume_context();
        ctx.select_event(&eid);
    };

    rsx! {
        div {
            role: "listitem",
            "data-selected": is_selected.then_some("true"),
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── AllDayRow ───────────────────────────────────────────────────────────────

/// Container for all-day events at the top of a day/week view.
#[component]
pub fn AllDayRow(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "row",
            "aria-label": "All day events",
            ..attributes,
            {children}
        }
    }
}

// ── TimeGrid ────────────────────────────────────────────────────────────────

/// Hour-slotted time grid for day/week views.
///
/// Contains [`TimeSlot`] and [`Event`] components.
#[component]
pub fn TimeGrid(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "grid",
            "aria-label": "Time grid",
            ..attributes,
            {children}
        }
    }
}

// ── TimeSlot ────────────────────────────────────────────────────────────────

/// Individual time slot in the grid.
///
/// ## Data attributes
/// - `data-slot-hour` — hour string (e.g., "14")
/// - `data-current-hour="true"` — if this is the current hour
/// - `data-past="true"` — if this slot is in the past
#[component]
pub fn TimeSlot(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Date of this slot.
    date: NaiveDate,
    /// Hour (0–23).
    hour: u32,
    /// Minute (0 or 30).
    #[props(default)]
    minute: u32,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();

    let now = chrono::Local::now().naive_local();
    let is_current = ctx.is_today(date) && now.time().hour() == hour;
    let is_past = date.and_hms_opt(hour, minute, 0).is_some_and(|dt| dt < now);

    let slot_data = TimeSlotData { date, hour, minute };
    let onclick = move |_: MouseEvent| {
        let ctx: SchedulerContext = consume_context();
        if let Some(handler) = &ctx.on_slot_click {
            handler.call(slot_data.clone());
        }
    };

    rsx! {
        div {
            role: "gridcell",
            "data-slot-hour": hour.to_string(),
            "data-current-hour": is_current.then_some("true"),
            "data-past": is_past.then_some("true"),
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Event ───────────────────────────────────────────────────────────────────

/// Positioned event block in the time grid.
///
/// ## Data attributes
/// - `data-selected="true"` — when selected
/// - `data-all-day="true"` — when all-day event
/// - `data-overlap-col` — overlap column index
/// - `data-overlap-total` — total overlap columns
#[component]
pub fn Event(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Event identifier.
    event_id: String,
    /// Whether this is an all-day event.
    #[props(default)]
    all_day: bool,
    /// Overlap column index (from layout computation).
    #[props(default)]
    overlap_col: Option<usize>,
    /// Total overlap columns (from layout computation).
    #[props(default)]
    overlap_total: Option<usize>,
    children: Element,
) -> Element {
    let ctx: SchedulerContext = use_context();
    let is_selected = ctx.selected_event().as_deref() == Some(event_id.as_str());

    let eid = event_id.clone();
    let onclick = move |_: MouseEvent| {
        let mut ctx: SchedulerContext = consume_context();
        ctx.select_event(&eid);
    };

    let onkeydown = {
        let eid = event_id.clone();
        move |event: KeyboardEvent| match event.key() {
            Key::Escape => {
                let mut ctx: SchedulerContext = consume_context();
                ctx.deselect_event();
            }
            Key::Enter => {
                let mut ctx: SchedulerContext = consume_context();
                ctx.select_event(&eid);
            }
            Key::Character(ref c) if c == " " => {
                let mut ctx: SchedulerContext = consume_context();
                ctx.select_event(&eid);
            }
            _ => {}
        }
    };

    rsx! {
        div {
            role: "button",
            tabindex: "0",
            "aria-label": event_id.clone(),
            "data-selected": is_selected.then_some("true"),
            "data-all-day": all_day.then_some("true"),
            "data-overlap-col": overlap_col.map(|c| c.to_string()),
            "data-overlap-total": overlap_total.map(|t| t.to_string()),
            onclick,
            onkeydown,
            ..attributes,
            {children}
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}
