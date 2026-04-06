use dioxus::prelude::*;
use dioxus_nox_calendar::{calendar, range_calendar, DateRange};
use dioxus_nox_date_picker::presets::PresetItem;
use dioxus_nox_date_picker::{date_field, date_picker, date_range_picker};
use time::macros::date;
use time::{Date, Weekday};

use crate::DemoSection;

#[component]
pub fn PickerDemos() -> Element {
    rsx! {
        DateFieldDemo {}
        DatePickerDemo {}
        RangePickerDemo {}
        ReadOnlyDemo {}
    }
}

// ── 11. Date Field ─────────────────────────────────────────────────

#[component]
fn DateFieldDemo() -> Element {
    let mut value = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "date-field",
            title: "Date Field (Standalone)",
            desc: "Segmented YYYY-MM-DD spinbutton input. Arrow keys to increment, Tab to advance segments.",
            date_field::Root {
                on_value_change: move |date: Option<Date>| value.set(date),
                date_field::Input {}
            }
            div { class: "output",
                {match (value)() {
                    Some(d) => format!("Value: {d}"),
                    None => "No complete date".to_string(),
                }}
            }
        }
    }
}

// ── 12. Date Picker ────────────────────────────────────────────────

#[component]
fn DatePickerDemo() -> Element {
    let mut value = use_signal(|| Option::<Date>::None);

    rsx! {
        DemoSection {
            id: "date-picker",
            title: "Date Picker (Popover)",
            desc: "Segmented input + trigger button opens a calendar popover. Auto-closes on selection.",
            div { class: "picker-container",
                date_picker::Root {
                    on_value_change: move |date: Option<Date>| value.set(date),
                    div { class: "row",
                        date_picker::Input {}
                        date_picker::Trigger { class: "picker-trigger", "Pick date" }
                    }
                    date_picker::Popover {
                        date_picker::Calendar {
                            calendar::Header {
                                calendar::PrevButton { "\u{2039}" }
                                calendar::Title {}
                                calendar::NextButton { "\u{203a}" }
                            }
                            calendar::Grid {}
                        }
                    }
                }
            }
            div { class: "output",
                {match (value)() {
                    Some(d) => format!("Picked: {d}"),
                    None => "No date picked".to_string(),
                }}
            }
        }
    }
}

// ── 13. Range Picker + Presets ─────────────────────────────────────

#[component]
fn RangePickerDemo() -> Element {
    let mut range = use_signal(|| Option::<DateRange>::None);

    let today = date!(2026-04-05);
    let presets = vec![
        PresetItem {
            label: "Last 7 days".to_string(),
            range: dioxus_nox_date_picker::last_n_days(7, today),
        },
        PresetItem {
            label: "Last 30 days".to_string(),
            range: dioxus_nox_date_picker::last_n_days(30, today),
        },
        PresetItem {
            label: "This week".to_string(),
            range: dioxus_nox_date_picker::this_week(today, Weekday::Monday),
        },
        PresetItem {
            label: "This month".to_string(),
            range: dioxus_nox_date_picker::this_month(today),
        },
        PresetItem {
            label: "Last month".to_string(),
            range: dioxus_nox_date_picker::last_month(today),
        },
    ];

    rsx! {
        DemoSection {
            id: "range-picker",
            title: "Date Range Picker + Presets",
            desc: "Popover with range calendar and a preset sidebar. Click a preset or pick manually.",
            div { class: "picker-container",
                date_range_picker::Root {
                    on_range_change: move |r: Option<DateRange>| range.set(r),
                    div { class: "row",
                        date_range_picker::InputStart {}
                        span { " \u{2013} " }
                        date_range_picker::InputEnd {}
                        date_range_picker::Trigger { class: "picker-trigger", "Pick range" }
                    }
                    date_range_picker::Popover {
                        div { class: "preset-sidebar",
                            date_range_picker::PresetList {
                                presets,
                                on_select: move |r: DateRange| range.set(Some(r)),
                                active_range: (range)(),
                            }
                            date_range_picker::Calendar {
                                range_calendar::Header {
                                    range_calendar::PrevButton { "\u{2039}" }
                                    range_calendar::Title {}
                                    range_calendar::NextButton { "\u{203a}" }
                                }
                                range_calendar::Grid {}
                            }
                        }
                    }
                }
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

// ── 14. Read-Only ──────────────────────────────────────────────────

#[component]
fn ReadOnlyDemo() -> Element {
    rsx! {
        DemoSection {
            id: "readonly",
            title: "Read-Only Calendar",
            desc: "Displays a pre-selected date but prevents any interaction.",
            calendar::Root {
                default_value: date!(2026-04-15),
                read_only: true,
                calendar::Header {
                    calendar::PrevButton { "\u{2039}" }
                    calendar::Title {}
                    calendar::NextButton { "\u{203a}" }
                }
                calendar::Grid {}
            }
        }
    }
}
