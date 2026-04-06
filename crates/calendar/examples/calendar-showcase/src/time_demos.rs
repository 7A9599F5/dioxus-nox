use dioxus::prelude::*;
use dioxus_nox_calendar::calendar;
use dioxus_nox_date_picker::date_picker;
use dioxus_nox_time_picker::time_picker;
use time::{Date, Time};

use crate::DemoSection;

#[component]
pub fn TimeDemos() -> Element {
    rsx! {
        Time24Demo {}
        Time12Demo {}
        DateTimeDemo {}
    }
}

// ── 15. 24-Hour Time ───────────────────────────────────────────────

#[component]
fn Time24Demo() -> Element {
    let mut value = use_signal(|| Option::<Time>::None);

    rsx! {
        DemoSection {
            id: "time-24h",
            title: "24-Hour Time Picker",
            desc: "Hour (0-23) and minute spinbuttons. Arrow keys to increment, type digits directly.",
            time_picker::Root {
                on_change: move |t: Option<Time>| value.set(t),
                time_picker::Hour {}
                time_picker::Separator {}
                time_picker::Minute {}
            }
            div { class: "output",
                {match (value)() {
                    Some(t) => format!("Time: {t}"),
                    None => "No time set".to_string(),
                }}
            }
        }
    }
}

// ── 16. 12-Hour + AM/PM ───────────────────────────────────────────

#[component]
fn Time12Demo() -> Element {
    let mut value = use_signal(|| Option::<Time>::None);

    rsx! {
        DemoSection {
            id: "time-12h",
            title: "12-Hour Time + AM/PM",
            desc: "12-hour display with AM/PM toggle. Press 'a' or 'p' keys on the period button.",
            time_picker::Root {
                use_12_hour: true,
                show_seconds: true,
                on_change: move |t: Option<Time>| value.set(t),
                time_picker::Hour {}
                time_picker::Separator {}
                time_picker::Minute {}
                time_picker::Separator {}
                time_picker::Second {}
                time_picker::Period {}
            }
            div { class: "output",
                {match (value)() {
                    Some(t) => format!("Time (24h internal): {t}"),
                    None => "No time set".to_string(),
                }}
            }
        }
    }
}

// ── 17. Date + Time Combined ───────────────────────────────────────

#[component]
fn DateTimeDemo() -> Element {
    let mut date_val = use_signal(|| Option::<Date>::None);
    let mut time_val = use_signal(|| Option::<Time>::None);

    let datetime_display = use_memo(move || match ((date_val)(), (time_val)()) {
        (Some(d), Some(t)) => format!("{d}T{t}"),
        (Some(d), None) => format!("{d}T--:--"),
        (None, Some(t)) => format!("----/--/--T{t}"),
        (None, None) => "No datetime".to_string(),
    });

    rsx! {
        DemoSection {
            id: "datetime",
            title: "Date + Time Combined",
            desc: "A date picker and time picker composed together for full datetime selection.",
            div { class: "datetime-row",
                div { class: "picker-container",
                    date_picker::Root {
                        on_value_change: move |d: Option<Date>| date_val.set(d),
                        div { class: "row",
                            date_picker::Input {}
                            date_picker::Trigger { class: "picker-trigger", "Date" }
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

                time_picker::Root {
                    on_change: move |t: Option<Time>| time_val.set(t),
                    time_picker::Hour {}
                    time_picker::Separator {}
                    time_picker::Minute {}
                }
            }
            div { class: "output",
                {(datetime_display)()}
            }
        }
    }
}
