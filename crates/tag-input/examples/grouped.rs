use dioxus::document::Stylesheet;
use dioxus::events::ClipboardData;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{find_match_ranges, use_tag_input_grouped, TagInputGroupConfig, TagLike};

fn main() {
    dioxus::launch(App);
}

// ---------------------------------------------------------------------------
// Custom tag type with group support
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
struct Skill {
    id: String,
    name: String,
    category: String,
    level: u8,
}

impl Skill {
    fn new(id: &str, name: &str, category: &str, level: u8) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            level,
        }
    }
}

impl TagLike for Skill {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn group(&self) -> Option<&str> {
        Some(&self.category)
    }
}

fn skill_data() -> Vec<Skill> {
    vec![
        // Languages
        Skill::new("rust", "Rust", "Languages", 4),
        Skill::new("ts", "TypeScript", "Languages", 3),
        Skill::new("python", "Python", "Languages", 3),
        Skill::new("go", "Go", "Languages", 2),
        Skill::new("java", "Java", "Languages", 2),
        Skill::new("csharp", "C#", "Languages", 1),
        Skill::new("ruby", "Ruby", "Languages", 2),
        // Frameworks
        Skill::new("react", "React", "Frameworks", 4),
        Skill::new("dioxus", "Dioxus", "Frameworks", 3),
        Skill::new("nextjs", "Next.js", "Frameworks", 3),
        Skill::new("django", "Django", "Frameworks", 2),
        Skill::new("rails", "Rails", "Frameworks", 2),
        Skill::new("spring", "Spring", "Frameworks", 1),
        // Infrastructure
        Skill::new("docker", "Docker", "Infrastructure", 3),
        Skill::new("k8s", "Kubernetes", "Infrastructure", 2),
        Skill::new("terraform", "Terraform", "Infrastructure", 2),
        Skill::new("aws", "AWS", "Infrastructure", 3),
        Skill::new("gcp", "GCP", "Infrastructure", 1),
        // Databases
        Skill::new("postgres", "PostgreSQL", "Databases", 4),
        Skill::new("redis", "Redis", "Databases", 3),
        Skill::new("sqlite", "SQLite", "Databases", 3),
        Skill::new("mongo", "MongoDB", "Databases", 2),
    ]
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[component]
fn App() -> Element {
    let config = TagInputGroupConfig {
        available_tags: skill_data(),
        initial_selected: vec![
            Skill::new("rust", "Rust", "Languages", 4),
            Skill::new("dioxus", "Dioxus", "Frameworks", 3),
        ],
        filter: None,
        sort_items: Some(|a: &Skill, b: &Skill| b.level.cmp(&a.level)),
        sort_groups: Some(|a: &str, b: &str| a.cmp(b)),
        max_items_per_group: Some(3),
        value: None,
        query: None,
        open: None,
    };
    let mut state = use_tag_input_grouped(config);

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-lg space-y-6",

                div {
                    h1 {
                        class: "text-2xl font-bold text-slate-50",
                        "Grouped Skills"
                    }
                    p {
                        class: "text-sm text-slate-400 mt-1",
                        "Suggestions grouped by category, sorted by proficiency, max 3 per group."
                    }
                }

                div {
                    class: "rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                    div {
                        class: "relative",

                        // Input area with pills
                        div {
                            class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-sky-500 focus-within:ring-1 focus-within:ring-sky-500/50 transition-all motion-reduce:transition-none",

                            for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                                {
                                    let is_pill_active = (*state.active_pill.read()) == Some(i);
                                    let pill_ring = if is_pill_active { "ring-2 ring-sky-400" } else { "" };
                                    rsx! {
                                        span {
                                            key: "{tag.id}",
                                            id: state.pill_id(i),
                                            class: "inline-flex items-center gap-1 rounded-lg bg-sky-600/30 border border-sky-500/40 px-2.5 py-0.5 text-sm text-sky-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                            "{tag.name}"
                                            button {
                                                r#type: "button",
                                                class: "ml-0.5 rounded hover:bg-sky-500/30 px-1 transition-colors motion-reduce:transition-none",
                                                onmousedown: move |evt: Event<MouseData>| {
                                                    evt.prevent_default();
                                                    evt.stop_propagation();
                                                    state.remove_tag(&tag.id);
                                                },
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
                                placeholder: "Search skills\u{2026}",
                                value: "{state.search_query}",
                                oninput: move |evt| state.set_query(evt.value()),
                                onkeydown: move |evt| state.handle_keydown(evt),
                                onclick: move |_| state.handle_click(),
                                onfocus: move |_| state.is_dropdown_open.set(true),
                                onblur: move |_| state.close_dropdown(),
                                onpaste: move |evt: Event<ClipboardData>| {
                                    if let Some(text) = dioxus_nox_tag_input::extract_clipboard_text(&evt) {
                                        evt.prevent_default();
                                        state.handle_paste(text);
                                    }
                                },
                            }
                        }

                        // Grouped dropdown
                        if *state.is_dropdown_open.read() && !state.filtered_suggestions.read().is_empty() {
                            div {
                                id: state.listbox_id(),
                                role: "listbox",
                                aria_multiselectable: "true",
                                class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg overflow-hidden max-h-80 overflow-y-auto",

                                {
                                    let groups = state.grouped_suggestions.read();
                                    let query = state.search_query.read().clone();
                                    let mut flat_idx = 0usize;

                                    rsx! {
                                        for group in groups.iter() {
                                            // Group header
                                            if !group.label.is_empty() {
                                                div {
                                                    class: "px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider text-slate-500",
                                                    "{group.label}"
                                                }
                                            }

                                            // Group items
                                            for item in group.items.iter() {
                                                {
                                                    let i = flat_idx;
                                                    flat_idx += 1;
                                                    let item_clone = item.clone();
                                                    let is_active = *state.highlighted_index.read() == Some(i);
                                                    let bg = if is_active { "bg-sky-600/80 text-white" } else { "" };
                                                    let ranges = find_match_ranges(item.name(), &query);
                                                    rsx! {
                                                        div {
                                                            key: "{item.id()}",
                                                            id: state.suggestion_id(i),
                                                            role: "option",
                                                            aria_selected: if is_active { "true" } else { "false" },
                                                            class: "flex items-center justify-between px-3 py-2 text-sm cursor-pointer transition-colors motion-reduce:transition-none hover:bg-slate-700 {bg}",
                                                            onmouseenter: move |_| state.highlighted_index.set(Some(i)),
                                                            onmousedown: move |evt: Event<MouseData>| {
                                                                evt.prevent_default();
                                                                state.add_tag(item_clone.clone());
                                                            },
                                                            span {
                                                                HighlightedText { text: item.name().to_string(), ranges: ranges }
                                                            }
                                                            LevelDots { level: item.level }
                                                        }
                                                    }
                                                }
                                            }

                                            // "and X more..." indicator
                                            if group.total_count > group.items.len() {
                                                div {
                                                    class: "px-3 py-1 text-xs text-slate-500 italic",
                                                    "and {group.total_count - group.items.len()} more\u{2026}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Screen reader live region for suggestion count announcements
                    div {
                        role: "status",
                        aria_live: "polite",
                        class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                        "{state.status_message}"
                    }

                    // Keyboard hints
                    p {
                        class: "mt-3 text-xs text-slate-500",
                        span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2191}\u{2193}" }
                        "navigate  "
                        span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
                        "pills  "
                        span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                        "select  "
                        span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Esc" }
                        "close"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Highlight component — renders text with <mark> around matched ranges
// ---------------------------------------------------------------------------

#[component]
fn HighlightedText(text: String, ranges: Vec<(usize, usize)>) -> Element {
    if ranges.is_empty() {
        return rsx! { "{text}" };
    }

    let mut parts: Vec<(String, bool)> = Vec::new();
    let mut cursor = 0;

    for (start, end) in &ranges {
        if cursor < *start {
            parts.push((text[cursor..*start].to_string(), false));
        }
        parts.push((text[*start..*end].to_string(), true));
        cursor = *end;
    }
    if cursor < text.len() {
        parts.push((text[cursor..].to_string(), false));
    }

    rsx! {
        for (i, (segment, matched)) in parts.iter().enumerate() {
            if *matched {
                mark {
                    key: "{i}",
                    class: "bg-sky-400/30 text-sky-100 rounded-sm px-0.5",
                    "{segment}"
                }
            } else {
                span {
                    key: "{i}",
                    "{segment}"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level dots — visual proficiency indicator
// ---------------------------------------------------------------------------

#[component]
fn LevelDots(level: u8) -> Element {
    rsx! {
        span {
            class: "flex gap-0.5",
            for i in 1u8..=5 {
                span {
                    key: "{i}",
                    class: if i <= level { "w-1.5 h-1.5 rounded-full bg-sky-400" } else { "w-1.5 h-1.5 rounded-full bg-slate-600" },
                }
            }
        }
    }
}
