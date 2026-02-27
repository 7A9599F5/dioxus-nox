use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{extract_clipboard_text, use_tag_input, TagLike};

fn main() {
    dioxus::launch(App);
}

/// A tag that supports locking (locked tags cannot be removed).
#[derive(Clone, PartialEq, Debug)]
struct FruitTag {
    id: String,
    name: String,
    locked: bool,
}

impl FruitTag {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            locked: false,
        }
    }
    fn locked(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            locked: true,
        }
    }
}

impl TagLike for FruitTag {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn is_locked(&self) -> bool {
        self.locked
    }
}

fn fruit_tags() -> Vec<FruitTag> {
    vec![
        FruitTag::new("apple", "Apple"),
        FruitTag::new("banana", "Banana"),
        FruitTag::new("cherry", "Cherry"),
        FruitTag::new("grape", "Grape"),
        FruitTag::new("mango", "Mango"),
        FruitTag::new("orange", "Orange"),
    ]
}

#[component]
fn App() -> Element {
    // Start with a locked "Cherry" tag that cannot be removed
    let initial = vec![FruitTag::locked("cherry", "Cherry")];
    let mut state = use_tag_input(fruit_tags(), initial);

    // Track the last callback event in a signal for display
    let mut last_event = use_signal(|| String::from("No events yet"));

    // Wire up on_add / on_remove callbacks for demonstration
    use_effect(move || {
        state.on_add.set(Some(Callback::new(move |tag: FruitTag| {
            last_event.set(format!("Added: {}", tag.name));
        })));
        state
            .on_remove
            .set(Some(Callback::new(move |tag: FruitTag| {
                last_event.set(format!("Removed: {}", tag.name));
            })));
    });

    // Limit to 4 tags
    use_hook(|| {
        state.max_tags.set(Some(4));
    });

    // Announce filtered suggestion count to screen readers when it changes
    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Pick some fruits"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Cherry is locked and cannot be removed. Max 4 tags."
                }

                // Tag input area
                div {
                    class: "relative",

                    div {
                        class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none",

                        for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                            {
                                let is_pill_active = (*state.active_pill.read()) == Some(i);
                                let pill_ring = if is_pill_active { "ring-2 ring-indigo-400" } else { "" };
                                let locked_style = if tag.is_locked() { "opacity-80" } else { "" };
                                rsx! {
                                    span {
                                        key: "{tag.id}",
                                        id: state.pill_id(i),
                                        class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/30 border border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring} {locked_style}",
                                        "{tag.name}"
                                        if tag.is_locked() {
                                            span {
                                                class: "ml-0.5 text-indigo-400/50 text-xs",
                                                "\u{1F512}"
                                            }
                                        } else {
                                            button {
                                                r#type: "button",
                                                class: "ml-0.5 rounded hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none",
                                                onclick: move |_| state.remove_tag(&tag.id),
                                                "\u{00D7}"
                                            }
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
                            placeholder: "Type to search\u{2026}",
                            value: "{state.search_query}",
                            oninput: move |evt| state.set_query(evt.value()),
                            onkeydown: move |evt| state.handle_keydown(evt),
                            onclick: move |_| state.handle_click(),
                            onfocus: move |_| state.is_dropdown_open.set(true),
                            onblur: move |_| state.close_dropdown(),
                            onpaste: move |evt: Event<ClipboardData>| {
                                if let Some(text) = extract_clipboard_text(&evt) {
                                    evt.prevent_default();
                                    state.handle_paste(text);
                                }
                            },
                        }
                        if *state.is_at_limit.read() {
                            span {
                                class: "text-xs text-amber-400 ml-1",
                                "(limit reached)"
                            }
                        }
                    }

                    // Dropdown
                    if *state.is_dropdown_open.read() && !state.filtered_suggestions.read().is_empty() {
                        div {
                            id: state.listbox_id(),
                            role: "listbox",
                            aria_multiselectable: "true",
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

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
                                            class: "px-3 py-2 text-sm cursor-pointer transition-colors hover:bg-slate-700 {bg}",
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

                // Callback event log
                div {
                    class: "mt-3 rounded-lg bg-slate-900/60 border border-slate-700/50 px-3 py-2 text-xs text-slate-400",
                    span { class: "text-slate-500 mr-1", "Last event:" }
                    "{last_event}"
                }

                // Screen-reader live region for status announcements
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message}"
                }

                // Keyboard shortcuts hint
                p {
                    class: "mt-3 text-xs text-slate-500",
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2191}\u{2193}" }
                    "navigate  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
                    "pills  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                    "select  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
                    "remove  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Esc" }
                    "close"
                }
            }
        }
    }
}
