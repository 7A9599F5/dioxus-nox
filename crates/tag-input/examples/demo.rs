use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{use_breakpoint, use_tag_input, Breakpoint, Tag};

fn main() {
    dioxus::launch(App);
}

fn sample_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("csharp", "C#"),
        Tag::new("swift", "Swift"),
        Tag::new("kotlin", "Kotlin"),
        Tag::new("ruby", "Ruby"),
        Tag::new("elixir", "Elixir"),
    ]
}

#[component]
fn App() -> Element {
    let state = use_tag_input(sample_tags(), vec![]);
    let breakpoint = use_breakpoint();

    let is_mobile = matches!(*breakpoint.read(), Breakpoint::Mobile);

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-start justify-center p-4 sm:p-8",

            div {
                class: "w-full max-w-xl",

                h1 {
                    class: "text-2xl font-bold mb-6 text-slate-50",
                    "Tag Input Demo"
                }

                if is_mobile {
                    MobileLayout { state: state }
                } else {
                    DesktopLayout { state: state }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Desktop / Tablet layout
// ---------------------------------------------------------------------------

#[component]
fn DesktopLayout(mut state: dioxus_nox_tag_input::TagInputState<Tag>) -> Element {
    rsx! {
        div {
            class: "relative",

            // Selected tags + input row
            div {
                class: "flex flex-wrap items-center gap-2 rounded-full border-2 border-dashed border-slate-600 bg-slate-800 px-4 py-2 focus-within:border-slate-400 transition-colors",

                // Selected tag pills
                for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                    DesktopPill {
                        key: "{tag.id}",
                        tag: tag.clone(),
                        active: (*state.active_pill.read()) == Some(i),
                        pill_id: state.pill_id(i),
                        on_remove: move |id: String| state.remove_tag(&id),
                    }
                }

                // Add icon
                span {
                    class: "text-slate-400 text-lg select-none",
                    "+"
                }

                // Search input
                input {
                    r#type: "text",
                    role: "combobox",
                    aria_expanded: state.aria_expanded(),
                    aria_controls: state.listbox_id(),
                    aria_activedescendant: state.active_descendant(),
                    aria_autocomplete: "list",
                    class: "flex-1 min-w-[120px] bg-transparent outline-none text-slate-100 placeholder-slate-500",
                    placeholder: "Add a tag\u{2026}",
                    value: "{state.search_query}",
                    oninput: move |evt| state.set_query(evt.value()),
                    onkeydown: move |evt| state.handle_keydown(evt),
                    onclick: move |_| state.handle_click(),
                    onfocus: move |_| state.is_dropdown_open.set(true),
                    onblur: move |_| {
                        state.close_dropdown();
                    },
                }
            }

            // Autocomplete dropdown
            if *state.is_dropdown_open.read() && !state.filtered_suggestions.read().is_empty() {
                div {
                    id: state.listbox_id(),
                    role: "listbox",
                    class: "absolute z-50 mt-2 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                    for (i, suggestion) in state.filtered_suggestions.read().iter().cloned().enumerate() {
                        {
                            let is_active = *state.highlighted_index.read() == Some(i);
                            let bg = if is_active { "bg-indigo-600/80 text-white" } else { "" };
                            rsx! {
                                div {
                                    key: "{suggestion.id}",
                                    id: state.suggestion_id(i),
                                    role: "option",
                                    aria_selected: if is_active { "true" } else { "false" },
                                    class: "px-4 py-2 cursor-pointer transition-colors hover:bg-slate-700 {bg}",
                                    onmouseenter: move |_| state.highlighted_index.set(Some(i)),
                                    onmousedown: move |evt: Event<MouseData>| {
                                        evt.prevent_default();
                                        state.add_tag(suggestion.clone());
                                    },
                                    "{suggestion.name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DesktopPill(
    tag: Tag,
    active: bool,
    pill_id: String,
    on_remove: EventHandler<String>,
) -> Element {
    let pill_ring = if active { "ring-2 ring-indigo-400" } else { "" };
    rsx! {
        span {
            id: pill_id,
            class: "inline-flex items-center gap-1 rounded-full bg-slate-600 px-3 py-1 text-sm text-slate-100 transition-shadow {pill_ring}",
            "{tag.name}"
            button {
                r#type: "button",
                class: "ml-1 rounded-full hover:bg-slate-500 p-0.5 transition-colors",
                onclick: move |_| on_remove.call(tag.id.clone()),
                "\u{00D7}"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mobile layout
// ---------------------------------------------------------------------------

#[component]
fn MobileLayout(mut state: dioxus_nox_tag_input::TagInputState<Tag>) -> Element {
    let show_sheet = *state.is_dropdown_open.read();

    rsx! {
        div {
            // Horizontal scroll tag strip
            div {
                class: "flex flex-nowrap overflow-x-auto gap-2 pb-3 scrollbar-none",
                for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                    MobilePill {
                        key: "{tag.id}",
                        tag: tag.clone(),
                        active: (*state.active_pill.read()) == Some(i),
                        pill_id: state.pill_id(i),
                        on_remove: move |id: String| state.remove_tag(&id),
                    }
                }
            }

            // Input row
            div {
                class: "flex items-center gap-2 rounded-xl border-2 border-dashed border-slate-600 bg-slate-800 px-4 py-3",
                span {
                    class: "text-slate-400 text-lg select-none",
                    "+"
                }
                input {
                    r#type: "text",
                    role: "combobox",
                    aria_expanded: state.aria_expanded(),
                    aria_controls: state.listbox_id(),
                    aria_activedescendant: state.active_descendant(),
                    aria_autocomplete: "list",
                    class: "flex-1 bg-transparent outline-none text-slate-100 placeholder-slate-500 text-base",
                    placeholder: "Add a tag\u{2026}",
                    value: "{state.search_query}",
                    oninput: move |evt| state.set_query(evt.value()),
                    onkeydown: move |evt| state.handle_keydown(evt),
                    onclick: move |_| state.handle_click(),
                    onfocus: move |_| state.is_dropdown_open.set(true),
                }
            }

            // Scrim overlay + bottom sheet
            if show_sheet && !state.filtered_suggestions.read().is_empty() {
                // Scrim
                div {
                    class: "fixed inset-0 z-40 bg-black/40",
                    onclick: move |_| state.close_dropdown(),
                }

                // Bottom sheet
                div {
                    id: state.listbox_id(),
                    role: "listbox",
                    class: "fixed bottom-0 left-0 right-0 z-50 rounded-t-2xl bg-slate-800 border-t border-slate-700 max-h-[50dvh] overflow-y-auto",

                    // Drag handle
                    div {
                        class: "flex justify-center pt-3 pb-2",
                        div {
                            class: "w-10 h-1 rounded-full bg-slate-600",
                        }
                    }

                    // Suggestions list
                    for (i, suggestion) in state.filtered_suggestions.read().iter().cloned().enumerate() {
                        {
                            let is_active = *state.highlighted_index.read() == Some(i);
                            let bg = if is_active { "bg-indigo-600/80 text-white" } else { "" };
                            rsx! {
                                div {
                                    key: "{suggestion.id}",
                                    id: state.suggestion_id(i),
                                    role: "option",
                                    aria_selected: if is_active { "true" } else { "false" },
                                    class: "min-h-[44px] flex items-center px-4 py-3 cursor-pointer transition-colors active:bg-slate-700 {bg}",
                                    onmouseenter: move |_| state.highlighted_index.set(Some(i)),
                                    onmousedown: move |evt: Event<MouseData>| {
                                        evt.prevent_default();
                                        state.add_tag(suggestion.clone());
                                    },
                                    "{suggestion.name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MobilePill(tag: Tag, active: bool, pill_id: String, on_remove: EventHandler<String>) -> Element {
    let pill_ring = if active { "ring-2 ring-indigo-400" } else { "" };
    rsx! {
        span {
            id: pill_id,
            class: "inline-flex items-center gap-1 shrink-0 rounded-full bg-slate-600 px-3 py-2 text-sm text-slate-100 transition-shadow {pill_ring}",
            "{tag.name}"
            button {
                r#type: "button",
                class: "min-w-[44px] min-h-[44px] flex items-center justify-center rounded-full hover:bg-slate-500 transition-colors -mr-1",
                onclick: move |_| on_remove.call(tag.id.clone()),
                "\u{00D7}"
            }
        }
    }
}
