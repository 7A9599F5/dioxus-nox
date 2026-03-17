use dioxus::prelude::*;
use dioxus_nox_cmdk::*;

const STYLE: Asset = asset!("/assets/style.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let palette = use_command_palette(true);
    let mut last_search = use_signal(String::new);
    let mut show_list_callout = use_signal(|| true);

    // Wave 2 demo / test-harness state
    let mut should_filter = use_signal(|| true);
    let mut debounce_ms: Signal<u32> = use_signal(|| 0);
    let mut on_value_change_log = use_signal(String::new);
    let mut controlled_input_value = use_signal(String::new);

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }

        div { class: "page",
            h1 { "dioxus-cmdk Example" }
            p { "Press ", kbd { "Cmd+K" }, " or click the button below to open the command palette." }

            button {
                class: "trigger-btn",
                onclick: move |_| palette.toggle(),
                "Open Command Palette"
            }

            // Show last search query for demo purposes
            if !last_search.read().is_empty() {
                p { class: "last-search",
                    "Last search: \"{last_search}\""
                }
            }

            // ── Wave 2 test controls ─────────────────────────────────────
            // These controls let Playwright (and manual testers) exercise
            // the new Wave 2 props without recompiling the example.
            div {
                "data-testid": "wave2-controls",
                style: "margin-top:1rem;padding:1rem;border:1px solid #333;border-radius:6px;display:flex;flex-wrap:wrap;gap:.5rem;align-items:center;",

                // P-002: toggle should_filter
                button {
                    "data-testid": "toggle-filter",
                    onclick: move |_| { should_filter.set(!should_filter()); },
                    if should_filter() { "should_filter: ON" } else { "should_filter: OFF" }
                }

                // P-016: toggle search_debounce_ms (0 ↔ 300)
                button {
                    "data-testid": "toggle-debounce",
                    onclick: move |_| {
                        debounce_ms.set(if debounce_ms() == 0 { 300 } else { 0 });
                    },
                    if debounce_ms() == 0 { "Debounce: OFF (0 ms)" } else { "Debounce: ON (300 ms)" }
                }

                // P-013: programmatically set controlled input value
                button {
                    "data-testid": "set-controlled-input",
                    onclick: move |_| { controlled_input_value.set("settings".to_string()); },
                    "Set input → 'settings'"
                }
                button {
                    "data-testid": "clear-controlled-input",
                    onclick: move |_| { controlled_input_value.set(String::new()); },
                    "Clear input"
                }

                // P-012: on_value_change output
                span {
                    style: "margin-left:auto;font-size:.85rem;color:#aaa;",
                    "on_value_change: "
                    span {
                        "data-testid": "value-change-log",
                        style: "color:#7dd3fc;font-family:monospace;",
                        if on_value_change_log.read().is_empty() {
                            "—"
                        } else {
                            "{on_value_change_log}"
                        }
                    }
                }
            }

            // The CommandDialog wraps everything in a modal with focus trap
            CommandDialog {
                open: palette.open,

                // CommandRoot provides context for all child components
                CommandRoot {
                    on_select: move |value: String| {
                        palette.hide();
                        println!("Selected: {value}");
                    },
                    on_search_change: move |query: String| {
                        last_search.set(query);
                    },
                    // P-002
                    should_filter: should_filter(),
                    // P-016
                    search_debounce_ms: debounce_ms(),
                    // P-012
                    on_value_change: move |v: String| {
                        on_value_change_log.set(v);
                    },

                    // Search input — P-013: controlled when controlled_input_value is non-empty
                    CommandInput {
                        placeholder: "Type a command or search...",
                        autofocus: true,
                        value: {
                            // Only engage controlled mode when the signal has been explicitly set
                            if controlled_input_value.read().is_empty() {
                                None
                            } else {
                                Some(controlled_input_value)
                            }
                        },
                    }

                    // Input-area callout: visible only when the search field is empty
                    if last_search.read().is_empty() {
                        CommandCallout {
                            class: "input-hint",
                            r#"Tip: type ">" for commands, "/" for files"#
                        }
                    }

                    CommandList {
                        label: "Commands",
                        CommandEmpty { "No results found." }
                        CommandLoading { "Loading..." }

                        // List callout: dismissible first-run hint before the first group
                        if show_list_callout() {
                            CommandCallout {
                                dismissible: true,
                                on_dismiss: move |_| show_list_callout.set(false),
                                "New here? Try typing "
                                kbd { ">" }
                                " for editor commands or "
                                kbd { "/" }
                                " for files."
                            }
                        }

                        CommandGroup {
                            id: "navigation",
                            heading: "Navigation",
                            CommandItem {
                                id: "home",
                                label: "Home",
                                value: "/",
                                keywords: vec!["dashboard".to_string(), "main".to_string()],
                                shortcut: Hotkey::try_parse("ctrl+h"),
                                span { "Home" }
                                CommandShortcut { "Ctrl+H" }
                            }
                            CommandItem {
                                id: "settings",
                                label: "Settings",
                                value: "/settings",
                                keywords: vec!["preferences".to_string(), "config".to_string()],
                                shortcut: Hotkey::try_parse("ctrl+s"),
                                span { "Settings" }
                                CommandShortcut { "Ctrl+S" }
                            }
                            CommandItem {
                                id: "profile",
                                label: "Profile",
                                value: "/profile",
                                keywords: vec!["account".to_string(), "user".to_string()],
                                shortcut: Hotkey::try_parse("ctrl+p"),
                                span { "Profile" }
                                CommandShortcut { "Ctrl+P" }
                            }
                        }

                        CommandSeparator { group_before: "navigation", group_after: "actions" }

                        CommandGroup {
                            id: "actions",
                            heading: "Actions",
                            CommandItem {
                                id: "new-file",
                                label: "New File",
                                keywords: vec!["create".to_string(), "add".to_string()],
                                shortcut: Hotkey::try_parse("alt+n"),
                                span { "New File" }
                                CommandShortcut { "Alt+N" }
                            }
                            CommandItem {
                                id: "copy-link",
                                label: "Copy Link",
                                keywords: vec!["url".to_string(), "share".to_string()],
                                shortcut: Hotkey::try_parse("ctrl+l"),
                                span { "Copy Link" }
                                CommandShortcut { "Ctrl+L" }
                            }
                            CommandItem {
                                id: "delete",
                                label: "Delete",
                                keywords: vec!["remove".to_string(), "trash".to_string()],
                                disabled: true,
                                span { "Delete (disabled)" }
                            }
                        }

                        CommandSeparator { group_before: "actions", group_after: "theme" }

                        CommandGroup {
                            id: "theme",
                            heading: "Theme",
                            CommandItem {
                                id: "light-theme",
                                label: "Light Theme",
                                keywords: vec!["appearance".to_string(), "bright".to_string()],
                                span { "Light Theme" }
                            }
                            CommandItem {
                                id: "dark-theme",
                                label: "Dark Theme",
                                keywords: vec!["appearance".to_string(), "night".to_string()],
                                span { "Dark Theme" }
                            }
                            CommandItem {
                                id: "system-theme",
                                label: "System Theme",
                                keywords: vec!["appearance".to_string(), "auto".to_string()],
                                span { "System Theme" }
                            }
                        }
                    }
                }
            }
        }
    }
}
