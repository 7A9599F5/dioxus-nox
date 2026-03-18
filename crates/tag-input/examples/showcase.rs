use std::cmp::Ordering;

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_select::{AutoComplete, SelectContext, select};
use dioxus_nox_tag_input::{
    Tag, TagInputState, TagLike, combo, components as tag_input, extract_clipboard_text,
};

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
                        KeyHint { keys: "\u{2190}\u{2192}", label: "pills" }
                        KeyHint { keys: "\u{2191}\u{2193}", label: "dropdown" }
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
const ITEM_CLS: &str = "px-3 py-2 text-sm text-slate-700 dark:text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-100 dark:data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-600 dark:data-[state=checked]:text-indigo-300";

// ---------------------------------------------------------------------------
// 1. Basic Section — uses combo module (Track 1)
// ---------------------------------------------------------------------------

#[component]
fn BasicSection(on_event: Callback<String>) -> Element {
    let fruits = fruit_tags();

    rsx! {
        Card {
            title: "Basic",
            description: "Locked tags, max 4 tags, add/remove callbacks. Dropdown via combo module.",

            combo::Root::<FruitTag> {
                available_tags: fruits.clone(),
                initial_selected: vec![FruitTag::locked("cherry", "Cherry")],
                max_tags: Some(4),
                on_add: move |tag: FruitTag| on_event.call(format!("Basic: added {}", tag.name())),
                on_remove: move |tag: FruitTag| on_event.call(format!("Basic: removed {}", tag.name())),

                BasicUI { fruits: fruits }
            }
        }
    }
}

#[component]
fn BasicUI(fruits: Vec<FruitTag>) -> Element {
    let ctx = use_context::<TagInputState<FruitTag>>();
    rsx! {
        div { class: "relative",
            combo::Control::<FruitTag> { class: CONTROL_CLS,
                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let is_locked = tag.is_locked();
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            combo::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: PILL_CLS,
                                "{name}"
                                if is_locked {
                                    span { class: "ml-0.5 text-indigo-400/50 text-xs", "\u{1F512}" }
                                } else {
                                    combo::TagRemove { tag: tag.clone(), class: REMOVE_CLS }
                                }
                            }
                        }
                    }
                }

                if *ctx.is_at_limit.read() {
                    span { class: "text-xs text-amber-600 dark:text-amber-400 ml-1", "(limit)" }
                }

                combo::Input::<FruitTag> { class: INPUT_CLS, placeholder: "Add a fruit\u{2026}".to_string() }
            }

            combo::Dropdown { class: DROPDOWN_CLS,
                select::Empty { class: "px-3 py-2 text-sm text-slate-400 dark:text-slate-500", "No fruits found." }
                for tag in &fruits {
                    select::Item {
                        value: "{tag.id()}",
                        label: tag.name().to_string(),
                        class: ITEM_CLS,
                        "{tag.name()}"
                    }
                }
            }
        }
        combo::LiveRegion::<FruitTag> {}
    }
}

// ---------------------------------------------------------------------------
// 2. Creatable Section — Track 2 (manual select::Root inside tag_input::Root)
// ---------------------------------------------------------------------------

#[component]
fn CreatableSection(on_event: Callback<String>) -> Element {
    let available = vec![
        Tag::new("work", "Work"),
        Tag::new("personal", "Personal"),
        Tag::new("urgent", "Urgent"),
    ];

    rsx! {
        Card {
            title: "Creatable",
            description: "Type + Enter to create. Comma delimiter. Paste splitting.",

            tag_input::Root::<Tag> {
                available_tags: available.clone(),
                on_create: Callback::new(move |text: String| -> Option<Tag> {
                    on_event.call(format!("Creatable: created \"{}\"", text));
                    Some(Tag::new(next_id(), text))
                }),
                paste_delimiters: Some(vec![',', '\n', '\t']),
                delimiters: Some(vec![',']),
                on_add: move |tag: Tag| on_event.call(format!("Creatable: added {}", tag.name())),

                select::Root {
                    multiple: true,
                    autocomplete: AutoComplete::List,

                    CreatableUI { available: available }
                }
            }
        }
    }
}

#[component]
fn CreatableUI(available: Vec<Tag>) -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let query = ctx.search_query.read().clone();
    let available_for_effect = available.clone();

    // Sync select -> tag-input
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = available_for_effect.iter().find(|t| t.id() == val.as_str())
            {
                ctx.add_tag(tag.clone());
            }
        }
    });

    // Sync tag-input -> select (reverse)
    use_effect(move || {
        let tag_ids: Vec<String> = ctx
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
                CreatableComboInput {}
            }

            select::Content { class: DROPDOWN_CLS,
                select::Empty { class: "px-3 py-2 text-sm text-slate-400 dark:text-slate-500", "No matches." }
                for tag in &available {
                    select::Item {
                        value: "{tag.id()}",
                        label: tag.name().to_string(),
                        class: ITEM_CLS,
                        "{tag.name()}"
                    }
                }
            }

            // "Press Enter to create" hint when query is non-empty and dropdown is closed or no match
            if !query.is_empty() {
                div {
                    class: "mt-1 text-xs text-slate-500 dark:text-slate-400",
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

#[component]
fn CreatableComboInput() -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    use_hook(|| {
        select_ctx.mark_has_input();
    });

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: INPUT_CLS,
            placeholder: "Add a tag\u{2026}",
            value: "{ctx.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt: Event<KeyboardData>| {
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
                            ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            // Delegate to tag-input for on_create / delimiter handling
                            ctx.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        ctx.handle_input_keydown(evt);
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
                ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    ctx.handle_paste(text);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Grouped Section — Track 2 with select::Group
// ---------------------------------------------------------------------------

#[component]
fn GroupedSection(on_event: Callback<String>) -> Element {
    rsx! {
        Card {
            title: "Grouped",
            description: "Tag management with grouped dropdown categories.",

            tag_input::Root::<Skill> {
                available_tags: skill_data(),
                initial_selected: vec![Skill::new("rust", "Rust", "Languages")],
                sort_items: Some(sort_skills_by_name as fn(&Skill, &Skill) -> Ordering),
                sort_groups: Some(sort_str_asc as fn(&str, &str) -> Ordering),
                max_items_per_group: Some(4),
                on_add: move |tag: Skill| on_event.call(format!("Grouped: added {}", tag.name())),
                on_remove: move |tag: Skill| on_event.call(format!("Grouped: removed {}", tag.name())),

                select::Root {
                    multiple: true,
                    autocomplete: AutoComplete::List,

                    GroupedUI {}
                }
            }
        }
    }
}

#[component]
fn GroupedUI() -> Element {
    let mut ctx = use_context::<TagInputState<Skill>>();
    let mut select_ctx = use_context::<SelectContext>();
    let skills = skill_data();

    // Sync select -> tag-input
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = skills.iter().find(|t| t.id() == val.as_str())
            {
                ctx.add_tag(tag.clone());
            }
        }
    });

    // Reverse sync
    use_effect(move || {
        let tag_ids: Vec<String> = ctx
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
                GroupedComboInput {}
            }

            select::Content { class: DROPDOWN_CLS,
                select::Empty { class: "px-3 py-2 text-sm text-slate-400 dark:text-slate-500", "No skills found." }

                select::Group { id: "databases".to_string(),
                    select::Label { class: "px-3 py-1.5 text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider", "Databases" }
                    select::Item { value: "postgres", label: "PostgreSQL".to_string(), class: ITEM_CLS, "PostgreSQL" }
                    select::Item { value: "redis", label: "Redis".to_string(), class: ITEM_CLS, "Redis" }
                    select::Item { value: "sqlite", label: "SQLite".to_string(), class: ITEM_CLS, "SQLite" }
                }
                select::Group { id: "frameworks".to_string(),
                    select::Label { class: "px-3 py-1.5 text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider", "Frameworks" }
                    select::Item { value: "react", label: "React".to_string(), class: ITEM_CLS, "React" }
                    select::Item { value: "dioxus", label: "Dioxus".to_string(), class: ITEM_CLS, "Dioxus" }
                    select::Item { value: "nextjs", label: "Next.js".to_string(), class: ITEM_CLS, "Next.js" }
                    select::Item { value: "django", label: "Django".to_string(), class: ITEM_CLS, "Django" }
                }
                select::Group { id: "infrastructure".to_string(),
                    select::Label { class: "px-3 py-1.5 text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider", "Infrastructure" }
                    select::Item { value: "docker", label: "Docker".to_string(), class: ITEM_CLS, "Docker" }
                    select::Item { value: "k8s", label: "Kubernetes".to_string(), class: ITEM_CLS, "Kubernetes" }
                    select::Item { value: "aws", label: "AWS".to_string(), class: ITEM_CLS, "AWS" }
                }
                select::Group { id: "languages".to_string(),
                    select::Label { class: "px-3 py-1.5 text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider", "Languages" }
                    select::Item { value: "rust", label: "Rust".to_string(), class: ITEM_CLS, "Rust" }
                    select::Item { value: "ts", label: "TypeScript".to_string(), class: ITEM_CLS, "TypeScript" }
                    select::Item { value: "python", label: "Python".to_string(), class: ITEM_CLS, "Python" }
                    select::Item { value: "go", label: "Go".to_string(), class: ITEM_CLS, "Go" }
                }
            }
        }
        tag_input::LiveRegion::<Skill> {}
    }
}

#[component]
fn GroupedComboInput() -> Element {
    let mut ctx = use_context::<TagInputState<Skill>>();
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    use_hook(|| {
        select_ctx.mark_has_input();
    });

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: INPUT_CLS,
            placeholder: "Search skills\u{2026}",
            value: "{ctx.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt: Event<KeyboardData>| {
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
                            ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            ctx.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        ctx.handle_input_keydown(evt);
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
                ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    ctx.handle_paste(text);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Advanced Section — Track 2 with dropdown
// ---------------------------------------------------------------------------

#[component]
fn AdvancedSection(on_event: Callback<String>) -> Element {
    let langs = language_tags();

    rsx! {
        Card {
            title: "Advanced",
            description: "Overflow badge, deny list (PHP), sorted. Dropdown for selection.",

            tag_input::Root::<Tag> {
                available_tags: langs.clone(),
                max_tag_length: Some(12),
                max_visible_tags: Some(3),
                deny_list: Some(vec!["php".to_string()]),
                sort_selected: Some(sort_tags_by_name as fn(&Tag, &Tag) -> Ordering),
                on_add: move |tag: Tag| on_event.call(format!("Advanced: added {}", tag.name())),
                on_remove: move |tag: Tag| on_event.call(format!("Advanced: removed {}", tag.name())),

                select::Root {
                    multiple: true,
                    autocomplete: AutoComplete::List,

                    AdvancedUI { langs: langs }
                }
            }
        }
    }
}

#[component]
fn AdvancedUI(langs: Vec<Tag>) -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let langs_for_effect = langs.clone();

    // Sync select -> tag-input
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = langs_for_effect.iter().find(|t| t.id() == val.as_str())
            {
                ctx.add_tag(tag.clone());
            }
        }
    });

    // Reverse sync
    use_effect(move || {
        let tag_ids: Vec<String> = ctx
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

                AdvancedComboInput {}
            }

            if let Some(ref err) = *ctx.validation_error.read() {
                div {
                    class: "mt-1 text-xs text-red-600 dark:text-red-400",
                    "{err}"
                }
            }

            select::Content { class: DROPDOWN_CLS,
                select::Empty { class: "px-3 py-2 text-sm text-slate-400 dark:text-slate-500", "No languages found." }
                for tag in &langs {
                    select::Item {
                        value: "{tag.id}",
                        label: tag.name.to_string(),
                        class: ITEM_CLS,
                        "{tag.name}"
                    }
                }
            }
        }

        // Info panel
        div {
            class: "mt-2 rounded-lg bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 px-3 py-2 text-xs text-slate-500 dark:text-slate-400 space-y-0.5",
            div { "Selected: {ctx.selected_tags.read().len()} \u{2022} Visible: {ctx.visible_tags.read().len()} + {ctx.overflow_count.read()} overflow" }
            div { "Form value: {ctx.form_value}" }
        }

        tag_input::LiveRegion::<Tag> {}
    }
}

#[component]
fn AdvancedComboInput() -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    use_hook(|| {
        select_ctx.mark_has_input();
    });

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: INPUT_CLS,
            placeholder: "Search languages\u{2026}",
            value: "{ctx.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt: Event<KeyboardData>| {
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
                            ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            ctx.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        ctx.handle_input_keydown(evt);
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
                ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    ctx.handle_paste(text);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Controlled Section — Track 2 with dropdown
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

                        select::Root {
                            multiple: true,
                            autocomplete: AutoComplete::List,

                            ControlledUI { colors: color_tags() }
                        }
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

                        select::Root {
                            multiple: true,
                            autocomplete: AutoComplete::List,

                            ControlledUI { colors: color_tags() }
                        }
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
fn ControlledUI(colors: Vec<Tag>) -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let colors_for_effect = colors.clone();

    // Sync select -> tag-input
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = colors_for_effect.iter().find(|t| t.id() == val.as_str())
            {
                ctx.add_tag(tag.clone());
            }
        }
    });

    // Reverse sync
    use_effect(move || {
        let tag_ids: Vec<String> = ctx
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
                ControlledComboInput {}
            }

            select::Content { class: DROPDOWN_CLS,
                select::Empty { class: "px-3 py-2 text-sm text-slate-400 dark:text-slate-500", "No colors found." }
                for tag in &colors {
                    select::Item {
                        value: "{tag.id}",
                        label: tag.name.to_string(),
                        class: ITEM_CLS,
                        "{tag.name}"
                    }
                }
            }
        }
        tag_input::LiveRegion::<Tag> {}
    }
}

#[component]
fn ControlledComboInput() -> Element {
    let mut ctx = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    use_hook(|| {
        select_ctx.mark_has_input();
    });

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: INPUT_CLS,
            placeholder: "Pick colors\u{2026}",
            value: "{ctx.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt: Event<KeyboardData>| {
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
                            ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            ctx.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        ctx.handle_input_keydown(evt);
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
                ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    ctx.handle_paste(text);
                }
            },
        }
    }
}
