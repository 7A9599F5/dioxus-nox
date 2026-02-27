use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{extract_clipboard_text, use_tag_input, Tag, TagLike};

fn main() {
    dioxus::launch(App);
}

/// Simulated database of tags that would normally come from a server.
fn all_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("javascript", "JavaScript"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("csharp", "C#"),
        Tag::new("cpp", "C++"),
        Tag::new("ruby", "Ruby"),
        Tag::new("swift", "Swift"),
        Tag::new("kotlin", "Kotlin"),
        Tag::new("scala", "Scala"),
        Tag::new("elixir", "Elixir"),
        Tag::new("haskell", "Haskell"),
        Tag::new("clojure", "Clojure"),
        Tag::new("php", "PHP"),
        Tag::new("dart", "Dart"),
        Tag::new("lua", "Lua"),
        Tag::new("zig", "Zig"),
        Tag::new("nim", "Nim"),
    ]
}

/// Simulate a server search. In a real app this would be `fetch()` or `reqwest`.
fn search_tags(query: &str) -> Vec<Tag> {
    let query_lower = query.to_lowercase();
    all_tags()
        .into_iter()
        .filter(|t| t.name().to_lowercase().contains(&query_lower))
        .collect()
}

#[component]
fn App() -> Element {
    // Start with empty available_tags — they come from async search
    let mut state = use_tag_input(vec![], vec![]);

    // Wire up async search callback
    use_effect(move || {
        state
            .on_search
            .set(Some(Callback::new(move |query: String| {
                if query.is_empty() {
                    state.async_suggestions.set(None);
                    state.is_loading.set(false);
                    return;
                }
                // In a real app: spawn(async move { fetch(...).await; })
                // For this demo, search synchronously to avoid extra deps.
                let results = search_tags(&query);
                state.async_suggestions.set(Some(results));
            })));
    });

    // Announce suggestion count
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
                    "Async Tag Search"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Type to search programming languages. Results are loaded via on_search callback."
                }

                div {
                    class: "relative",

                    div {
                        class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-cyan-500 focus-within:ring-1 focus-within:ring-cyan-500/50 transition-all motion-reduce:transition-none",

                        for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                            {
                                let is_pill_active = (*state.active_pill.read()) == Some(i);
                                let pill_ring = if is_pill_active { "ring-2 ring-cyan-400" } else { "" };
                                rsx! {
                                    span {
                                        key: "{tag.id}",
                                        id: state.pill_id(i),
                                        class: "inline-flex items-center gap-1 rounded-lg bg-cyan-600/30 border border-cyan-500/40 px-2.5 py-0.5 text-sm text-cyan-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-cyan-400 {pill_ring}",
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "ml-0.5 rounded hover:bg-cyan-500/30 px-1 transition-colors motion-reduce:transition-none",
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
                            placeholder: "Search languages\u{2026}",
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

                        if *state.is_loading.read() {
                            span {
                                class: "text-xs text-cyan-400 animate-pulse",
                                "Loading\u{2026}"
                            }
                        }
                    }

                    // Dropdown
                    if *state.is_dropdown_open.read()
                        && !state.filtered_suggestions.read().is_empty()
                        && !*state.is_loading.read()
                    {
                        div {
                            id: state.listbox_id(),
                            role: "listbox",
                            aria_multiselectable: "true",
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                            for (i, suggestion) in state.filtered_suggestions.read().iter().cloned().enumerate() {
                                {
                                    let is_active = *state.highlighted_index.read() == Some(i);
                                    let bg = if is_active { "bg-cyan-600/80 text-white" } else { "" };
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

                    // No matches
                    if *state.is_dropdown_open.read()
                        && *state.has_no_matches.read()
                        && !*state.is_loading.read()
                    {
                        div {
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg px-3 py-4 text-sm text-slate-400 text-center",
                            "No languages found."
                        }
                    }
                }

                // Screen-reader live region
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message}"
                }

                p {
                    class: "mt-3 text-xs text-slate-500",
                    "Demonstrates: is_loading, async_suggestions, on_search, has_no_matches"
                }
            }
        }
    }
}
