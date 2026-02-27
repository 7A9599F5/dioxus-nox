use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{use_tag_input, Tag};

fn main() {
    dioxus::launch(App);
}

/// Simple counter for generating unique IDs.
static NEXT_ID: GlobalSignal<u32> = Signal::global(|| 1000);

fn next_id() -> String {
    let id = *NEXT_ID.read();
    *NEXT_ID.write() += 1;
    format!("created-{id}")
}

#[component]
fn App() -> Element {
    // Start with a few seed tags; the user can create more.
    let seed_tags = vec![
        Tag::new("work", "Work"),
        Tag::new("personal", "Personal"),
        Tag::new("urgent", "Urgent"),
    ];

    let mut state = use_tag_input(seed_tags, vec![]);

    // Enable ad-hoc creation: type anything and press Enter.
    use_hook(|| {
        state.on_create.set(Some(Callback::new(move |text: String| {
            let tag = Tag::new(next_id(), text);
            Some(tag)
        })));
    });

    // Enable paste splitting on comma, newline, and tab.
    use_hook(|| {
        state.paste_delimiters.set(Some(vec![',', '\n', '\t']));
    });

    // Enable comma as a commit delimiter while typing.
    use_hook(|| {
        state.delimiters.set(Some(vec![',']));
    });

    // Announce suggestion count to screen readers when filtered list changes.
    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    // Derive "can create" hint from current state
    let query = state.search_query.read().clone();
    let highlight = *state.highlighted_index.read();
    let show_create_hint = !query.is_empty() && highlight.is_none();

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Tag your tasks"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Type a new tag name and press Enter or comma to create it. Comma or Enter commits."
                }

                // Tag input area
                div {
                    class: "relative",

                    div {
                        class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500/50 transition-all motion-reduce:transition-none",

                        for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                            {
                                let is_pill_active = (*state.active_pill.read()) == Some(i);
                                let pill_ring = if is_pill_active { "ring-2 ring-emerald-400" } else { "" };
                                rsx! {
                                    span {
                                        key: "{tag.id}",
                                        id: state.pill_id(i),
                                        class: "inline-flex items-center gap-1 rounded-lg bg-emerald-600/30 border border-emerald-500/40 px-2.5 py-0.5 text-sm text-emerald-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-emerald-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "ml-0.5 rounded hover:bg-emerald-500/30 px-1 transition-colors motion-reduce:transition-none",
                                            onclick: move |_| state.remove_tag(&tag.id),
                                            "\u{00D7}"
                                        }
                                    }
                                }
                            }
                        }

                        input {
                            r#type: "text",
                            role: "combobox",
                            aria_expanded: state.aria_expanded(),
                            aria_controls: state.listbox_id(),
                            aria_activedescendant: state.active_descendant(),
                            aria_autocomplete: "list",
                            class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
                            placeholder: "Add a tag\u{2026}",
                            value: "{state.search_query}",
                            oninput: move |evt| state.set_query(evt.value()),
                            onkeydown: move |evt| state.handle_keydown(evt),
                            onpaste: move |evt: Event<ClipboardData>| {
                                if let Some(text) = dioxus_nox_tag_input::extract_clipboard_text(&evt) {
                                    evt.prevent_default();
                                    state.handle_paste(text);
                                }
                            },
                            onclick: move |_| state.handle_click(),
                            onfocus: move |_| state.is_dropdown_open.set(true),
                            onblur: move |_| state.close_dropdown(),
                        }
                    }

                    // Dropdown with suggestions + create hint
                    if *state.is_dropdown_open.read() {
                        div {
                            id: state.listbox_id(),
                            role: "listbox",
                            aria_multiselectable: "true",
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                            for (i, suggestion) in state.filtered_suggestions.read().iter().cloned().enumerate() {
                                {
                                    let is_active = *state.highlighted_index.read() == Some(i);
                                    let bg = if is_active { "bg-emerald-600/80 text-white" } else { "" };
                                    rsx! {
                                        div {
                                            key: "{suggestion.id}",
                                            id: state.suggestion_id(i),
                                            role: "option",
                                            aria_selected: if is_active { "true" } else { "false" },
                                            class: "px-3 py-2 text-sm cursor-pointer transition-colors motion-reduce:transition-none hover:bg-slate-700 {bg}",
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

                            // "Press Enter to create" hint
                            if show_create_hint {
                                div {
                                    class: "px-3 py-2 text-sm text-slate-400 border-t border-slate-700",
                                    "Press "
                                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5", "Enter" }
                                    " to create "
                                    span { class: "font-semibold text-emerald-300", "\"{query}\"" }
                                }
                            }
                        }
                    }
                }

                // Screen reader live region for status announcements
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "sr-only absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message}"
                }

                // Keyboard hints
                p {
                    class: "mt-3 text-xs text-slate-500",
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                    "create / select  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2191}\u{2193}" }
                    "navigate  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
                    "remove"
                }
            }
        }
    }
}
