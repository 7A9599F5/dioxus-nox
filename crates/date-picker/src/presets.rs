//! Preset date range utilities and component.

use dioxus::prelude::*;
use dioxus_nox_calendar::DateRange;
use time::{Date, Weekday, ext::NumericalDuration};

/// Create a range covering the last `n` days ending at `today`.
pub fn last_n_days(n: u32, today: Date) -> DateRange {
    let start = today.saturating_sub((n as i64).days());
    DateRange::new(start, today)
}

/// Range for the current week (Monday to Sunday or as configured).
pub fn this_week(today: Date, first_day: Weekday) -> DateRange {
    let days_since_start = {
        let today_num = today.weekday().number_days_from_monday();
        let start_num = first_day.number_days_from_monday();
        ((today_num as i8 - start_num as i8 + 7) % 7) as i64
    };
    let start = today.saturating_sub(days_since_start.days());
    let end = start.saturating_add(6.days());
    DateRange::new(start, end)
}

/// Range for the current month.
pub fn this_month(today: Date) -> DateRange {
    let start = today.replace_day(1).expect("day 1 is always valid");
    let max = today.month().length(today.year());
    let end = today.replace_day(max).expect("valid");
    DateRange::new(start, end)
}

/// Range for the current year.
pub fn this_year(today: Date) -> DateRange {
    let start = Date::from_calendar_date(today.year(), time::Month::January, 1).unwrap();
    let end = Date::from_calendar_date(today.year(), time::Month::December, 31).unwrap();
    DateRange::new(start, end)
}

/// Range for the previous month.
pub fn last_month(today: Date) -> DateRange {
    let prev = today.month().previous();
    let year = today.year() + if prev == time::Month::December { -1 } else { 0 };
    let start = Date::from_calendar_date(year, prev, 1).unwrap();
    let max = prev.length(year);
    let end = Date::from_calendar_date(year, prev, max).unwrap();
    DateRange::new(start, end)
}

/// Range for the previous year.
pub fn last_year(today: Date) -> DateRange {
    let year = today.year() - 1;
    let start = Date::from_calendar_date(year, time::Month::January, 1).unwrap();
    let end = Date::from_calendar_date(year, time::Month::December, 31).unwrap();
    DateRange::new(start, end)
}

/// A preset item for the PresetList.
#[derive(Clone, PartialEq)]
pub struct PresetItem {
    pub label: String,
    pub range: DateRange,
}

/// Renders a listbox of preset date ranges.
///
/// ## Data attributes
/// - `data-selected` — on the preset matching the current selection
#[component]
pub fn PresetList(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// The available presets.
    presets: Vec<PresetItem>,
    /// Fires when a preset is selected.
    on_select: EventHandler<DateRange>,
    /// Currently active range (for `data-selected`).
    #[props(default)]
    active_range: Option<DateRange>,
) -> Element {
    rsx! {
        div {
            role: "listbox",
            ..attributes,
            for preset in presets {
                {
                    let range = preset.range;
                    let is_selected = active_range == Some(range);
                    rsx! {
                        button {
                            r#type: "button",
                            role: "option",
                            "data-selected": is_selected.then_some("true"),
                            aria_selected: if is_selected { "true" } else { "false" },
                            onclick: move |_| on_select.call(range),
                            {preset.label}
                        }
                    }
                }
            }
        }
    }
}
