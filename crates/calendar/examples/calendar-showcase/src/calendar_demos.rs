use dioxus::prelude::*;
use dioxus_nox_calendar::{calendar, range_calendar, CellRenderData, DateRange};
use time::{Date, Month, Weekday};
use time::macros::date;

use crate::DemoSection;

#[component]
pub fn CalendarDemos() -> Element {
    rsx! {
        BasicDemo {}
        ControlledDemo {}
        RangeDemo {}
        MultiMonthDemo {}
        WeekNumbersDemo {}
        DisabledDemo {}
        BoundsDemo {}
        I18nDemo {}
        CustomCellDemo {}
        ViewsDemo {}
    }
}

// ── 1. Basic Single Select ─────────────────────────────────────────

#[component]
fn BasicDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "basic",
            title: "Basic Single Select",
            desc: "Click a date to select it. Click again to deselect.",
            calendar::Root {
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 2. Controlled Mode ─────────────────────────────────────────────

#[component]
fn ControlledDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::Some(date!(2026-04-05)));

    rsx! {
        DemoSection {
            id: "controlled",
            title: "Controlled Mode",
            desc: "External signal drives the selection. Use buttons to set today or clear.",
            calendar::Root {
                value: selected,
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
            div { class: "btn-row",
                button {
                    class: "btn",
                    onclick: move |_| selected.set(Some(date!(2026-04-05))),
                    "Set to Today"
                }
                button {
                    class: "btn",
                    onclick: move |_| selected.set(None),
                    "Clear"
                }
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 3. Range Selection ─────────────────────────────────────────────

#[component]
fn RangeDemo() -> Element {
    let mut range = use_signal(|| Option::<DateRange>::None);

    rsx! {
        DemoSection {
            id: "range",
            title: "Range Selection",
            desc: "Click once for start, again for end. Hover to preview the range.",
            range_calendar::Root {
                on_range_change: move |r: Option<DateRange>| range.set(r),
                range_calendar::Header {
                    range_calendar::PrevButton { "\u{2039}" }
                    range_calendar::Title {}
                    range_calendar::NextButton { "\u{203a}" }
                }
                range_calendar::Grid {}
            }
            div { class: "output",
                {match (range)() {
                    Some(r) => format!("Range: {} to {}", r.start(), r.end()),
                    None => "No range selected".to_string(),
                }}
            }
        }
    }
}

// ── 4. Multi-Month ─────────────────────────────────────────────────

#[component]
fn MultiMonthDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "multi-month",
            title: "Multi-Month (2-up)",
            desc: "Two months displayed side by side. Prev/Next buttons auto-hide on inner panes.",
            calendar::Root {
                month_count: 2,
                on_value_change: move |date: Option<Date>| selected.set(date),
                div { class: "multi-month",
                    calendar::MonthView { offset: 0,
                        calendar::Header {
                            calendar::PrevButton { "\u{2039}" }
                            calendar::Title {}
                            calendar::NextButton { "\u{203a}" }
                        }
                        calendar::Grid {}
                    }
                    calendar::MonthView { offset: 1,
                        calendar::Header {
                            calendar::PrevButton { "\u{2039}" }
                            calendar::Title {}
                            calendar::NextButton { "\u{203a}" }
                        }
                        calendar::Grid {}
                    }
                }
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 5. Week Numbers ────────────────────────────────────────────────

#[component]
fn WeekNumbersDemo() -> Element {
    rsx! {
        DemoSection {
            id: "week-numbers",
            title: "Week Numbers",
            desc: "ISO week numbers displayed in the first column.",
            calendar::Root {
                first_day_of_week: Weekday::Monday,
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid { show_week_numbers: true }
            }
        }
    }
}

// ── 6. Disabled & Unavailable ──────────────────────────────────────

#[component]
fn DisabledDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    // Weekends are disabled (non-interactive)
    let is_disabled = Callback::new(|date: Date| {
        matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday)
    });

    // Specific dates are unavailable (marked but focusable)
    let holidays = [date!(2026-04-10), date!(2026-04-17), date!(2026-05-01)];
    let is_unavailable = Callback::new(move |date: Date| holidays.contains(&date));

    rsx! {
        DemoSection {
            id: "disabled",
            title: "Disabled & Unavailable Dates",
            desc: "Weekends are disabled (grayed out). Specific holidays are unavailable (strikethrough, still focusable).",
            calendar::Root {
                is_date_disabled: is_disabled,
                is_date_unavailable: is_unavailable,
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 7. Min / Max Bounds ────────────────────────────────────────────

#[component]
fn BoundsDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "bounds",
            title: "Min / Max Date Bounds",
            desc: "Selectable range constrained to April 1 – May 31, 2026.",
            calendar::Root {
                min_date: date!(2026-04-01),
                max_date: date!(2026-05-31),
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 8. Monday Start + i18n ─────────────────────────────────────────

#[component]
fn I18nDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    let format_weekday = Callback::new(|day: Weekday| {
        match day {
            Weekday::Monday => "Lu",
            Weekday::Tuesday => "Ma",
            Weekday::Wednesday => "Me",
            Weekday::Thursday => "Je",
            Weekday::Friday => "Ve",
            Weekday::Saturday => "Sa",
            Weekday::Sunday => "Di",
        }
        .to_string()
    });

    let format_month = Callback::new(|month: Month| {
        match month {
            Month::January => "Janvier",
            Month::February => "F\u{00e9}vrier",
            Month::March => "Mars",
            Month::April => "Avril",
            Month::May => "Mai",
            Month::June => "Juin",
            Month::July => "Juillet",
            Month::August => "Ao\u{00fb}t",
            Month::September => "Septembre",
            Month::October => "Octobre",
            Month::November => "Novembre",
            Month::December => "D\u{00e9}cembre",
        }
        .to_string()
    });

    rsx! {
        DemoSection {
            id: "i18n",
            title: "Monday Start + French Labels",
            desc: "Week starts on Monday. Weekday and month labels in French via formatter callbacks.",
            calendar::Root {
                first_day_of_week: Weekday::Monday,
                format_weekday,
                format_month,
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("S\u{00e9}lectionn\u{00e9}: {d}"),
                    None => "Aucune date s\u{00e9}lectionn\u{00e9}e".to_string(),
                }}
            }
        }
    }
}

// ── 9. Custom Cell Render ──────────────────────────────────────────

#[component]
fn CustomCellDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    // Dates with "events" (dot indicators)
    let event_dates = [
        date!(2026-04-07),
        date!(2026-04-12),
        date!(2026-04-15),
        date!(2026-04-22),
        date!(2026-04-28),
    ];

    let render_cell = Callback::new(move |cell: CellRenderData| {
        let has_event = event_dates.contains(&cell.date);
        let dot_color = if cell.is_selected { "#fff" } else { "#3b82f6" };
        let dot_style = format!(
            "display:block;width:4px;height:4px;border-radius:50%;background:{dot_color};margin:1px auto 0;"
        );
        rsx! {
            span {
                {cell.day.to_string()}
                if has_event {
                    span { style: "{dot_style}" }
                }
            }
        }
    });

    rsx! {
        DemoSection {
            id: "custom-cell",
            title: "Custom Cell Render",
            desc: "Dot indicators on dates with events via the render_cell callback.",
            calendar::Root {
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid { render_cell }
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}

// ── 10. Year & Decade Views ────────────────────────────────────────

#[component]
fn ViewsDemo() -> Element {
    let mut selected = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "views",
            title: "Year & Decade Views + Dropdowns",
            desc: "Click the title to drill: Month \u{2192} Year \u{2192} Decade. Use dropdowns for quick navigation.",
            calendar::Root {
                on_value_change: move |date: Option<Date>| selected.set(date),
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    div { class: "row",
                        calendar::SelectMonth {}
                        calendar::SelectYear {}
                    }
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Title {}
                calendar::Grid {}
                calendar::YearView {}
                calendar::DecadeView {}
            }
            div { class: "output",
                {match (selected)() {
                    Some(d) => format!("Selected: {d}"),
                    None => "No date selected".to_string(),
                }}
            }
        }
    }
}
