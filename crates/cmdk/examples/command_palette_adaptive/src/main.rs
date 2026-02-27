use dioxus::prelude::*;
use dioxus_nox_cmdk::*;

const STYLE: Asset = asset!("/assets/style.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let palette = use_adaptive_palette();
    let mut last_selected = use_signal(String::new);

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }

        div { class: "page",
            h1 { "dioxus-cmdk Adaptive" }
            p {
                "This palette auto-detects mobile vs desktop. "
                "Try toggling device emulation in DevTools."
            }

            // Conditional trigger based on detected mode
            if (palette.is_mobile)() {
                button {
                    class: "trigger-btn",
                    onclick: move |_| palette.show(),
                    "Tap to Open"
                }
            } else {
                p { class: "hint",
                    "Press "
                    kbd { "Cmd+K" }
                    " or "
                    button {
                        class: "trigger-btn",
                        onclick: move |_| palette.toggle(),
                        "Open Command Palette"
                    }
                }
            }

            if !last_selected.read().is_empty() {
                p { class: "last-selected",
                    "Selected: \"{last_selected}\""
                }
            }

            p { class: "mode-indicator",
                "Mode: "
                strong {
                    if (palette.is_mobile)() { "Sheet (mobile)" } else { "Dialog (desktop)" }
                }
            }

            CommandPalette {
                open: palette.open,
                snap_points: vec![0.5, 1.0],
                close_threshold: 0.4,
                autofocus_on_open: true,

                CommandRoot {
                    on_select: move |value: String| {
                        last_selected.set(value);
                        palette.hide();
                    },

                    CommandInput {
                        placeholder: "Search commands...",
                        autofocus: true,
                    }

                    CommandList {
                        label: "Commands",

                        CommandEmpty { "No results found." }

                        CommandGroup {
                            id: "navigation",
                            heading: "Navigation",

                            CommandItem {
                                id: "home",
                                label: "Home",
                                value: "/",
                                keywords: vec!["dashboard".to_string()],
                                "Home"
                            }
                            CommandItem {
                                id: "settings",
                                label: "Settings",
                                value: "/settings",
                                keywords: vec!["preferences".to_string(), "config".to_string()],
                                "Settings"
                            }
                            CommandItem {
                                id: "profile",
                                label: "Profile",
                                value: "/profile",
                                keywords: vec!["account".to_string(), "user".to_string()],
                                "Profile"
                            }
                        }

                        CommandSeparator {
                            group_before: "navigation",
                            group_after: "actions",
                        }

                        CommandGroup {
                            id: "actions",
                            heading: "Actions",

                            CommandItem {
                                id: "new-file",
                                label: "New File",
                                keywords: vec!["create".to_string(), "add".to_string()],
                                "New File"
                            }
                            CommandItem {
                                id: "share",
                                label: "Share",
                                keywords: vec!["send".to_string(), "export".to_string()],
                                "Share"
                            }
                            CommandItem {
                                id: "download",
                                label: "Download",
                                keywords: vec!["save".to_string()],
                                "Download"
                            }
                        }

                        CommandSeparator {
                            group_before: "actions",
                            group_after: "theme",
                        }

                        CommandGroup {
                            id: "theme",
                            heading: "Theme",

                            CommandItem {
                                id: "light",
                                label: "Light Theme",
                                keywords: vec!["appearance".to_string()],
                                "Light"
                            }
                            CommandItem {
                                id: "dark",
                                label: "Dark Theme",
                                keywords: vec!["appearance".to_string()],
                                "Dark"
                            }
                            CommandItem {
                                id: "system",
                                label: "System Theme",
                                keywords: vec!["auto".to_string()],
                                "System"
                            }
                        }
                    }
                }
            }
        }
    }
}
