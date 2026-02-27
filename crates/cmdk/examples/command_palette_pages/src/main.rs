use dioxus::prelude::*;
use dioxus_nox_cmdk::*;

const STYLE: Asset = asset!("/assets/style.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let palette = use_command_palette(true);
    let mut selected = use_signal(|| "None".to_string());

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }

        div { class: "page",
            h1 { "dioxus-cmdk Pages Example" }
            p {
                "Press ", kbd { "Cmd+K" },
                " to open. Select a drill-in command to navigate pages."
            }
            p { "Backspace on empty search goes back." }

            button {
                class: "trigger-btn",
                onclick: move |_| palette.toggle(),
                "Open Command Palette"
            }

            p { class: "last-action", "Last selected: {selected}" }

            CommandDialog {
                open: palette.open,

                CommandRoot {
                    on_select: move |value: String| {
                        palette.hide();
                        selected.set(value);
                    },

                    PaletteContent {}
                }
            }
        }
    }
}

/// Inner content extracted so `use_command_pages` can access `CommandContext`.
#[component]
fn PaletteContent() -> Element {
    let pages = use_command_pages();

    // Breadcrumb display
    let crumbs = pages.breadcrumbs();

    rsx! {
        // Breadcrumb bar (only shown when navigated into a page)
        if !crumbs.is_empty() {
            div { class: "breadcrumbs",
                button {
                    class: "breadcrumb-btn",
                    onclick: move |_| { pages.clear(); },
                    "Root"
                }
                for (id, title) in crumbs.iter() {
                    span { class: "breadcrumb-sep", " / " }
                    span { class: "breadcrumb-current",
                        "{title.as_deref().unwrap_or(id)}"
                    }
                }
            }
        }

        CommandInput {
            placeholder: if pages.is_root() { "Type a command..." } else { "Search..." },
            autofocus: true,
        }

        CommandList {
            label: "Workout Commands",

            CommandEmpty { "No results found." }

            // ── Root-level commands ──────────────────────
            CommandGroup { id: "workout-actions", heading: "Workout",
                CommandItem {
                    id: "jump-to-exercise",
                    label: "Jump to Exercise",
                    keywords: vec!["navigate".to_string(), "go".to_string()],
                    on_select: move |_: String| { pages.push("exercises"); },
                    "Jump to Exercise"
                }
                CommandItem {
                    id: "add-set",
                    label: "Add Set to Exercise",
                    keywords: vec!["log".to_string(), "record".to_string()],
                    on_select: move |_: String| { pages.push("add-set"); },
                    "Add Set to Exercise"
                }
                CommandItem {
                    id: "start-timer",
                    label: "Start Rest Timer",
                    value: "timer:start",
                    keywords: vec!["rest".to_string(), "countdown".to_string()],
                    "Start Rest Timer"
                }
                CommandItem {
                    id: "finish-workout",
                    label: "Finish Workout",
                    value: "workout:finish",
                    keywords: vec!["done".to_string(), "complete".to_string()],
                    "Finish Workout"
                }
            }

            // ── Exercises page (drill-in from "Jump to Exercise") ──
            CommandPage { id: "exercises", title: "Choose Exercise",
                CommandGroup { id: "upper-body", heading: "Upper Body",
                    CommandItem { id: "bench-press", label: "Bench Press", value: "exercise:bench-press",
                        "Bench Press"
                    }
                    CommandItem { id: "overhead-press", label: "Overhead Press", value: "exercise:overhead-press",
                        "Overhead Press"
                    }
                    CommandItem { id: "pull-up", label: "Pull-up", value: "exercise:pull-up",
                        "Pull-up"
                    }
                    CommandItem { id: "barbell-row", label: "Barbell Row", value: "exercise:barbell-row",
                        "Barbell Row"
                    }
                }

                CommandSeparator { group_before: "upper-body", group_after: "lower-body" }

                CommandGroup { id: "lower-body", heading: "Lower Body",
                    CommandItem { id: "squat", label: "Squat", value: "exercise:squat",
                        "Squat"
                    }
                    CommandItem { id: "deadlift", label: "Deadlift", value: "exercise:deadlift",
                        "Deadlift"
                    }
                    CommandItem { id: "leg-press", label: "Leg Press", value: "exercise:leg-press",
                        "Leg Press"
                    }
                    CommandItem { id: "lunges", label: "Lunges", value: "exercise:lunges",
                        "Lunges"
                    }
                }
            }

            // ── Add set page (drill-in from "Add Set to Exercise") ──
            CommandPage { id: "add-set", title: "Add Set",
                CommandGroup { id: "recent-exercises", heading: "Recent Exercises",
                    CommandItem { id: "set-bench", label: "Bench Press", value: "add-set:bench-press",
                        "Bench Press"
                    }
                    CommandItem { id: "set-squat", label: "Squat", value: "add-set:squat",
                        "Squat"
                    }
                    CommandItem { id: "set-deadlift", label: "Deadlift", value: "add-set:deadlift",
                        "Deadlift"
                    }
                }

                CommandSeparator { group_before: "recent-exercises", group_after: "all-exercises" }

                CommandGroup { id: "all-exercises", heading: "All Exercises",
                    CommandItem { id: "set-ohp", label: "Overhead Press", value: "add-set:overhead-press",
                        "Overhead Press"
                    }
                    CommandItem { id: "set-row", label: "Barbell Row", value: "add-set:barbell-row",
                        "Barbell Row"
                    }
                    CommandItem { id: "set-pullup", label: "Pull-up", value: "add-set:pull-up",
                        "Pull-up"
                    }
                    CommandItem { id: "set-lunge", label: "Lunges", value: "add-set:lunges",
                        "Lunges"
                    }
                }
            }
        }
    }
}
