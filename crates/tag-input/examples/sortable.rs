//! Sortable tag pills with drag-and-drop reordering + dropdown.
//!
//! Demonstrates dx-dnd integration with compound components for
//! visual drag-to-reorder pills. Locked tags cannot be dragged.
//! Uses `dioxus-nox-select` for dropdown functionality.
//!
//! Run with: dx serve --example sortable

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    ActiveDrag, DragId, DragOverlay, FEEDBACK_STYLES, FUNCTIONAL_STYLES, ReorderEvent,
    SortableContext, SortableItem,
};
use dioxus_nox_select::{SelectContext, select};
use dioxus_nox_tag_input::{
    TagInputState, TagLike, components as tag_input, extract_clipboard_text,
};

fn main() {
    dioxus::launch(App);
}

// -- Tag type ────────────────────────────────────────────────────────────

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

// -- App ─────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let mut last_event = use_signal(|| String::from("No events yet"));

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }
        style { {FUNCTIONAL_STYLES} }
        style { {FEEDBACK_STYLES} }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 { class: "text-xl font-bold mb-1 text-slate-50", "Sortable Tag Input" }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Drag pills to reorder. Cherry is locked. Type to search the dropdown."
                }

                tag_input::Root::<FruitTag> {
                    available_tags: fruit_tags(),
                    initial_selected: vec![
                        FruitTag::locked("cherry", "Cherry"),
                        FruitTag::new("apple", "Apple"),
                        FruitTag::new("banana", "Banana"),
                    ],
                    max_tags: Some(6),
                    on_add: move |tag: FruitTag| last_event.set(format!("Added: {}", tag.name())),
                    on_remove: move |tag: FruitTag| last_event.set(format!("Removed: {}", tag.name())),
                    on_reorder: move |(from, to): (usize, usize)| {
                        last_event.set(format!("Reordered: position {from} \u{2192} {to}"))
                    },
                    TagInputWithSelect {}
                }

                div {
                    class: "mt-3 rounded-lg bg-slate-900/60 border border-slate-700/50 px-3 py-2 text-xs text-slate-400",
                    "data-testid": "event-log",
                    span { class: "text-slate-500 mr-1", "Last event:" }
                    "{last_event}"
                }
            }
        }
    }
}

// -- Bridge: sync select ↔ tag-input ─────────────────────────────────────

#[component]
fn SelectTagBridge(available: Vec<FruitTag>, children: Element) -> Element {
    let mut state = use_context::<TagInputState<FruitTag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Forward sync: select → tag-input (when user picks from dropdown)
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

    // Reverse sync: tag-input → select (when user removes a pill)
    use_effect(move || {
        let tag_ids: Vec<String> = state
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        let select_values = select_ctx.current_values_peek();
        for val in &select_values {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

// -- Tag input UI with sortable pills + dropdown ─────────────────────────

#[component]
fn TagInputWithSelect() -> Element {
    let available = fruit_tags();

    rsx! {
        select::Root {
            multiple: true,
            open_on_focus: true,
            SelectTagBridge {
                available: available.clone(),
                TagInputUI { available: available.clone() }
            }
        }
    }
}

#[component]
fn TagInputUI(available: Vec<FruitTag>) -> Element {
    let mut ctx = use_context::<TagInputState<FruitTag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Derive DragId list for SortableContext from selected tags
    let item_ids: Vec<DragId> = ctx
        .selected_tags
        .read()
        .iter()
        .map(|t| DragId::new(t.id()))
        .collect();

    rsx! {
        div { class: "relative",
            tag_input::Control::<FruitTag> {
                class: "rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none",

                SortableContext {
                    id: "tags",
                    items: item_ids,
                    orientation: Orientation::Horizontal,
                    on_reorder: move |e: ReorderEvent| {
                        let tags = ctx.selected_tags.read();
                        if let Some(from) = tags.iter().position(|t| t.id() == e.item_id.as_str()) {
                            // Compute insertion index in the list after removing the dragged item
                            let mut ids: Vec<DragId> = tags.iter().map(|t| DragId::new(t.id())).collect();
                            ids.remove(from);
                            let to = e.to_index.min(ids.len());
                            drop(tags);
                            ctx.move_tag(from, to);
                        }
                    },

                    div { class: "flex flex-wrap items-center gap-2",
                        for (_i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                            {
                                let is_locked = tag.is_locked();
                                let key = tag.id().to_string();
                                let name = tag.name().to_string();
                                let i = _i;
                                rsx! {
                                    SortableItem {
                                        key: "{key}",
                                        id: key.clone(),
                                        disabled: is_locked,

                                        tag_input::Tag {
                                            tag: tag.clone(),
                                            index: i,
                                            class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/30 border border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 cursor-grab active:cursor-grabbing",
                                            "{name}"
                                            if is_locked {
                                                span { class: "ml-0.5 text-indigo-400/50 text-xs", "\u{1F512}" }
                                            } else {
                                                tag_input::TagRemove {
                                                    tag: tag.clone(),
                                                    class: "ml-0.5 rounded hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    DragOverlay {
                        class: "z-50",
                        align_to_grab_point: true,
                        render: move |active: ActiveDrag| {
                            let tags = ctx.selected_tags.read();
                            if let Some(tag) = tags.iter().find(|t| t.id() == active.data.id.as_str()) {
                                let name = tag.name().to_string();
                                rsx! {
                                    div {
                                        class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/50 border border-indigo-400/60 px-2.5 py-0.5 text-sm text-indigo-100 shadow-lg shadow-indigo-500/20",
                                        "{name}"
                                    }
                                }
                            } else {
                                VNode::empty()
                            }
                        },
                    }
                }

                // Combo-style input with select keyboard wiring
                input {
                    r#type: "text",
                    role: "combobox",
                    disabled: *ctx.is_disabled.read(),
                    readonly: *ctx.is_readonly.read(),
                    class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm mt-2",
                    placeholder: "Type to search\u{2026}",
                    value: "{ctx.search_query}",
                    aria_expanded: select_ctx.is_open(),
                    aria_controls: select_ctx.listbox_id(),
                    aria_activedescendant: select_ctx.active_descendant(),
                    aria_autocomplete: "list",
                    oninput: move |evt| {
                        let val = evt.value();
                        ctx.set_query(val.clone());
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

                if *ctx.is_at_limit.read() {
                    span { class: "text-xs text-amber-400 ml-1 mt-2", "(limit reached)" }
                }
            }

            // Dropdown via select::Content
            select::Content {
                class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
                select::Empty {
                    class: "px-3 py-2 text-sm text-slate-500",
                    "No results found."
                }
                for tag in &available {
                    select::Item {
                        value: "{tag.id()}",
                        label: tag.name().to_string(),
                        class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-300",
                        "{tag.name()}"
                    }
                }
            }
        }

        tag_input::LiveRegion::<FruitTag> {}

        p {
            class: "mt-3 text-xs text-slate-500",
            "Drag to reorder.  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
            "pills  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
            "select  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
            "remove"
        }
    }
}
