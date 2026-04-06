use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use time::{Date, Month, Weekday};

use crate::context::*;
use crate::math;
use crate::types::{CellRenderData, DateRange, DateStatus, ViewMode};

// ── Root (single-select calendar) ───────────────────────────────────

/// Context provider for a single-date calendar.
///
/// Ships **zero visual styles** — all state is expressed through `data-*` attributes.
///
/// ```text
/// calendar::Root {
///     calendar::Header {
///         calendar::PrevButton { "<" }
///         calendar::Title {}
///         calendar::NextButton { ">" }
///     }
///     calendar::Grid {}
/// }
/// ```
///
/// ## Data attributes
/// - `data-disabled` — present when the entire calendar is disabled
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial selected date (uncontrolled mode).
    #[props(default)]
    default_value: Option<Date>,
    /// Controlled selected date signal.
    #[props(default)]
    value: Option<Signal<Option<Date>>>,
    /// Fires when the selected date changes.
    #[props(default)]
    on_value_change: Option<EventHandler<Option<Date>>>,
    /// Initial view date. Defaults to `default_value`, then today.
    #[props(default)]
    default_view_date: Option<Date>,
    /// Controlled view date signal.
    #[props(default)]
    view_date: Option<Signal<Date>>,
    /// Fires when the displayed month changes.
    #[props(default)]
    on_view_change: Option<EventHandler<Date>>,
    /// Today's date. Defaults to current date.
    #[props(default)]
    today: Option<Date>,
    /// Disable the entire calendar.
    #[props(default)]
    disabled: bool,
    /// First day of the week.
    #[props(default = Weekday::Sunday)]
    first_day_of_week: Weekday,
    /// Earliest selectable date.
    #[props(default = date_min())]
    min_date: Date,
    /// Latest selectable date.
    #[props(default = date_max())]
    max_date: Date,
    /// Number of months visible at once.
    #[props(default = 1)]
    month_count: u8,
    /// Per-date disabled callback.
    #[props(default)]
    is_date_disabled: Option<Callback<Date, bool>>,
    /// Per-date unavailable callback.
    #[props(default)]
    is_date_unavailable: Option<Callback<Date, bool>>,
    /// Display selections but prevent interaction.
    #[props(default)]
    read_only: bool,
    /// Format a weekday for display (grid headers). Default: "Mo", "Tu", ...
    #[props(default)]
    format_weekday: Option<Callback<Weekday, String>>,
    /// Format a month for display (title, select). Default: "January", "February", ...
    #[props(default)]
    format_month: Option<Callback<Month, String>>,
    /// Format a date for aria-label. Default: "Friday, April 4, 2026"
    #[props(default)]
    format_date_label: Option<Callback<Date, String>>,
    children: Element,
) -> Element {
    let today_val = today.unwrap_or_else(today_date);
    let initial_view = default_view_date
        .or(default_value)
        .unwrap_or(today_val);

    let internal_view = use_signal(|| initial_view);
    let internal_selected = use_signal(|| default_value);
    let instance_id = use_hook(next_instance_id);

    // Cache disabled/unavailable status for visible dates
    let is_disabled_cb = is_date_disabled;
    let is_unavailable_cb = is_date_unavailable;
    let view_sig = view_date.unwrap_or(internal_view);
    let date_status_cache = use_memo(move || {
        let view = (view_sig)();
        let grid = math::month_grid(view.year(), view.month(), first_day_of_week);
        let mut cache = HashMap::with_capacity(grid.len());
        for date in &grid {
            let d = DateStatus {
                disabled: is_disabled_cb.as_ref().is_some_and(|cb| cb.call(*date)),
                unavailable: is_unavailable_cb.as_ref().is_some_and(|cb| cb.call(*date)),
            };
            if d.disabled || d.unavailable {
                cache.insert(*date, d);
            }
        }
        cache
    });

    let base = BaseCalendarContext {
        view_date: internal_view,
        controlled_view: view_date,
        disabled,
        first_day_of_week,
        min_date,
        max_date,
        month_count,
        today: today_val,
        instance_id,
        date_status_cache,
        on_view_change,
        read_only,
        format_weekday,
        format_month,
        format_date_label,
        view_mode: use_signal(|| ViewMode::Month),
    };

    let focus = CalendarFocusContext {
        focused_date: use_signal(|| None),
    };

    let selection = SelectionContext::Single(SingleContext {
        selected_date: internal_selected,
        controlled_selected: value,
        on_value_change,
    });

    use_context_provider(|| base);
    use_context_provider(|| focus);
    use_context_provider(|| selection);

    rsx! {
        div {
            role: "application",
            aria_label: "Calendar",
            "data-disabled": disabled.then_some("true"),
            "data-readonly": read_only.then_some("true"),
            "data-view-mode": base.view_mode().as_data_attr(),
            onkeydown: move |e| handle_keyboard(e, base.enabled_range()),
            ..attributes,
            {children}
        }
    }
}

// ── RangeRoot (range-select calendar) ───────────────────────────────

/// Context provider for a range-select calendar.
///
/// Same compound parts as `Root`, but provides range selection context.
///
/// ## Data attributes
/// - `data-disabled` — present when the entire calendar is disabled
#[component]
pub fn RangeRoot(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial selected range (uncontrolled mode).
    #[props(default)]
    default_value: Option<DateRange>,
    /// Controlled selected range signal.
    #[props(default)]
    value: Option<Signal<Option<DateRange>>>,
    /// Fires when the selected range changes.
    #[props(default)]
    on_range_change: Option<EventHandler<Option<DateRange>>>,
    /// Initial view date.
    #[props(default)]
    default_view_date: Option<Date>,
    /// Controlled view date signal.
    #[props(default)]
    view_date: Option<Signal<Date>>,
    /// Fires when the displayed month changes.
    #[props(default)]
    on_view_change: Option<EventHandler<Date>>,
    /// Today's date.
    #[props(default)]
    today: Option<Date>,
    /// Disable the entire calendar.
    #[props(default)]
    disabled: bool,
    /// First day of the week.
    #[props(default = Weekday::Sunday)]
    first_day_of_week: Weekday,
    /// Earliest selectable date.
    #[props(default = date_min())]
    min_date: Date,
    /// Latest selectable date.
    #[props(default = date_max())]
    max_date: Date,
    /// Number of months visible at once.
    #[props(default = 1)]
    month_count: u8,
    /// Per-date disabled callback.
    #[props(default)]
    is_date_disabled: Option<Callback<Date, bool>>,
    /// Per-date unavailable callback.
    #[props(default)]
    is_date_unavailable: Option<Callback<Date, bool>>,
    /// Display selections but prevent interaction.
    #[props(default)]
    read_only: bool,
    /// Format a weekday for display (grid headers). Default: "Mo", "Tu", ...
    #[props(default)]
    format_weekday: Option<Callback<Weekday, String>>,
    /// Format a month for display (title, select). Default: "January", "February", ...
    #[props(default)]
    format_month: Option<Callback<Month, String>>,
    /// Format a date for aria-label. Default: "Friday, April 4, 2026"
    #[props(default)]
    format_date_label: Option<Callback<Date, String>>,
    children: Element,
) -> Element {
    let today_val = today.unwrap_or_else(today_date);
    let initial_view = default_view_date
        .or_else(|| default_value.map(|r| r.start()))
        .unwrap_or(today_val);

    let internal_view = use_signal(|| initial_view);
    let internal_range = use_signal(|| default_value);
    let instance_id = use_hook(next_instance_id);

    let is_disabled_cb = is_date_disabled;
    let is_unavailable_cb = is_date_unavailable;
    let view_sig = view_date.unwrap_or(internal_view);
    let date_status_cache = use_memo(move || {
        let view = (view_sig)();
        let grid = math::month_grid(view.year(), view.month(), first_day_of_week);
        let mut cache = HashMap::with_capacity(grid.len());
        for date in &grid {
            let d = DateStatus {
                disabled: is_disabled_cb.as_ref().is_some_and(|cb| cb.call(*date)),
                unavailable: is_unavailable_cb.as_ref().is_some_and(|cb| cb.call(*date)),
            };
            if d.disabled || d.unavailable {
                cache.insert(*date, d);
            }
        }
        cache
    });

    let base = BaseCalendarContext {
        view_date: internal_view,
        controlled_view: view_date,
        disabled,
        first_day_of_week,
        min_date,
        max_date,
        month_count,
        today: today_val,
        instance_id,
        date_status_cache,
        on_view_change,
        read_only,
        format_weekday,
        format_month,
        format_date_label,
        view_mode: use_signal(|| ViewMode::Month),
    };

    let focus = CalendarFocusContext {
        focused_date: use_signal(|| default_value.map(|r| r.end())),
    };

    let selection = SelectionContext::Range(RangeContext {
        anchor_date: use_signal(|| None),
        highlighted_range: use_signal(|| default_value),
        selected_range: internal_range,
        controlled_range: value,
        on_range_change,
    });

    use_context_provider(|| base);
    use_context_provider(|| focus);
    use_context_provider(|| selection);

    rsx! {
        div {
            role: "application",
            aria_label: "Calendar",
            "data-disabled": disabled.then_some("true"),
            "data-readonly": read_only.then_some("true"),
            "data-view-mode": base.view_mode().as_data_attr(),
            onkeydown: move |e| handle_keyboard(e, base.enabled_range()),
            ..attributes,
            {children}
        }
    }
}

// ── MonthView (multi-month wrapper) ─────────────────────────────────

/// Wrapper for multi-month layouts. Provides a `MonthViewContext`
/// with an offset from the base view date.
///
/// ```text
/// calendar::Root { month_count: 2,
///     calendar::MonthView { offset: 0,
///         calendar::Header { calendar::PrevButton { "<" } calendar::Title {} calendar::NextButton { ">" } }
///         calendar::Grid {}
///     }
///     calendar::MonthView { offset: 1,
///         calendar::Header { calendar::Title {} }
///         calendar::Grid {}
///     }
/// }
/// ```
///
/// When `MonthView` is not used, all components default to offset 0.
#[component]
pub fn MonthView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Month offset from the base view date (0 = current month, 1 = next, etc.)
    #[props(default = 0)]
    offset: u8,
    children: Element,
) -> Element {
    use_context_provider(|| MonthViewContext { offset });

    rsx! {
        div {
            ..attributes,
            {children}
        }
    }
}

// ── Header ──────────────────────────────────────────────────────────

/// Container for navigation controls (prev/next buttons, title).
#[component]
pub fn Header(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "group",
            ..attributes,
            {children}
        }
    }
}

// ── PrevButton ──────────────────────────────────────────────────────

/// Navigate to the previous month.
///
/// Automatically disabled when at `min_date` boundary.
#[component]
pub fn PrevButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: BaseCalendarContext = use_context();

    // In multi-month, only show on the first pane
    if current_offset() != 0 {
        return rsx! {};
    }

    let is_disabled = ctx.is_prev_disabled();

    let onclick = move |e: MouseEvent| {
        e.prevent_default();
        let mut ctx: BaseCalendarContext = consume_context();
        ctx.go_prev_month();
    };

    rsx! {
        button {
            r#type: "button",
            aria_label: "Previous month",
            disabled: is_disabled,
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── NextButton ──────────────────────────────────────────────────────

/// Navigate to the next month.
///
/// Automatically disabled when at `max_date` boundary.
#[component]
pub fn NextButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: BaseCalendarContext = use_context();

    // In multi-month, only show on the last pane
    if current_offset() + 1 != ctx.month_count {
        return rsx! {};
    }

    let is_disabled = ctx.is_next_disabled();

    let onclick = move |e: MouseEvent| {
        e.prevent_default();
        let mut ctx: BaseCalendarContext = consume_context();
        ctx.go_next_month();
    };

    rsx! {
        button {
            r#type: "button",
            aria_label: "Next month",
            disabled: is_disabled,
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Title ───────────────────────────────────────────────────────────

/// Displays the current month/year/decade label.
///
/// Click behavior (when not in a multi-month layout):
/// - In `Month` mode → switches to `Year` mode
/// - In `Year` mode → switches to `Decade` mode
/// - In `Decade` mode → no-op
#[component]
pub fn Title(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view = effective_view(&ctx);
    let offset = current_offset();
    let heading_id = ctx.element_id(&format!("heading-{offset}"));
    let mode = ctx.view_mode();

    let label = match mode {
        ViewMode::Month => format!("{} {}", ctx.month_label(view.month()), view.year()),
        ViewMode::Year => format!("{}", view.year()),
        ViewMode::Decade => {
            let (start, end) = math::decade_range(view.year());
            format!("{start} – {end}")
        }
    };

    let onclick = move |_: MouseEvent| {
        let ctx: BaseCalendarContext = consume_context();
        match ctx.view_mode() {
            ViewMode::Month => ctx.set_view_mode(ViewMode::Year),
            ViewMode::Year => ctx.set_view_mode(ViewMode::Decade),
            ViewMode::Decade => {}
        }
    };

    rsx! {
        div {
            id: heading_id,
            role: "heading",
            aria_level: "2",
            aria_live: "polite",
            onclick,
            ..attributes,
            {label}
        }
    }
}

// ── SelectMonth ─────────────────────────────────────────────────────

/// Month dropdown for quick month navigation.
///
/// Renders a `<select>` element with months within the enabled date range.
#[component]
pub fn SelectMonth(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view = effective_view(&ctx);
    let current_month = view.month();

    let offset = current_offset();
    let months = use_memo(move || {
        let base = ctx.current_view();
        let v = math::nth_month_next(base, offset).unwrap_or(base);
        let year = v.year();
        let min_month = if year == ctx.min_date.year() {
            ctx.min_date.month() as u8
        } else {
            1
        };
        let max_month = if year == ctx.max_date.year() {
            ctx.max_date.month() as u8
        } else {
            12
        };
        (min_month..=max_month)
            .map(|m| Month::try_from(m).unwrap())
            .collect::<Vec<_>>()
    });

    let onchange = move |e: Event<FormData>| {
        let Ok(num) = e.value().parse::<u8>() else {
            return;
        };
        let Ok(month) = Month::try_from(num) else {
            return;
        };
        let mut ctx: BaseCalendarContext = consume_context();
        let new_view = math::replace_month(ctx.current_view(), month);
        ctx.set_view(new_view);
    };

    rsx! {
        select {
            aria_label: "Month",
            disabled: ctx.disabled,
            onchange,
            ..attributes,
            for month in months() {
                option {
                    key: "{month}",
                    value: "{month as u8}",
                    selected: month == current_month,
                    {ctx.month_label(month)}
                }
            }
        }
    }
}

// ── SelectYear ──────────────────────────────────────────────────────

/// Year dropdown for quick year navigation.
///
/// Renders a `<select>` element with years within the enabled date range.
#[component]
pub fn SelectYear(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view = effective_view(&ctx);
    let current_year = view.year();

    let years = use_memo(move || {
        let min_year = ctx.min_date.year();
        let max_year = ctx.max_date.year();
        (min_year..=max_year).collect::<Vec<_>>()
    });

    let onchange = move |e: Event<FormData>| {
        let Ok(year) = e.value().parse::<i32>() else {
            return;
        };
        let mut ctx: BaseCalendarContext = consume_context();
        let current = ctx.current_view();
        let max_day = math::days_in_month(current.month(), year);
        let new_view =
            Date::from_calendar_date(year, current.month(), current.day().min(max_day))
                .unwrap_or(current);
        ctx.set_view(new_view);
    };

    rsx! {
        select {
            aria_label: "Year",
            disabled: ctx.disabled,
            onchange,
            ..attributes,
            for year in years() {
                option {
                    key: "{year}",
                    value: "{year}",
                    selected: year == current_year,
                    "{year}"
                }
            }
        }
    }
}

// ── Grid ────────────────────────────────────────────────────────────

/// Renders the calendar grid: weekday headers + date cells.
///
/// Generates a `<table>` with `role="grid"` containing the month view.
///
/// ## Data attributes on cells
/// - `data-today` — present on today's date
/// - `data-selected` — present on selected date(s)
/// - `data-disabled` — present on disabled dates
/// - `data-unavailable` — present on unavailable dates
/// - `data-outside-month` — present on dates outside the displayed month
/// - `data-focused` — present on keyboard-focused date
/// - `data-range-position="start|middle|end"` — range mode only
#[component]
pub fn Grid(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Show ISO week numbers as the first column.
    #[props(default)]
    show_week_numbers: bool,
    /// Custom render function for cell content. Receives computed state.
    /// Only replaces button children — attributes/ARIA/events are framework-managed.
    #[props(default)]
    render_cell: Option<Callback<CellRenderData, Element>>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();

    let offset = current_offset();
    let grid = use_memo(move || {
        let base = ctx.current_view();
        let v = math::nth_month_next(base, offset).unwrap_or(base);
        math::month_grid(v.year(), v.month(), ctx.first_day_of_week)
    });

    let headers = use_memo(move || math::weekday_headers(ctx.first_day_of_week));

    let grid_id = ctx.element_id(&format!("grid-{offset}"));
    let heading_id = ctx.element_id(&format!("heading-{offset}"));

    // Provide render_cell to Cell via context (avoids prop drilling through table/tr)
    use_context_provider(|| GridOptionsContext { render_cell });

    rsx! {
        table {
            id: grid_id,
            role: "grid",
            aria_labelledby: heading_id,
            ..attributes,

            thead { aria_hidden: "true",
                tr {
                    if show_week_numbers {
                        th { scope: "col", "W" }
                    }
                    for weekday in headers() {
                        th {
                            key: "{weekday:?}",
                            scope: "col",
                            abbr: "{weekday}",
                            {ctx.weekday_label(weekday)}
                        }
                    }
                }
            }

            tbody {
                for row in grid_rows(&grid.read()) {
                    tr { role: "row",
                        if show_week_numbers {
                            td {
                                aria_hidden: "true",
                                "data-week-number": "{math::iso_week_number(row[0])}",
                                {math::iso_week_number(row[0]).to_string()}
                            }
                        }
                        for date in row {
                            Cell { key: "{date}", date: date }
                        }
                    }
                }
            }
        }
    }
}

/// Split a flat grid into rows of 7.
fn grid_rows(grid: &[Date]) -> Vec<Vec<Date>> {
    grid.chunks(7).map(|c| c.to_vec()).collect()
}

// ── Cell ────────────────────────────────────────────────────────────

/// A single date cell in the calendar grid.
///
/// Reads all three contexts (base, focus, selection) and renders
/// appropriate data attributes for consumer styling.
#[component]
fn Cell(date: Date) -> Element {
    let base: BaseCalendarContext = use_context();
    let focus: CalendarFocusContext = use_context();
    let selection: SelectionContext = use_context();
    let grid_opts: GridOptionsContext = use_context();

    let view = base.current_view();
    let is_today = date == base.today;
    let is_disabled = base.is_date_disabled(date);
    let is_unavailable = base.is_date_unavailable(date);
    let is_focused = focus.is_focused(date);
    let rel_month = math::relative_month(date, view.month(), base.enabled_range());
    let is_outside = !rel_month.is_current();

    let (is_selected, range_pos) = match selection {
        SelectionContext::Single(ctx) => (ctx.is_selected(date), None),
        SelectionContext::Range(ctx) => {
            let pos = ctx.range_position(date);
            (pos.is_some(), pos)
        }
    };

    let cell_id = base.cell_id(date);
    let aria_label = base.date_aria_label(&date);

    // Determine tabindex: focusable date gets 0, others get -1
    let focusable_date = focus.focused().unwrap_or(view);
    let tabindex = if date == focusable_date { "0" } else { "-1" };

    let is_read_only = base.is_read_only();

    let onclick = move |e: MouseEvent| {
        e.prevent_default();
        if is_disabled || is_outside || is_read_only {
            return;
        }
        let mut focus: CalendarFocusContext = consume_context();
        focus.set_focused(Some(date));

        let mut selection: SelectionContext = consume_context();
        match &mut selection {
            SelectionContext::Single(ctx) => ctx.select(date),
            SelectionContext::Range(ctx) => ctx.click_date(date),
        }
    };

    let onfocus = move |_: FocusEvent| {
        if !is_disabled && !is_outside {
            let mut focus: CalendarFocusContext = consume_context();
            focus.set_focused(Some(date));
        }
    };

    let onmouseenter = move |_: MouseEvent| {
        if !is_disabled && !is_outside {
            let mut selection: SelectionContext = consume_context();
            if let SelectionContext::Range(ctx) = &mut selection {
                ctx.hover_date(date);
            }
        }
    };

    // Focus management via onmounted
    let mut mounted_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    use_effect(move || {
        let is_focused = focus.is_focused(date);
        if is_focused
            && let Some(el) = (mounted_ref)()
        {
            spawn(async move {
                _ = el.set_focus(true).await;
            });
        }
    });

    // Build cell content: custom render or default day number
    let cell_content = match grid_opts.render_cell {
        Some(render_cb) => render_cb.call(CellRenderData {
            date,
            day: date.day(),
            is_today,
            is_selected,
            is_disabled,
            is_unavailable,
            is_focused,
            is_outside_month: is_outside,
            relative_month: rel_month,
            range_position: range_pos,
        }),
        None => rsx! { {date.day().to_string()} },
    };

    rsx! {
        td { role: "gridcell",
            button {
                id: cell_id,
                r#type: "button",
                role: "gridcell",
                tabindex: tabindex,
                aria_label: aria_label,
                aria_selected: if is_selected { "true" } else { "false" },
                aria_disabled: if is_disabled { "true" } else { "false" },
                disabled: is_disabled,
                "data-date": "{date}",
                "data-today": is_today.then_some("true"),
                "data-selected": is_selected.then_some("true"),
                "data-disabled": is_disabled.then_some("true"),
                "data-unavailable": is_unavailable.then_some("true"),
                "data-outside-month": is_outside.then_some("true"),
                "data-focused": is_focused.then_some("true"),
                "data-month": rel_month.as_data_attr(),
                "data-range-position": range_pos,
                "data-readonly": is_read_only.then_some("true"),
                aria_readonly: is_read_only.then_some("true"),
                onclick,
                onfocus,
                onmouseenter,
                onmounted: move |e| mounted_ref.set(Some(e.data())),
                {cell_content}
            }
        }
    }
}

// ── Keyboard handler ────────────────────────────────────────────────

fn handle_keyboard(event: KeyboardEvent, enabled_range: DateRange) {
    let mut focus: CalendarFocusContext = consume_context();
    let mut base: BaseCalendarContext = consume_context();

    let Some(focused) = focus.focused() else {
        return;
    };

    // Enter/Space: select the focused date (unless read-only or disabled)
    let is_enter_or_space = event.key() == Key::Enter
        || matches!(event.key(), Key::Character(ref c) if c == " ");
    if is_enter_or_space {
        event.prevent_default();
        if !base.is_read_only() && !base.is_date_disabled(focused) {
            let mut selection: SelectionContext = consume_context();
            match &mut selection {
                SelectionContext::Single(ctx) => ctx.select(focused),
                SelectionContext::Range(ctx) => ctx.click_date(focused),
            }
        }
        return;
    }

    let nav_key = match event.key() {
        Key::ArrowLeft => Some(math::NavigationKey::Left),
        Key::ArrowRight => Some(math::NavigationKey::Right),
        Key::ArrowUp if event.modifiers().shift() => Some(math::NavigationKey::ShiftUp),
        Key::ArrowDown if event.modifiers().shift() => Some(math::NavigationKey::ShiftDown),
        Key::ArrowUp => Some(math::NavigationKey::Up),
        Key::ArrowDown => Some(math::NavigationKey::Down),
        Key::Home => Some(math::NavigationKey::Home),
        Key::End => Some(math::NavigationKey::End),
        Key::Escape => {
            if !base.is_read_only() {
                let mut selection: SelectionContext = consume_context();
                if let SelectionContext::Range(ctx) = &mut selection {
                    ctx.reset();
                }
            }
            return;
        }
        _ => None,
    };

    if let Some(key) = nav_key {
        event.prevent_default();

        // During range selection, clamp navigation to the contiguous non-disabled
        // zone around the anchor so the user can't create invalid ranges.
        let effective_range = {
            let selection: SelectionContext = consume_context();
            if let SelectionContext::Range(ctx) = &selection
                && (ctx.anchor_date)().is_some()
            {
                let anchor = (ctx.anchor_date)().unwrap();
                math::contiguous_range(anchor, enabled_range, |d| base.is_date_disabled(d))
            } else {
                enabled_range
            }
        };

        if let Some(new_date) = math::navigate_with(focused, key, effective_range, |d| {
            base.is_date_disabled(d)
        }) {
            focus.set_focused(Some(new_date));

            // If navigated outside current view, update view month
            let view = base.current_view();
            if new_date.month() != view.month() || new_date.year() != view.year() {
                let new_view = math::first_of_month(new_date);
                base.set_view(new_view);
            }

            // Update hover preview in range mode
            let mut selection: SelectionContext = consume_context();
            if let SelectionContext::Range(ctx) = &mut selection {
                ctx.hover_date(new_date);
            }
        }
    }
}

// ── View date resolution ────────────────────────────────────────────

/// Resolve the effective view date: base view + MonthView offset (if any).
fn effective_view(base: &BaseCalendarContext) -> Date {
    match try_use_context::<MonthViewContext>() {
        Some(mv) => mv.view_date(base),
        None => base.current_view(),
    }
}

/// Get the MonthView offset (0 if not in a MonthView).
fn current_offset() -> u8 {
    try_use_context::<MonthViewContext>().map_or(0, |mv| mv.offset)
}

// ── YearView (4x3 grid of months) ──────────────────────────────────

/// 4×3 grid of months for the current year. Click a month to navigate
/// to it and switch back to `ViewMode::Month`.
///
/// ## Data attributes
/// - `data-month-cell` — on each month button
/// - `data-selected` — on the month matching the current view
#[component]
pub fn YearView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view = ctx.current_view();
    let months = math::year_grid();

    rsx! {
        div {
            role: "grid",
            aria_label: "Year view",
            ..attributes,
            for row in months.chunks(4) {
                div { role: "row",
                    for &month in row {
                        button {
                            r#type: "button",
                            role: "gridcell",
                            "data-month-cell": "true",
                            "data-selected": (month == view.month()).then_some("true"),
                            onclick: move |_| {
                                let mut ctx: BaseCalendarContext = consume_context();
                                let new_view = math::replace_month(ctx.current_view(), month);
                                ctx.set_view(new_view);
                                ctx.set_view_mode(ViewMode::Month);
                            },
                            {ctx.month_label(month)}
                        }
                    }
                }
            }
        }
    }
}

// ── DecadeView (4x3 grid of years) ────────────────────────────────

/// 4×3 grid of years for the current decade. Click a year to navigate
/// to it and switch to `ViewMode::Year`.
///
/// ## Data attributes
/// - `data-year-cell` — on each year button
/// - `data-selected` — on the year matching the current view
#[component]
pub fn DecadeView(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view = ctx.current_view();
    let years = math::decade_grid(view.year());

    rsx! {
        div {
            role: "grid",
            aria_label: "Decade view",
            ..attributes,
            for row in years.chunks(4) {
                div { role: "row",
                    for &year in row {
                        button {
                            r#type: "button",
                            role: "gridcell",
                            "data-year-cell": "true",
                            "data-selected": (year == view.year()).then_some("true"),
                            onclick: move |_| {
                                let mut ctx: BaseCalendarContext = consume_context();
                                let current = ctx.current_view();
                                let max_day = math::days_in_month(current.month(), year);
                                if let Ok(new_view) = Date::from_calendar_date(
                                    year, current.month(), current.day().min(max_day)
                                ) {
                                    ctx.set_view(new_view);
                                }
                                ctx.set_view_mode(ViewMode::Year);
                            },
                            "{year}"
                        }
                    }
                }
            }
        }
    }
}

// ── Utility ─────────────────────────────────────────────────────────

fn today_date() -> Date {
    // Use UTC — calendar is timezone-agnostic
    time::OffsetDateTime::now_utc().date()
}

fn date_min() -> Date {
    Date::from_calendar_date(1925, Month::January, 1).unwrap()
}

fn date_max() -> Date {
    Date::from_calendar_date(2050, Month::December, 31).unwrap()
}
