use dioxus::prelude::*;
use dioxus_nox_cmdk::*;

const STYLE: Asset = asset!("/assets/style.css");

#[derive(Clone, Copy, PartialEq)]
enum InputPosition {
    Bottom,
    Top,
    Middle,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut position = use_signal(|| InputPosition::Bottom);
    let mut is_open = use_signal(|| false);
    let mut last_selected = use_signal(String::new);

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }

        div { class: "page",
            h1 { "Floating CommandList" }
            p {
                "The list positions itself relative to the search input. "
                "Move the input to see auto-placement in action."
            }

            // ── Position switcher ─────────────────────────────────────
            div { class: "controls",
                button {
                    class: if position() == InputPosition::Bottom { "active" } else { "" },
                    onclick: move |_| position.set(InputPosition::Bottom),
                    "Input: Bottom"
                }
                button {
                    class: if position() == InputPosition::Top { "active" } else { "" },
                    onclick: move |_| position.set(InputPosition::Top),
                    "Input: Top"
                }
                button {
                    class: if position() == InputPosition::Middle { "active" } else { "" },
                    onclick: move |_| position.set(InputPosition::Middle),
                    "Input: Middle (auto)"
                }
            }

            if !last_selected.read().is_empty() {
                p { class: "selected-log",
                    "Selected: " strong { "{last_selected}" }
                }
            }

            // ── CommandRoot — no dialog wrapper ───────────────────────
            // The input is positioned by CSS class; the list floats relative to it.
            CommandRoot {
                on_select: move |value: String| {
                    last_selected.set(value);
                    is_open.set(false);
                },
                on_close: move |_| is_open.set(false),

                // CommandAnchor wraps the input — reports its rect to context.
                // class drives the input position on the page.
                CommandAnchor {
                    class: match position() {
                        InputPosition::Bottom => "anchor anchor--bottom",
                        InputPosition::Top    => "anchor anchor--top",
                        InputPosition::Middle => "anchor anchor--middle",
                    },
                    CommandInput {
                        placeholder: "Search commands...",
                        autofocus: true,
                        onfocus: move |_| is_open.set(true),
                        onblur:  move |_| is_open.set(false),
                    }
                }

                // CommandList with floating=true anchors to CommandAnchor above.
                // preferred_side drives the initial preference; auto-flip kicks in
                // when the preferred side lacks space.
                if is_open() {
                    CommandList {
                        floating: true,
                        // Bottom input → prefer Top; Top input → prefer Bottom; Middle → auto
                        preferred_side: match position() {
                            InputPosition::Bottom => Side::Top,
                            InputPosition::Top    => Side::Bottom,
                            InputPosition::Middle => Side::Bottom, // auto-flips if needed
                        },
                        side_offset: 6.0,
                        label: "Commands",

                        CommandEmpty { "No results." }

                        CommandGroup { id: "nav", heading: "Navigation",
                            CommandItem { id: "home",     label: "Home",     span { "Home"     } }
                            CommandItem { id: "docs",     label: "Docs",     span { "Docs"     } }
                            CommandItem { id: "settings", label: "Settings", span { "Settings" } }
                            CommandItem { id: "profile",  label: "Profile",  span { "Profile"  } }
                        }

                        CommandGroup { id: "actions", heading: "Actions",
                            CommandItem { id: "new",    label: "New File",   span { "New File"   } }
                            CommandItem { id: "delete", label: "Delete",     span { "Delete"     } }
                            CommandItem { id: "share",  label: "Share Link", span { "Share Link" } }
                        }
                    }
                }
            }
        }
    }
}
