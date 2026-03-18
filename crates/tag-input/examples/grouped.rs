use dioxus::document::Stylesheet;
use dioxus::events::ClipboardData;
use dioxus::prelude::*;
use dioxus_nox_select::{AutoComplete, SelectContext, select};
use dioxus_nox_tag_input::{TagInputGroupConfig, TagInputState, TagLike, use_tag_input_grouped};

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

/// Group skills by category for the dropdown.
fn skills_by_category(skills: &[Skill]) -> Vec<(&str, Vec<&Skill>)> {
    let categories = ["Languages", "Frameworks", "Infrastructure", "Databases"];
    categories
        .iter()
        .filter_map(|&cat| {
            let items: Vec<&Skill> = skills.iter().filter(|s| s.category == cat).collect();
            if items.is_empty() {
                None
            } else {
                Some((cat, items))
            }
        })
        .collect()
}

/// Bridge component that syncs select context values with tag-input state.
#[component]
fn SelectTagBridge(available: Vec<Skill>, children: Element) -> Element {
    let mut state = use_context::<TagInputState<Skill>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Select -> TagInput: when select adds a value, add the tag
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = state
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val) {
                if let Some(tag) = available.iter().find(|t| t.id() == val.as_str()) {
                    state.add_tag(tag.clone());
                }
            }
        }
    });

    // TagInput -> Select: when a tag is removed, deselect in select context
    use_effect(move || {
        let tag_ids: Vec<String> = state
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &select_ctx.current_values_peek() {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[component]
fn App() -> Element {
    let skills = skill_data();
    let config = TagInputGroupConfig {
        available_tags: skills.clone(),
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
    };
    let mut state = use_tag_input_grouped(config);

    // Provide TagInputState as context so the bridge can access it
    use_context_provider(|| state);

    let grouped = skills_by_category(&skills);

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
                        "Tags grouped by category, sorted by proficiency, max 3 per group."
                    }
                }

                div {
                    class: "rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                    select::Root {
                        multiple: true,
                        autocomplete: AutoComplete::List,
                        open_on_focus: true,
                        class: "relative",

                        SelectTagBridge { available: skills.clone(),

                            div {
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
                                                            state.remove_tag(tag.id());
                                                        },
                                                        "\u{00D7}"
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    ComboInput { state }
                                }

                                // Grouped dropdown
                                select::Content {
                                    class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
                                    select::Empty {
                                        class: "px-3 py-2 text-sm text-slate-500",
                                        "No results found."
                                    }
                                    for (category, skills_in_cat) in &grouped {
                                        select::Group {
                                            id: "{category}",
                                            label: category.to_string(),
                                            class: "py-1",
                                            for skill in skills_in_cat {
                                                select::Item {
                                                    value: "{skill.id()}",
                                                    label: skill.name().to_string(),
                                                    class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-sky-600/30 data-[state=checked]:text-sky-300",
                                                    "{skill.name()}"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Screen reader live region for announcements
                                div {
                                    role: "status",
                                    aria_live: "polite",
                                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                                    "{state.status_message}"
                                }

                                // Keyboard hints
                                p {
                                    class: "mt-3 text-xs text-slate-500",
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
        }
    }
}

/// Combobox-style input that wires keyboard/mouse events to both
/// tag-input state and select context.
#[component]
fn ComboInput(mut state: TagInputState<Skill>) -> Element {
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
            placeholder: "Search skills\u{2026}",
            value: "{state.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt| {
                let val = evt.value();
                state.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt| {
                match evt.key() {
                    Key::ArrowDown => {
                        evt.prevent_default();
                        if !select_ctx.is_open() {
                            select_ctx.set_open(true);
                            select_ctx.highlight_first();
                        } else {
                            select_ctx.highlight_next();
                        }
                    }
                    Key::ArrowUp => {
                        if select_ctx.is_open() {
                            evt.prevent_default();
                            select_ctx.highlight_prev();
                        }
                    }
                    Key::Enter => {
                        evt.prevent_default();
                        if select_ctx.is_open() && select_ctx.has_highlighted() {
                            select_ctx.confirm_highlighted();
                            state.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            state.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        state.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        state.handle_input_keydown(evt);
                    }
                }
            },
            onfocus: move |_| {
                if select_ctx.open_on_focus() {
                    select_ctx.set_open(true);
                }
            },
            onblur: move |_| {
                select_ctx.set_open(false);
            },
            onclick: move |_| {
                state.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = dioxus_nox_tag_input::extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    state.handle_paste(text);
                }
            },
        }
    }
}
