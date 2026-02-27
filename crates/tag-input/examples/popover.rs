use std::collections::HashMap;

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{extract_clipboard_text, use_tag_input, TagInputState, TagLike};

fn main() {
    dioxus::launch(App);
}

// ---------------------------------------------------------------------------
// Custom tag type with rich metadata for popovers
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
struct SkillTag {
    id: String,
    name: String,
    description: String,
    level: u8,
    category: String,
}

impl SkillTag {
    fn new(id: &str, name: &str, description: &str, level: u8, category: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            level,
            category: category.into(),
        }
    }

    fn level_label(&self) -> &'static str {
        match self.level {
            1 => "Beginner",
            2 => "Intermediate",
            3 => "Advanced",
            4 => "Expert",
            5 => "Master",
            _ => "Unknown",
        }
    }

    fn level_bar_width(&self) -> &'static str {
        match self.level {
            1 => "w-1/5",
            2 => "w-2/5",
            3 => "w-3/5",
            4 => "w-4/5",
            5 => "w-full",
            _ => "w-0",
        }
    }
}

impl TagLike for SkillTag {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn skill_data() -> Vec<SkillTag> {
    vec![
        SkillTag::new("rust", "Rust", "Systems programming with memory safety guarantees. Zero-cost abstractions and fearless concurrency.", 4, "Systems"),
        SkillTag::new("ts", "TypeScript", "Typed superset of JavaScript for scalable web applications.", 3, "Web"),
        SkillTag::new("python", "Python", "Versatile language for scripting, data science, and automation.", 3, "Scripting"),
        SkillTag::new("go", "Go", "Fast compiled language with built-in concurrency primitives.", 2, "Systems"),
        SkillTag::new("react", "React", "Component-based UI library with virtual DOM and hooks.", 4, "Frontend"),
        SkillTag::new("docker", "Docker", "Container platform for building, shipping, and running applications.", 3, "DevOps"),
        SkillTag::new("postgres", "PostgreSQL", "Advanced open-source relational database with JSON support.", 3, "Database"),
        SkillTag::new("redis", "Redis", "In-memory data store used as cache, message broker, and queue.", 2, "Database"),
    ]
}

// ---------------------------------------------------------------------------
// Key-value tag data for workout attributes demo
// ---------------------------------------------------------------------------

/// Key-value tag with optional lock support for required workout attributes.
#[derive(Clone, PartialEq, Debug)]
struct KvTag {
    id: String,
    name: String,
    locked: bool,
}

impl KvTag {
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

impl TagLike for KvTag {
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

fn kv_tag_data() -> Vec<KvTag> {
    vec![
        KvTag::new("bodyweight", "Body Weight"),
        KvTag::new("location", "Gym / Location"),
        KvTag::new("duration", "Duration"),
        KvTag::new("mood", "Mood"),
        KvTag::new("sleep", "Sleep Hours"),
        KvTag::new("calories", "Calories"),
    ]
}

fn kv_initial() -> Vec<KvTag> {
    vec![
        KvTag::locked("bodyweight", "Body Weight"),
        KvTag::locked("duration", "Duration"),
        KvTag::new("location", "Gym / Location"),
        KvTag::new("mood", "Mood"),
    ]
}

// ---------------------------------------------------------------------------
// Editable tag type for form popover demo
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
struct EditableTag {
    id: String,
    name: String,
    notes: String,
    priority: u8,
    is_active: bool,
    certified: bool,
}

impl EditableTag {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            notes: String::new(),
            priority: 5,
            is_active: true,
            certified: false,
        }
    }
}

impl TagLike for EditableTag {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn editable_tag_data() -> Vec<EditableTag> {
    vec![
        EditableTag::new("design", "Design"),
        EditableTag::new("backend", "Backend"),
        EditableTag::new("frontend", "Frontend"),
        EditableTag::new("devops", "DevOps"),
        EditableTag::new("data", "Data Science"),
        EditableTag::new("mobile", "Mobile"),
        EditableTag::new("security", "Security"),
        EditableTag::new("ml", "Machine Learning"),
    ]
}

fn editable_initial() -> Vec<EditableTag> {
    vec![
        EditableTag {
            id: "design".into(),
            name: "Design".into(),
            notes: "Lead designer on project".into(),
            priority: 8,
            is_active: true,
            certified: false,
        },
        EditableTag {
            id: "backend".into(),
            name: "Backend".into(),
            notes: String::new(),
            priority: 7,
            is_active: true,
            certified: true,
        },
        EditableTag {
            id: "frontend".into(),
            name: "Frontend".into(),
            notes: "React + Dioxus".into(),
            priority: 5,
            is_active: false,
            certified: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[component]
fn App() -> Element {
    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 p-6 sm:p-10",

            div {
                class: "max-w-2xl mx-auto space-y-10",

                div {
                    h1 {
                        class: "text-2xl font-bold text-slate-50",
                        "Pill Popovers"
                    }
                    p {
                        class: "text-sm text-slate-400 mt-1",
                        "Composable trigger patterns — each section combines different ways to open a popover."
                    }
                }

                ClickAndKeyboard {}
                InfoButtonAndKeyboard {}
                AllTriggers {}
                EditablePopovers {}
                KeyValuePopovers {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Section 1: Click + Keyboard (Recommended)
// ---------------------------------------------------------------------------

#[component]
fn ClickAndKeyboard() -> Element {
    let mut state = use_tag_input(
        skill_data(),
        vec![
            SkillTag::new("rust", "Rust", "Systems programming with memory safety guarantees. Zero-cost abstractions and fearless concurrency.", 4, "Systems"),
            SkillTag::new("react", "React", "Component-based UI library with virtual DOM and hooks.", 4, "Frontend"),
        ],
    );

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        SectionCard {
            title: "Click + Keyboard (Recommended)",
            subtitle: "Click any pill to toggle its popover. Enter on an active pill also works.",

            div {
                class: "relative",

                // Click-outside overlay
                if state.popover_pill.read().is_some() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| state.close_popover(),
                    }
                }

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-indigo-400" } else { "" };
                            let is_popover_open = (*state.popover_pill.read()) == Some(i);
                            rsx! {
                                div {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "relative z-50",
                                    span {
                                        class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/30 border border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-200 cursor-pointer select-none transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                        onmousedown: move |evt: Event<MouseData>| {
                                            evt.prevent_default();
                                            state.toggle_popover(i);
                                        },
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "ml-0.5 rounded hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                evt.stop_propagation();
                                                state.remove_tag(&tag.id);
                                            },
                                            "\u{00D7}"
                                        }
                                    }
                                    if is_popover_open {
                                        PopoverCard { tag: tag.clone() }
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
        }
    }
}

// ---------------------------------------------------------------------------
// Section 2: Info Button + Keyboard
// ---------------------------------------------------------------------------

#[component]
fn InfoButtonAndKeyboard() -> Element {
    let mut state = use_tag_input(
        skill_data(),
        vec![
            SkillTag::new(
                "docker",
                "Docker",
                "Container platform for building, shipping, and running applications.",
                3,
                "DevOps",
            ),
            SkillTag::new(
                "postgres",
                "PostgreSQL",
                "Advanced open-source relational database with JSON support.",
                3,
                "Database",
            ),
        ],
    );

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        SectionCard {
            title: "Info Button + Keyboard",
            subtitle: "Pill body stays inert. Use the \u{24D8} icon or Enter on an active pill.",

            div {
                class: "relative",

                // Click-outside overlay
                if state.popover_pill.read().is_some() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| state.close_popover(),
                    }
                }

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-emerald-400" } else { "" };
                            let is_popover_open = (*state.popover_pill.read()) == Some(i);
                            rsx! {
                                div {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "relative z-50",
                                    span {
                                        class: "inline-flex items-center gap-1 rounded-lg bg-emerald-600/25 border border-emerald-500/40 px-2.5 py-0.5 text-sm text-emerald-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-emerald-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "ml-0.5 rounded hover:bg-emerald-500/30 px-1 text-emerald-300/60 hover:text-emerald-200 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                state.toggle_popover(i);
                                            },
                                            "\u{24D8}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "rounded hover:bg-emerald-500/30 px-1 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                evt.stop_propagation();
                                                state.remove_tag(&tag.id);
                                            },
                                            "\u{00D7}"
                                        }
                                    }
                                    if is_popover_open {
                                        PopoverCard { tag: tag.clone() }
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
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Section 3: All Triggers
// ---------------------------------------------------------------------------

#[component]
fn AllTriggers() -> Element {
    let mut state = use_tag_input(
        skill_data(),
        vec![
            SkillTag::new(
                "ts",
                "TypeScript",
                "Typed superset of JavaScript for scalable web applications.",
                3,
                "Web",
            ),
            SkillTag::new(
                "python",
                "Python",
                "Versatile language for scripting, data science, and automation.",
                3,
                "Scripting",
            ),
            SkillTag::new(
                "redis",
                "Redis",
                "In-memory data store used as cache, message broker, and queue.",
                2,
                "Database",
            ),
        ],
    );

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        SectionCard {
            title: "All Triggers",
            subtitle: "Click pill body, \u{24D8} icon, or Enter on active pill \u{2014} all three compose without conflict.",

            div {
                class: "relative",

                // Click-outside overlay
                if state.popover_pill.read().is_some() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| state.close_popover(),
                    }
                }

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-violet-500 focus-within:ring-1 focus-within:ring-violet-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-violet-400" } else { "" };
                            let is_popover_open = (*state.popover_pill.read()) == Some(i);
                            rsx! {
                                div {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "relative z-50",
                                    span {
                                        class: "inline-flex items-center gap-1 rounded-lg bg-violet-600/25 border border-violet-500/40 px-2.5 py-0.5 text-sm text-violet-200 cursor-pointer select-none transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-violet-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                        onmousedown: move |evt: Event<MouseData>| {
                                            evt.prevent_default();
                                            state.toggle_popover(i);
                                        },
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "ml-0.5 rounded hover:bg-violet-500/30 px-1 text-violet-300/60 hover:text-violet-200 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                evt.stop_propagation();
                                                state.toggle_popover(i);
                                            },
                                            "\u{24D8}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "rounded hover:bg-violet-500/30 px-1 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                evt.stop_propagation();
                                                state.remove_tag(&tag.id);
                                            },
                                            "\u{00D7}"
                                        }
                                    }
                                    if is_popover_open {
                                        PopoverCard { tag: tag.clone() }
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
                                let bg = if is_active { "bg-violet-600/80 text-white" } else { "" };
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

            // Live region for screen reader announcements
            div {
                role: "status",
                aria_live: "polite",
                class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
            }

            // Keyboard hints
            p {
                class: "mt-3 text-xs text-slate-500",
                span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
                "pills  "
                span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                "popover  "
                span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Esc" }
                "close  "
                span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Esc Esc" }
                "deactivate"
            }
            button {
                r#type: "button",
                class: "mt-2 text-xs text-slate-400 hover:text-slate-200 transition-colors motion-reduce:transition-none",
                onclick: move |_| state.clear_all(),
                "Clear all tags"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Section 4: Editable Popovers (form inputs with optimistic save)
// ---------------------------------------------------------------------------

#[component]
fn EditablePopovers() -> Element {
    let mut state = use_tag_input(editable_tag_data(), editable_initial());

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        SectionCard {
            title: "Editable Popovers",
            subtitle: "Click a pill to edit its properties. Changes save automatically when you close the popover.",

            div {
                class: "relative",

                // Click-outside overlay
                if state.popover_pill.read().is_some() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| state.close_popover(),
                    }
                }

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-amber-500 focus-within:ring-1 focus-within:ring-amber-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-amber-400" } else { "" };
                            let is_popover_open = (*state.popover_pill.read()) == Some(i);
                            let has_edits = !tag.notes.is_empty() || tag.priority != 5 || !tag.is_active || tag.certified;
                            let pill_style = if tag.is_active {
                                "bg-amber-600/25 border-amber-500/40 text-amber-200"
                            } else {
                                "bg-slate-700/30 border-slate-600/40 text-slate-400 opacity-60"
                            };
                            let dot_color = if tag.is_active { "bg-amber-400" } else { "bg-slate-500" };
                            rsx! {
                                div {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "relative z-50",
                                    span {
                                        class: "inline-flex items-center gap-1 rounded-lg border px-2.5 py-0.5 text-sm cursor-pointer select-none transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-amber-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_style} {pill_ring}",
                                        onmousedown: move |evt: Event<MouseData>| {
                                            evt.prevent_default();
                                            state.toggle_popover(i);
                                        },
                                        if has_edits {
                                            span {
                                                class: "w-1.5 h-1.5 rounded-full {dot_color}",
                                            }
                                        }
                                        "{tag.name}"
                                        button {
                                            r#type: "button",
                                            class: "rounded hover:bg-amber-500/30 px-1 transition-colors motion-reduce:transition-none",
                                            onmousedown: move |evt: Event<MouseData>| {
                                                evt.prevent_default();
                                                evt.stop_propagation();
                                                state.remove_tag(&tag.id);
                                            },
                                            "\u{00D7}"
                                        }
                                    }
                                    if is_popover_open {
                                        EditablePopoverCard { state: state, index: i }
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
                        onblur: move |_| {
                            // Only close dropdown — popover stays open for editing
                            state.is_dropdown_open.set(false);
                            state.highlighted_index.set(None);
                        },
                        onpaste: move |evt: Event<ClipboardData>| {
                            if let Some(text) = extract_clipboard_text(&evt) {
                                evt.prevent_default();
                                state.handle_paste(text);
                            }
                        },
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
                                let bg = if is_active { "bg-amber-600/80 text-white" } else { "" };
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

            // Live data preview
            div {
                class: "mt-4 rounded-xl border border-slate-700/50 bg-slate-900/50 p-3",
                div {
                    class: "flex items-center gap-2 mb-2",
                    span {
                        class: "text-[10px] font-semibold text-slate-500 uppercase tracking-wider",
                        "Live Data"
                    }
                    span {
                        class: "h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse",
                    }
                }

                if state.selected_tags.read().is_empty() {
                    p { class: "text-xs text-slate-600 italic", "No tags selected" }
                } else {
                    div {
                        class: "space-y-1.5",
                        for tag in state.selected_tags.read().iter() {
                            div {
                                key: "{tag.id}-preview",
                                class: "text-xs font-mono text-slate-500 bg-slate-800/50 rounded-lg px-3 py-1.5",
                                span { class: "text-amber-300 font-semibold", "{tag.name}" }
                                " "
                                if !tag.notes.is_empty() {
                                    span { class: "text-slate-400", "\u{201C}{tag.notes}\u{201D}" }
                                    " "
                                }
                                span { class: "text-slate-600", "pri:" }
                                span { class: "text-slate-300", "{tag.priority}" }
                                " "
                                if tag.is_active {
                                    span { class: "text-emerald-400", "active" }
                                } else {
                                    span { class: "text-slate-600 line-through", "active" }
                                }
                                " "
                                if tag.certified {
                                    span { class: "text-amber-400", "\u{2713} certified" }
                                } else {
                                    span { class: "text-slate-600", "\u{2717} certified" }
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
fn EditablePopoverCard(mut state: TagInputState<EditableTag>, index: usize) -> Element {
    let tag = state.selected_tags.read()[index].clone();

    rsx! {
        div {
            class: "absolute z-50 top-full mt-2 left-0 w-72 rounded-xl border border-slate-700 bg-slate-800 shadow-xl p-4",

            // Header
            div {
                class: "flex items-center justify-between mb-3",
                span {
                    class: "text-sm font-semibold text-slate-100",
                    "{tag.name}"
                }
                span {
                    class: "text-[10px] rounded-full bg-amber-500/20 text-amber-300 px-2 py-0.5 font-medium",
                    "Editable"
                }
            }

            div {
                class: "space-y-3",

                // Notes — text input
                div {
                    label {
                        class: "block text-xs font-medium text-slate-400 mb-1",
                        "Notes"
                    }
                    input {
                        r#type: "text",
                        class: "w-full rounded-lg border border-slate-600 bg-slate-900 px-2.5 py-1.5 text-sm text-slate-100 placeholder-slate-500 outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/50 transition-all",
                        placeholder: "Add a note\u{2026}",
                        value: "{tag.notes}",
                        oninput: move |evt: Event<FormData>| {
                            state.selected_tags.write()[index].notes = evt.value();
                        },
                        onkeydown: move |evt: Event<KeyboardData>| {
                            if evt.key() == Key::Escape {
                                state.close_popover();
                            }
                        },
                    }
                }

                // Priority — number input with visual bar
                div {
                    label {
                        class: "block text-xs font-medium text-slate-400 mb-1",
                        "Priority"
                    }
                    div {
                        class: "flex items-center gap-2",
                        input {
                            r#type: "number",
                            class: "w-16 rounded-lg border border-slate-600 bg-slate-900 px-2 py-1.5 text-sm text-slate-100 outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/50 transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none",
                            min: "1",
                            max: "10",
                            value: "{tag.priority}",
                            oninput: move |evt: Event<FormData>| {
                                if let Ok(n) = evt.value().parse::<u8>() {
                                    state.selected_tags.write()[index].priority = n.clamp(1, 10);
                                }
                            },
                            onkeydown: move |evt: Event<KeyboardData>| {
                                if evt.key() == Key::Escape {
                                    state.close_popover();
                                }
                            },
                        }
                        div {
                            class: "flex-1 h-1.5 rounded-full bg-slate-700 overflow-hidden",
                            div {
                                class: "h-full rounded-full bg-amber-500 transition-all duration-150",
                                style: "width: {tag.priority as f32 * 10.0}%",
                            }
                        }
                        span {
                            class: "text-xs text-slate-500 tabular-nums",
                            "{tag.priority}/10"
                        }
                    }
                }

                // Active — toggle switch
                div {
                    class: "flex items-center justify-between",
                    span {
                        class: "text-xs font-medium text-slate-400",
                        "Active"
                    }
                    button {
                        r#type: "button",
                        class: if tag.is_active {
                            "relative inline-flex h-6 w-11 items-center rounded-full bg-amber-500 transition-colors cursor-pointer"
                        } else {
                            "relative inline-flex h-6 w-11 items-center rounded-full bg-slate-600 transition-colors cursor-pointer"
                        },
                        onmousedown: move |evt: Event<MouseData>| {
                            evt.prevent_default();
                        },
                        onclick: move |_| {
                            let current = state.selected_tags.read()[index].is_active;
                            state.selected_tags.write()[index].is_active = !current;
                        },
                        span {
                            class: if tag.is_active {
                                "inline-block h-4 w-4 rounded-full bg-white shadow transition-transform translate-x-6"
                            } else {
                                "inline-block h-4 w-4 rounded-full bg-white shadow transition-transform translate-x-1"
                            },
                        }
                    }
                }

                // Certified — checkbox
                div {
                    class: "flex items-center gap-2.5",
                    button {
                        r#type: "button",
                        class: if tag.certified {
                            "flex items-center justify-center h-4 w-4 rounded border border-amber-500 bg-amber-500 transition-all cursor-pointer"
                        } else {
                            "flex items-center justify-center h-4 w-4 rounded border border-slate-500 bg-slate-900 hover:border-slate-400 transition-all cursor-pointer"
                        },
                        onmousedown: move |evt: Event<MouseData>| {
                            evt.prevent_default();
                        },
                        onclick: move |_| {
                            let current = state.selected_tags.read()[index].certified;
                            state.selected_tags.write()[index].certified = !current;
                        },
                        if tag.certified {
                            span {
                                class: "text-white text-[10px] leading-none font-bold",
                                "\u{2713}"
                            }
                        }
                    }
                    span {
                        class: "text-xs font-medium text-slate-400",
                        "Certified"
                    }
                }
            }

            // Auto-save hint
            div {
                class: "mt-3 pt-2 border-t border-slate-700/50",
                p {
                    class: "text-[10px] text-slate-500 text-center",
                    "Changes save automatically"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Section 5: Key : Value Pills (workout attributes)
// ---------------------------------------------------------------------------

#[component]
fn KeyValuePopovers() -> Element {
    let mut state = use_tag_input(kv_tag_data(), kv_initial());
    let mut values: Signal<HashMap<String, String>> = use_signal(|| {
        HashMap::from([
            ("bodyweight".into(), "224 lbs".into()),
            ("location".into(), "Gold's Gym".into()),
        ])
    });

    // Clean up value entries when a tag is removed
    use_effect(move || {
        state.on_remove.set(Some(Callback::new(move |tag: KvTag| {
            values.write().remove(tag.id());
        })));
    });

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        SectionCard {
            title: "Key : Value Pills",
            subtitle: "Workout attributes \u{2014} tap a pill to set its value. Body Weight and Duration are locked (required).",

            div {
                class: "relative",

                // Click-outside overlay
                if state.popover_pill.read().is_some() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| state.close_popover(),
                    }
                }

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-teal-500 focus-within:ring-1 focus-within:ring-teal-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-teal-400" } else { "" };
                            let is_popover_open = (*state.popover_pill.read()) == Some(i);
                            let value = values.read().get(&tag.id).cloned().unwrap_or_default();
                            let has_value = !value.is_empty();
                            let pill_bg = if has_value {
                                "bg-teal-600/30 border-teal-500/40"
                            } else {
                                "bg-teal-600/15 border-teal-500/25"
                            };
                            let locked_style = if tag.is_locked() { "opacity-80" } else { "" };
                            let tag_id = tag.id.clone();
                            let tag_id_for_popover = tag.id.clone();
                            let tag_name_for_popover = tag.name.clone();
                            rsx! {
                                div {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "relative z-50",
                                    span {
                                        class: "inline-flex items-center gap-1 rounded-lg border px-2.5 py-0.5 text-sm cursor-pointer select-none transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-teal-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_bg} {pill_ring} {locked_style}",
                                        onmousedown: move |evt: Event<MouseData>| {
                                            evt.prevent_default();
                                            state.toggle_popover(i);
                                        },
                                        span {
                                            class: "text-teal-300/70 font-medium",
                                            "{tag.name}"
                                        }
                                        if has_value {
                                            span { class: "text-teal-300/40", ":" }
                                            span { class: "text-teal-100 font-semibold", " {value}" }
                                        }
                                        if tag.is_locked() {
                                            span {
                                                class: "ml-0.5 text-teal-400/50 text-xs",
                                                "\u{1F512}"
                                            }
                                        } else {
                                            button {
                                                r#type: "button",
                                                class: "ml-0.5 rounded hover:bg-teal-500/30 px-1 transition-colors motion-reduce:transition-none text-teal-300/50 hover:text-teal-200",
                                                onmousedown: move |evt: Event<MouseData>| {
                                                    evt.prevent_default();
                                                    evt.stop_propagation();
                                                    state.remove_tag(&tag_id);
                                                },
                                                "\u{00D7}"
                                            }
                                        }
                                    }
                                    if is_popover_open {
                                        KeyValuePopoverCard {
                                            tag_id: tag_id_for_popover,
                                            tag_name: tag_name_for_popover,
                                            values: values,
                                            state: state,
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
                        onblur: move |_| {
                            state.is_dropdown_open.set(false);
                            state.highlighted_index.set(None);
                        },
                        onpaste: move |evt: Event<ClipboardData>| {
                            if let Some(text) = extract_clipboard_text(&evt) {
                                evt.prevent_default();
                                state.handle_paste(text);
                            }
                        },
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
                                let bg = if is_active { "bg-teal-600/80 text-white" } else { "" };
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

            // Live data preview
            div {
                class: "mt-4 rounded-xl border border-slate-700/50 bg-slate-900/50 p-3",
                div {
                    class: "flex items-center gap-2 mb-2",
                    span {
                        class: "text-[10px] font-semibold text-slate-500 uppercase tracking-wider",
                        "Live Data"
                    }
                    span {
                        class: "h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse",
                    }
                }

                if state.selected_tags.read().is_empty() {
                    p { class: "text-xs text-slate-600 italic", "No tags selected" }
                } else {
                    div {
                        class: "space-y-1.5",
                        for tag in state.selected_tags.read().iter() {
                            {
                                let value = values.read().get(tag.id()).cloned().unwrap_or_default();
                                rsx! {
                                    div {
                                        key: "{tag.id}-preview",
                                        class: "text-xs font-mono text-slate-500 bg-slate-800/50 rounded-lg px-3 py-1.5",
                                        span { class: "text-teal-300 font-semibold", "{tag.name}" }
                                        span { class: "text-slate-600", ": " }
                                        if !value.is_empty() {
                                            span { class: "text-slate-300", "{value}" }
                                        } else {
                                            span { class: "text-slate-600 italic", "(empty)" }
                                        }
                                    }
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
fn KeyValuePopoverCard(
    tag_id: String,
    tag_name: String,
    mut values: Signal<HashMap<String, String>>,
    mut state: TagInputState<KvTag>,
) -> Element {
    let current_value = values.read().get(&tag_id).cloned().unwrap_or_default();

    rsx! {
        div {
            class: "absolute z-50 top-full mt-2 left-0 w-72 rounded-xl border border-slate-700 bg-slate-800 shadow-xl p-4",
            onmousedown: move |evt: Event<MouseData>| {
                evt.prevent_default();
            },

            // Header
            div {
                class: "flex items-center justify-between mb-3",
                span {
                    class: "text-sm font-semibold text-slate-100",
                    "{tag_name}"
                }
                span {
                    class: "text-[10px] rounded-full bg-teal-500/20 text-teal-300 px-2 py-0.5 font-medium",
                    "Key : Value"
                }
            }

            // Value input
            div {
                label {
                    class: "block text-xs font-medium text-slate-400 mb-1",
                    "Value"
                }
                input {
                    r#type: "text",
                    class: "w-full rounded-lg border border-slate-600 bg-slate-900 px-2.5 py-1.5 text-sm text-slate-100 placeholder-slate-500 outline-none focus:border-teal-500 focus:ring-1 focus:ring-teal-500/50 transition-all",
                    placeholder: "Enter value\u{2026}",
                    value: "{current_value}",
                    oninput: {
                        let tag_id = tag_id.clone();
                        move |evt: Event<FormData>| {
                            let v = evt.value();
                            if v.is_empty() {
                                values.write().remove(&tag_id);
                            } else {
                                values.write().insert(tag_id.clone(), v);
                            }
                        }
                    },
                    onkeydown: move |evt: Event<KeyboardData>| {
                        if evt.key() == Key::Escape {
                            state.close_popover();
                        }
                    },
                }
            }

            // Auto-save hint
            div {
                class: "mt-3 pt-2 border-t border-slate-700/50",
                p {
                    class: "text-[10px] text-slate-500 text-center",
                    "Changes save automatically"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared components
// ---------------------------------------------------------------------------

#[component]
fn SectionCard(title: String, subtitle: String, children: Element) -> Element {
    rsx! {
        div {
            class: "rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

            h2 {
                class: "text-lg font-semibold text-slate-50 mb-0.5",
                "{title}"
            }
            p {
                class: "text-sm text-slate-400 mb-4",
                "{subtitle}"
            }

            {children}
        }
    }
}

#[component]
fn PopoverCard(tag: SkillTag) -> Element {
    rsx! {
        div {
            class: "absolute z-50 top-full mt-2 left-0 w-64 rounded-xl border border-slate-700 bg-slate-800 shadow-lg p-3",
            onmousedown: move |evt: Event<MouseData>| {
                evt.prevent_default();
            },

            // Header
            div {
                class: "flex items-center justify-between mb-2",
                span {
                    class: "text-sm font-semibold text-slate-100",
                    "{tag.name}"
                }
                span {
                    class: "text-xs rounded-full bg-slate-700 px-2 py-0.5 text-slate-300",
                    "{tag.category}"
                }
            }

            // Description
            p {
                class: "text-xs text-slate-400 leading-relaxed mb-3",
                "{tag.description}"
            }

            // Level bar
            div {
                class: "space-y-1",
                div {
                    class: "flex items-center justify-between text-xs",
                    span { class: "text-slate-400", "Proficiency" }
                    span { class: "text-slate-300 font-medium", "{tag.level_label()}" }
                }
                div {
                    class: "h-1.5 rounded-full bg-slate-700 overflow-hidden",
                    div {
                        class: "h-full rounded-full bg-indigo-500 {tag.level_bar_width()}",
                    }
                }
            }
        }
    }
}
