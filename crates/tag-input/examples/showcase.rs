use std::cmp::Ordering;

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{components as tag_input, find_match_ranges, Tag, TagInputState, TagLike};

fn main() {
    dioxus::launch(App);
}

// ---------------------------------------------------------------------------
// Tag types
// ---------------------------------------------------------------------------

/// Tag with lock support (Basic section).
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

/// Tag with group/category support (Grouped section).
#[derive(Clone, PartialEq, Debug)]
struct Skill {
    id: String,
    name: String,
    category: String,
}

impl Skill {
    fn new(id: &str, name: &str, category: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
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

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

fn fruit_tags() -> Vec<FruitTag> {
    vec![
        FruitTag::new("apple", "Apple"),
        FruitTag::new("banana", "Banana"),
        FruitTag::new("cherry", "Cherry"),
        FruitTag::new("grape", "Grape"),
        FruitTag::new("mango", "Mango"),
        FruitTag::new("orange", "Orange"),
        FruitTag::new("peach", "Peach"),
        FruitTag::new("pear", "Pear"),
    ]
}

fn skill_data() -> Vec<Skill> {
    vec![
        Skill::new("rust", "Rust", "Languages"),
        Skill::new("ts", "TypeScript", "Languages"),
        Skill::new("python", "Python", "Languages"),
        Skill::new("go", "Go", "Languages"),
        Skill::new("react", "React", "Frameworks"),
        Skill::new("dioxus", "Dioxus", "Frameworks"),
        Skill::new("nextjs", "Next.js", "Frameworks"),
        Skill::new("django", "Django", "Frameworks"),
        Skill::new("docker", "Docker", "Infrastructure"),
        Skill::new("k8s", "Kubernetes", "Infrastructure"),
        Skill::new("aws", "AWS", "Infrastructure"),
        Skill::new("postgres", "PostgreSQL", "Databases"),
        Skill::new("redis", "Redis", "Databases"),
        Skill::new("sqlite", "SQLite", "Databases"),
    ]
}

fn language_tags() -> Vec<Tag> {
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

fn color_tags() -> Vec<Tag> {
    vec![
        Tag::new("red", "Red"),
        Tag::new("blue", "Blue"),
        Tag::new("green", "Green"),
        Tag::new("yellow", "Yellow"),
        Tag::new("purple", "Purple"),
        Tag::new("orange", "Orange"),
        Tag::new("pink", "Pink"),
        Tag::new("teal", "Teal"),
    ]
}

// ID counter for creatable tags.
static NEXT_ID: GlobalSignal<u32> = Signal::global(|| 1000);

fn next_id() -> String {
    let id = *NEXT_ID.read();
    *NEXT_ID.write() += 1;
    format!("created-{id}")
}

/// Simulated async search.
fn search_languages(query: &str) -> Vec<Tag> {
    let q = query.to_lowercase();
    language_tags()
        .into_iter()
        .filter(|t| t.name().to_lowercase().contains(&q))
        .collect()
}

// fn-pointer helpers (closures can't convert through Dioxus SuperInto).
fn sort_skills_by_name(a: &Skill, b: &Skill) -> Ordering {
    a.name().cmp(b.name())
}
fn sort_str_asc(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}
fn sort_tags_by_name(a: &Tag, b: &Tag) -> Ordering {
    a.name().cmp(b.name())
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[component]
fn App() -> Element {
    let mut dark = use_signal(|| true);
    let mut event_log = use_signal(Vec::<String>::new);
    let dark_class = if *dark.read() { "dark" } else { "" };

    let log_event = move |msg: String| {
        let mut log = event_log.write();
        log.push(msg);
        let len = log.len();
        if len > 50 {
            log.drain(0..len - 50);
        }
    };

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "{dark_class}",

            div {
                class: "min-h-screen bg-slate-100 dark:bg-slate-950 text-slate-900 dark:text-slate-100 transition-colors motion-reduce:transition-none",

                // Header
                div {
                    class: "border-b border-slate-200 dark:border-slate-800 bg-white/80 dark:bg-slate-900/80 backdrop-blur-sm sticky top-0 z-50",

                    div {
                        class: "max-w-6xl mx-auto px-4 sm:px-6 py-4 flex items-center justify-between",

                        div {
                            h1 {
                                class: "text-2xl font-bold tracking-tight bg-gradient-to-r from-indigo-500 to-violet-500 bg-clip-text text-transparent",
                                "dioxus-tag-input"
                            }
                            p {
                                class: "text-sm text-slate-500 dark:text-slate-400 mt-0.5",
                                "Headless tag input for Dioxus 0.7 \u{2014} compound components showcase"
                            }
                        }

                        button {
                            class: "rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-1.5 text-sm font-medium hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors motion-reduce:transition-none",
                            "data-testid": "theme-toggle",
                            onclick: move |_| dark.toggle(),
                            if *dark.read() { "\u{2600}\u{FE0F} Light" } else { "\u{1F319} Dark" }
                        }
                    }
                }

                // Main content
                div {
                    class: "max-w-6xl mx-auto px-4 sm:px-6 py-8",

                    div {
                        class: "grid grid-cols-1 sm:grid-cols-2 gap-6",

                        BasicSection { on_event: log_event }
                        CreatableSection { on_event: log_event }
                        GroupedSection { on_event: log_event }
                        AsyncSection { on_event: log_event }
                        AdvancedSection { on_event: log_event }
                        ControlledSection { on_event: log_event }
                    }

                    // Event log
                    div {
                        class: "mt-8 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4",
                        h3 {
                            class: "text-sm font-semibold text-slate-500 dark:text-slate-400 mb-2",
                            "Event Log"
                        }
                        div {
                            class: "h-32 overflow-y-auto font-mono text-xs space-y-0.5 text-slate-600 dark:text-slate-400 scrollbar-none",
                            "data-testid": "event-log",
                            if event_log.read().is_empty() {
                                span { class: "text-slate-400 dark:text-slate-600 italic", "Interact with any section above\u{2026}" }
                            }
                            for (i, entry) in event_log.read().iter().enumerate().rev() {
                                div {
                                    key: "{i}",
                                    span { class: "text-slate-400 dark:text-slate-600 mr-2 select-none", "{i + 1}." }
                                    "{entry}"
                                }
                            }
                        }
                    }

                    // Keyboard hints
                    div {
                        class: "mt-4 flex flex-wrap gap-x-4 gap-y-1 text-xs text-slate-500 dark:text-slate-400",
                        KeyHint { keys: "\u{2191}\u{2193}", label: "navigate" }
                        KeyHint { keys: "\u{2190}\u{2192}", label: "pills" }
                        KeyHint { keys: "Enter", label: "select" }
                        KeyHint { keys: "Tab", label: "autocomplete" }
                        KeyHint { keys: "Bksp", label: "remove" }
                        KeyHint { keys: "Esc", label: "close" }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared sub-components
// ---------------------------------------------------------------------------

#[component]
fn KeyHint(keys: String, label: String) -> Element {
    rsx! {
        span {
            span {
                class: "font-mono bg-slate-200 dark:bg-slate-800 rounded px-1 py-0.5 mr-1 text-slate-700 dark:text-slate-300",
                "{keys}"
            }
            "{label}"
        }
    }
}

#[component]
fn Card(title: String, description: String, children: Element) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-5 space-y-3",
            div {
                h2 { class: "text-lg font-semibold text-slate-900 dark:text-slate-100", "{title}" }
                p { class: "text-sm text-slate-500 dark:text-slate-400 mt-0.5", "{description}" }
            }
            {children}
        }
    }
}

// Shared Tailwind class constants for consistency.
const CONTROL_CLS: &str = "flex flex-wrap items-center gap-2 rounded-xl border border-slate-300 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none";
const PILL_CLS: &str = "inline-flex items-center gap-1 rounded-lg bg-indigo-100 dark:bg-indigo-600/30 border border-indigo-200 dark:border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-700 dark:text-indigo-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-slate-900";
const REMOVE_CLS: &str = "ml-0.5 rounded hover:bg-indigo-200 dark:hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none text-indigo-400 dark:text-indigo-300";
const INPUT_CLS: &str = "flex-1 min-w-[100px] bg-transparent outline-none text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 text-sm";
const DROPDOWN_CLS: &str = "absolute z-50 mt-1 w-full rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-lg max-h-60 overflow-y-auto";
const OPTION_CLS: &str = "px-3 py-2 text-sm cursor-pointer transition-colors motion-reduce:transition-none hover:bg-slate-100 dark:hover:bg-slate-800";

// ---------------------------------------------------------------------------
// 1. Basic Section
// ---------------------------------------------------------------------------

#[component]
fn BasicSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Basic",
            description: "Locked tags, max 4 tags, add/remove callbacks.",

            tag_input::Root::<FruitTag> {
                available_tags: fruit_tags(),
                initial_selected: vec![FruitTag::locked("cherry", "Cherry")],
                max_tags: Some(4),
                on_add: move |tag: FruitTag| on_event.call(format!("Basic: added {}", tag.name())),
                on_remove: move |tag: FruitTag| on_event.call(format!("Basic: removed {}", tag.name())),
                BasicUI {}
            }
        }
    }
}

#[component]
fn BasicUI() -> Element {
    let ctx = use_context::<TagInputState<FruitTag>>();
    rsx! {
        div { class: "relative",
            tag_input::Control::<FruitTag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let is_locked = tag.is_locked();
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                if is_locked {
                                    span { class: "ml-0.5 text-indigo-400/50 text-xs", "\u{1F512}" }
                                } else {
                                    tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                                }
                            }
                        }
                    }
                }

                if *ctx.is_at_limit.read() {
                    span { class: "text-xs text-amber-600 dark:text-amber-400 ml-1", "(limit)" }
                }

                tag_input::Input::<FruitTag> { class: INPUT_CLS, placeholder: "Add a fruit\u{2026}".to_string() }
            }

            tag_input::Dropdown::<FruitTag> { class: DROPDOWN_CLS,
                for (i, s) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = s.name().to_string();
                        rsx! {
                            tag_input::Option { key: "{s.id()}", tag: s, index: i, class: OPTION_CLS, "{name}" }
                        }
                    }
                }
            }
        }
        tag_input::LiveRegion::<FruitTag> {}
    }
}

// ---------------------------------------------------------------------------
// 2. Creatable Section
// ---------------------------------------------------------------------------

#[component]
fn CreatableSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Creatable",
            description: "Type + Enter to create. Comma delimiter. Paste splitting.",

            tag_input::Root::<Tag> {
                available_tags: vec![
                    Tag::new("work", "Work"),
                    Tag::new("personal", "Personal"),
                    Tag::new("urgent", "Urgent"),
                ],
                on_create: Callback::new(move |text: String| -> Option<Tag> {
                    on_event.call(format!("Creatable: created \"{}\"", text));
                    Some(Tag::new(next_id(), text))
                }),
                paste_delimiters: Some(vec![',', '\n', '\t']),
                delimiters: Some(vec![',']),
                on_add: move |tag: Tag| on_event.call(format!("Creatable: added {}", tag.name())),
                CreatableUI {}
            }
        }
    }
}

#[component]
fn CreatableUI() -> Element {
    let ctx = use_context::<TagInputState<Tag>>();
    let query = ctx.search_query.read().clone();
    let show_hint = !query.is_empty() && ctx.highlighted_index.read().is_none();

    rsx! {
        div { class: "relative",
            tag_input::Control::<Tag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                            }
                        }
                    }
                }
                tag_input::Input::<Tag> { class: INPUT_CLS, placeholder: "Add a tag\u{2026}".to_string() }
            }

            tag_input::Dropdown::<Tag> { class: DROPDOWN_CLS,
                for (i, s) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = s.name().to_string();
                        rsx! {
                            tag_input::Option { key: "{s.id()}", tag: s, index: i, class: OPTION_CLS, "{name}" }
                        }
                    }
                }
            }

            // "Press Enter to create" hint when dropdown is open with unmatched query
            if *ctx.is_dropdown_open.read() && show_hint {
                div {
                    class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-lg px-3 py-2 text-sm text-slate-500 dark:text-slate-400",
                    "Press "
                    span { class: "font-mono bg-slate-200 dark:bg-slate-800 rounded px-1 py-0.5", "Enter" }
                    " to create "
                    span { class: "font-semibold text-indigo-600 dark:text-indigo-300", "\"{query}\"" }
                }
            }
        }
        tag_input::LiveRegion::<Tag> {}
    }
}

// ---------------------------------------------------------------------------
// 3. Grouped Section
// ---------------------------------------------------------------------------

#[component]
fn GroupedSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Grouped",
            description: "Suggestions grouped by category with match highlighting.",

            tag_input::Root::<Skill> {
                available_tags: skill_data(),
                initial_selected: vec![Skill::new("rust", "Rust", "Languages")],
                sort_items: Some(sort_skills_by_name as fn(&Skill, &Skill) -> Ordering),
                sort_groups: Some(sort_str_asc as fn(&str, &str) -> Ordering),
                max_items_per_group: Some(4),
                on_add: move |tag: Skill| on_event.call(format!("Grouped: added {}", tag.name())),
                on_remove: move |tag: Skill| on_event.call(format!("Grouped: removed {}", tag.name())),
                GroupedUI {}
            }
        }
    }
}

#[component]
fn GroupedUI() -> Element {
    let ctx = use_context::<TagInputState<Skill>>();
    rsx! {
        div { class: "relative",
            tag_input::Control::<Skill> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                            }
                        }
                    }
                }
                tag_input::Input::<Skill> { class: INPUT_CLS, placeholder: "Search skills\u{2026}".to_string() }
            }

            // Grouped dropdown
            tag_input::Dropdown::<Skill> { class: DROPDOWN_CLS,
                {
                    let groups = ctx.grouped_suggestions.read();
                    let query = ctx.search_query.read().clone();
                    let mut flat_idx = 0usize;
                    rsx! {
                        for group in groups.iter() {
                            tag_input::DropdownGroup::<Skill> {
                                key: "{group.label}",
                                label: group.label.clone(),
                                class: "py-1",

                                // Group header
                                if !group.label.is_empty() {
                                    div {
                                        class: "px-3 pt-2 pb-1 text-xs font-semibold uppercase tracking-wider text-slate-400 dark:text-slate-500",
                                        "{group.label}"
                                    }
                                }

                                for item in group.items.iter() {
                                    {
                                        let i = flat_idx;
                                        flat_idx += 1;
                                        let ranges = find_match_ranges(item.name(), &query);
                                        rsx! {
                                            tag_input::Option {
                                                key: "{item.id()}",
                                                tag: item.clone(),
                                                index: i,
                                                class: OPTION_CLS,
                                                HighlightedText { text: item.name().to_string(), ranges: ranges }
                                            }
                                        }
                                    }
                                }

                                if group.total_count > group.items.len() {
                                    div {
                                        class: "px-3 py-1 text-xs text-slate-400 dark:text-slate-500 italic",
                                        "and {group.total_count - group.items.len()} more\u{2026}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        tag_input::LiveRegion::<Skill> {}
    }
}

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
                    class: "bg-indigo-200/60 dark:bg-indigo-400/30 text-indigo-700 dark:text-indigo-100 rounded-sm px-0.5",
                    "{segment}"
                }
            } else {
                span { key: "{i}", "{segment}" }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Async Search Section
// ---------------------------------------------------------------------------

#[component]
fn AsyncSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Async Search",
            description: "on_search callback with loading state.",

            tag_input::Root::<Tag> {
                available_tags: Vec::<Tag>::new(),
                on_search: move |query: String| on_event.call(format!("Async: searched \"{}\"", query)),
                on_add: move |tag: Tag| on_event.call(format!("Async: added {}", tag.name())),
                AsyncUI {}
            }
        }
    }
}

#[component]
fn AsyncUI() -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();

    // Wire async search: on_search sets results synchronously for demo.
    use_effect(move || {
        ctx.on_search.set(Some(Callback::new(move |query: String| {
            if query.is_empty() {
                ctx.async_suggestions.set(None);
                ctx.is_loading.set(false);
                return;
            }
            let results = search_languages(&query);
            ctx.async_suggestions.set(Some(results));
        })));
    });

    rsx! {
        div { class: "relative",
            tag_input::Control::<Tag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                            }
                        }
                    }
                }

                tag_input::Input::<Tag> { class: INPUT_CLS, placeholder: "Search languages\u{2026}".to_string() }

                if *ctx.is_loading.read() {
                    span { class: "text-xs text-indigo-500 dark:text-indigo-400 animate-pulse", "Loading\u{2026}" }
                }
            }

            tag_input::Dropdown::<Tag> { class: DROPDOWN_CLS,
                for (i, s) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = s.name().to_string();
                        rsx! {
                            tag_input::Option { key: "{s.id()}", tag: s, index: i, class: OPTION_CLS, "{name}" }
                        }
                    }
                }
            }

            if *ctx.is_dropdown_open.read() && *ctx.has_no_matches.read() && !*ctx.is_loading.read() {
                div {
                    class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-lg px-3 py-4 text-sm text-slate-400 dark:text-slate-500 text-center",
                    "No languages found."
                }
            }
        }
        tag_input::LiveRegion::<Tag> {}
    }
}

// ---------------------------------------------------------------------------
// 5. Advanced Section
// ---------------------------------------------------------------------------

#[component]
fn AdvancedSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Advanced",
            description: "Overflow badge, deny list (PHP), max 5 suggestions, autocomplete.",

            tag_input::Root::<Tag> {
                available_tags: language_tags(),
                max_tag_length: Some(12),
                max_visible_tags: Some(3),
                max_suggestions: Some(5),
                deny_list: Some(vec!["php".to_string()]),
                sort_selected: Some(sort_tags_by_name as fn(&Tag, &Tag) -> Ordering),
                on_add: move |tag: Tag| on_event.call(format!("Advanced: added {}", tag.name())),
                on_remove: move |tag: Tag| on_event.call(format!("Advanced: removed {}", tag.name())),
                AdvancedUI {}
            }
        }
    }
}

#[component]
fn AdvancedUI() -> Element {
    let ctx = use_context::<TagInputState<Tag>>();
    let shown = ctx.filtered_suggestions.read().len();
    let total = *ctx.total_filtered_count.read();

    rsx! {
        div { class: "relative",
            tag_input::Control::<Tag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                            }
                        }
                    }
                }

                tag_input::Count::<Tag> {
                    class: "rounded-lg bg-slate-200 dark:bg-slate-800 border border-slate-300 dark:border-slate-700 px-2.5 py-0.5 text-sm text-slate-600 dark:text-slate-300",
                }

                tag_input::Input::<Tag> { class: INPUT_CLS, placeholder: "Search languages\u{2026}".to_string() }
            }

            if let Some(ref err) = *ctx.validation_error.read() {
                div {
                    class: "mt-1 text-xs text-red-600 dark:text-red-400",
                    "{err}"
                }
            }

            tag_input::Dropdown::<Tag> { class: DROPDOWN_CLS,
                for (i, s) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = s.name().to_string();
                        rsx! {
                            tag_input::Option { key: "{s.id()}", tag: s, index: i, class: OPTION_CLS, "{name}" }
                        }
                    }
                }

                if shown < total {
                    div {
                        class: "px-3 py-2 text-xs text-slate-400 dark:text-slate-500 border-t border-slate-200 dark:border-slate-700",
                        "Showing {shown} of {total} \u{2014} type to refine"
                    }
                }
            }
        }

        // Info panel
        div {
            class: "mt-2 rounded-lg bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 px-3 py-2 text-xs text-slate-500 dark:text-slate-400 space-y-0.5",
            div { "Selected: {ctx.selected_tags.read().len()} \u{2022} Visible: {ctx.visible_tags.read().len()} + {ctx.overflow_count.read()} overflow" }
            div { "Form value: {ctx.form_value}" }
            if let Some(ref ac) = *ctx.auto_complete_suggestion.read() {
                div { "Autocomplete hint: {ac.name()} (press Tab)" }
            }
        }

        tag_input::LiveRegion::<Tag> {}
    }
}

// ---------------------------------------------------------------------------
// 6. Controlled Section
// ---------------------------------------------------------------------------

#[component]
fn ControlledSection(on_event: Callback<String>) -> Element {
    let mut shared_tags: Signal<Vec<Tag>> = use_signal(|| vec![Tag::new("blue", "Blue")]);
    let shared_query: Signal<String> = use_signal(String::new);
    let tag_count = shared_tags.read().len();

    rsx! {
        Card {
            title: "Controlled",
            description: "Two inputs sharing the same signal. External buttons.",

            div {
                class: "flex items-center gap-2 mb-3",
                button {
                    class: "rounded-lg bg-rose-100 dark:bg-rose-900/40 border border-rose-200 dark:border-rose-800 text-rose-700 dark:text-rose-300 hover:bg-rose-200 dark:hover:bg-rose-900/60 px-3 py-1 text-sm font-medium transition-colors motion-reduce:transition-none",
                    "data-testid": "showcase-clear",
                    onclick: move |_| {
                        shared_tags.set(vec![]);
                        on_event.call("Controlled: cleared all".to_string());
                    },
                    "Clear"
                }
                button {
                    class: "rounded-lg bg-emerald-100 dark:bg-emerald-900/40 border border-emerald-200 dark:border-emerald-800 text-emerald-700 dark:text-emerald-300 hover:bg-emerald-200 dark:hover:bg-emerald-900/60 px-3 py-1 text-sm font-medium transition-colors motion-reduce:transition-none",
                    "data-testid": "showcase-preset",
                    onclick: move |_| {
                        shared_tags.set(vec![
                            Tag::new("red", "Red"),
                            Tag::new("green", "Green"),
                            Tag::new("blue", "Blue"),
                        ]);
                        on_event.call("Controlled: preset R/G/B".to_string());
                    },
                    "Preset (R/G/B)"
                }
                {
                    let suffix = if tag_count != 1 { "s" } else { "" };
                    rsx! {
                        span {
                            class: "text-xs text-slate-500 dark:text-slate-400",
                            "{tag_count} tag{suffix}"
                        }
                    }
                }
            }

            div { class: "space-y-3",
                // Input A
                div {
                    p { class: "text-xs font-medium text-slate-500 dark:text-slate-400 mb-1", "Input A" }
                    tag_input::Root::<Tag> {
                        available_tags: color_tags(),
                        value: Some(shared_tags),
                        query: Some(shared_query),
                        on_add: move |tag: Tag| on_event.call(format!("Controlled A: added {}", tag.name())),
                        ControlledUI {}
                    }
                }

                // Input B
                div {
                    p { class: "text-xs font-medium text-slate-500 dark:text-slate-400 mb-1", "Input B" }
                    tag_input::Root::<Tag> {
                        available_tags: color_tags(),
                        value: Some(shared_tags),
                        query: Some(shared_query),
                        on_add: move |tag: Tag| on_event.call(format!("Controlled B: added {}", tag.name())),
                        ControlledUI {}
                    }
                }
            }

            // Signal readout
            div {
                class: "mt-2 rounded-lg bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 px-3 py-2 text-xs font-mono text-slate-500 dark:text-slate-400",
                span { class: "text-slate-400 dark:text-slate-600", "selected: " }
                for (i, tag) in shared_tags.read().iter().enumerate() {
                    if i > 0 { ", " }
                    span { class: "text-indigo-600 dark:text-indigo-300", "{tag.name()}" }
                }
                br {}
                span { class: "text-slate-400 dark:text-slate-600", "query: " }
                span { class: "text-amber-600 dark:text-amber-300", "\"{shared_query}\"" }
            }
        }
    }
}

#[component]
fn ControlledUI() -> Element {
    let ctx = use_context::<TagInputState<Tag>>();
    rsx! {
        div { class: "relative",
            tag_input::Control::<Tag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            tag_input::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                tag_input::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                            }
                        }
                    }
                }
                tag_input::Input::<Tag> { class: INPUT_CLS, placeholder: "Pick colors\u{2026}".to_string() }
            }

            tag_input::Dropdown::<Tag> { class: DROPDOWN_CLS,
                for (i, s) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = s.name().to_string();
                        rsx! {
                            tag_input::Option { key: "{s.id()}", tag: s, index: i, class: OPTION_CLS, "{name}" }
                        }
                    }
                }
            }
        }
        tag_input::LiveRegion::<Tag> {}
    }
}
