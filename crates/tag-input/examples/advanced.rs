use std::cmp::Ordering;

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{Tag, TagInputState, combo};

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
    let available = demo_tags();

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
                    "Max 12-char tags, auto-sorted, +N overflow, deny list (PHP blocked). Dropdown for selection."
                }

                combo::Root::<Tag> {
                    available_tags: available.clone(),
                    deny_list: vec!["php".to_string()],
                    close_on_select: true,
                    class: "relative",

                    AdvancedInner { available: available.clone() }
                }

                p {
                    class: "mt-3 text-xs text-slate-500",
                    "Demonstrates: max_tag_length, sort_selected, max_visible_tags, overflow_count, visible_tags, deny_list, form_value, select dropdown"
                }
            }
        }
    }
}

/// Inner component that reads tag-input state from context (provided by combo::Root).
#[component]
fn AdvancedInner(available: Vec<Tag>) -> Element {
    let mut state = use_context::<TagInputState<Tag>>();

    // Configure: max tag length of 12, auto-sort alphabetically, max 3 visible tags
    use_hook(|| {
        state.max_tag_length.set(Some(12));
        state.max_visible_tags.set(Some(3));
        let sort_fn: fn(&Tag, &Tag) -> Ordering = |a, b| a.name.cmp(&b.name);
        state.sort_selected.set(Some(sort_fn));
    });

    let overflow = *state.overflow_count.read();

    rsx! {
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

            combo::Input::<Tag> {
                placeholder: "Search languages\u{2026}",
                class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
            }
        }

        combo::Dropdown {
            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
            combo::Empty {
                class: "px-3 py-2 text-sm text-slate-500",
                "No results found."
            }
            for tag in &available {
                combo::Item {
                    value: "{tag.id}",
                    label: tag.name.clone(),
                    class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-emerald-600/30 data-[state=checked]:text-emerald-300 data-[disabled]:opacity-50 data-[disabled]:cursor-not-allowed",
                    "{tag.name}"
                }
            }
        }

        // Validation error
        if let Some(ref err) = *state.validation_error.read() {
            div {
                class: "mt-1 text-xs text-red-400",
                "{err}"
            }
        }

        // Info panel
        div {
            class: "mt-3 rounded-lg bg-slate-900/60 border border-slate-700/50 px-3 py-2 text-xs text-slate-400 space-y-1",
            div { "Selected: {state.selected_tags.read().len()} tags (sorted alphabetically)" }
            div { "Visible: {state.visible_tags.read().len()} + {overflow} overflow" }
            div { "Form value: {state.form_value}" }
        }

        // Screen-reader live region
        div {
            role: "status",
            aria_live: "polite",
            class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
            "{state.status_message}"
        }
    }
}
