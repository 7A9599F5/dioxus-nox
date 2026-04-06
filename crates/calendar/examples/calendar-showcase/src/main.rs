mod calendar_demos;
mod css;
mod picker_demos;
mod time_demos;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        style { {css::CSS} }
        div { class: "page",
            nav { class: "sidebar",
                h2 { "Calendar" }
                a { href: "#basic", "Basic Single Select" }
                a { href: "#controlled", "Controlled Mode" }
                a { href: "#range", "Range Selection" }
                a { href: "#multi-month", "Multi-Month" }
                a { href: "#week-numbers", "Week Numbers" }
                a { href: "#disabled", "Disabled & Unavailable" }
                a { href: "#bounds", "Min / Max Bounds" }
                a { href: "#i18n", "Monday Start + i18n" }
                a { href: "#custom-cell", "Custom Cell Render" }
                a { href: "#views", "Year & Decade Views" }

                h2 { "Date Picker" }
                a { href: "#date-field", "Date Field" }
                a { href: "#date-picker", "Date Picker" }
                a { href: "#range-picker", "Range Picker + Presets" }
                a { href: "#readonly", "Read-Only" }

                h2 { "Time Picker" }
                a { href: "#time-24h", "24-Hour Time" }
                a { href: "#time-12h", "12-Hour + AM/PM" }
                a { href: "#datetime", "Date + Time Combined" }
            }

            main { class: "main",
                h1 { "Calendar Showcase" }
                p { "All features of dioxus-nox-calendar, date-picker, and time-picker." }

                calendar_demos::CalendarDemos {}
                picker_demos::PickerDemos {}
                time_demos::TimeDemos {}
            }
        }
    }
}

#[component]
pub fn DemoSection(id: String, title: String, desc: String, children: Element) -> Element {
    rsx! {
        section { class: "section", id: "{id}",
            h2 { {title} }
            p { class: "desc", {desc} }
            div { class: "demo",
                {children}
            }
        }
    }
}
