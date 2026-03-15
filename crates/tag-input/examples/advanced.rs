use std::cmp::Ordering;

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{Tag, TagLike, extract_clipboard_text, use_tag_input};

fn main() {
    dioxus::launch(App);
}

fn demo_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("javascript", "JavaScript"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("ruby", "Ruby"),
        Tag::new("swift", "Swift"),
        Tag::new("kotlin", "Kotlin"),
        Tag::new("elixir", "Elixir"),
        Tag::new("haskell", "Haskell"),
        Tag::new("php", "PHP"),
    ]
}

#[component]
fn App() -> Element {
    let mut state = use_tag_input(demo_tags(), vec![]);

    // Configure: max tag length of 12, auto-sort alphabetically, max 3 visible tags,
    // deny list blocking "PHP", max 5 suggestions
    use_hook(|| {
        state.max_tag_length.set(Some(12));
        state.max_visible_tags.set(Some(3));
        state.max_suggestions.set(Some(5));
        state.deny_list.set(Some(vec!["php".to_string()]));
        let sort_fn: fn(&Tag, &Tag) -> Ordering = |a, b| a.name().cmp(b.name());
        state.sort_selected.set(Some(sort_fn));
    });

    // Announce suggestion count
    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    let overflow = *state.overflow_count.read();
    let total_filtered = *state.total_filtered_count.read();
    let shown = state.filtered_suggestions.read().len();

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Advanced Features"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Max 12-char tags, auto-sorted, +N overflow, deny list (PHP blocked), max 5 suggestions."
                }

                div {
                    class: "relative",

                    div {
                        class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500/50 transition-all motion-reduce:transition-none",

                        // Render only visible_tags (respects max_visible_tags)
                        for (i, tag) in state.visible_tags.read().iter().cloned().enumerate() {
                            {
                                let is_pill_active = (*state.active_pill.read()) == Some(i);
                                let pill_ring = if is_pill_active { "ring-2 ring-emerald-400" } else { "" };
                                rsx! {
                                    span {
                                        key: "{tag.id}",
                                        id: state.pill_id(i),
                                        class: "inline-flex items-center gap-1 rounded-lg bg-emerald-600/30 border border-emerald-500/40 px-2.5 py-0.5 text-sm text-emerald-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-emerald-400 {pill_ring}",
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

                        // Overflow badge
                        if overflow > 0 {
                            span {
                                class: "rounded-lg bg-slate-700 border border-slate-600 px-2.5 py-0.5 text-sm text-slate-300",
                                "+{overflow} more"
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
                    }

                    // Validation error
                    if let Some(ref err) = *state.validation_error.read() {
                        div {
                            class: "mt-1 text-xs text-red-400",
                            "{err}"
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
                                    let bg = if is_active { "bg-emerald-600/80 text-white" } else { "" };
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

                            // Show truncation info if max_suggestions is capping results
                            if shown < total_filtered {
                                div {
                                    class: "px-3 py-2 text-xs text-slate-400 border-t border-slate-700",
                                    "Showing {shown} of {total_filtered} — type to refine"
                                }
                            }
                        }
                    }

                    // No matches
                    if *state.is_dropdown_open.read() && *state.has_no_matches.read() {
                        div {
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg px-3 py-4 text-sm text-slate-400 text-center",
                            "No matches found."
                        }
                    }
                }

                // Info panel
                div {
                    class: "mt-3 rounded-lg bg-slate-900/60 border border-slate-700/50 px-3 py-2 text-xs text-slate-400 space-y-1",
                    div { "Selected: {state.selected_tags.read().len()} tags (sorted alphabetically)" }
                    div { "Visible: {state.visible_tags.read().len()} + {overflow} overflow" }
                    div { "Form value: {state.form_value}" }
                    if let Some(ref ac) = *state.auto_complete_suggestion.read() {
                        div { "Auto-complete: {ac.name()}" }
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
                    "Demonstrates: max_tag_length, sort_selected, max_visible_tags, overflow_count, visible_tags, deny_list, max_suggestions, total_filtered_count, form_value, auto_complete_suggestion, has_no_matches"
                }
            }
        }
    }
}
