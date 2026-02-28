//! Sortable tag pills with drag-and-drop reordering.
//!
//! Demonstrates dx-dnd integration with compound components for
//! visual drag-to-reorder pills. Locked tags cannot be dragged.
//!
//! Run with: dx serve --example sortable

use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{components as tag_input, TagInputState, TagLike};
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    ActiveDrag, DragId, DragOverlay, ReorderEvent, SortableContext, SortableItem, FEEDBACK_STYLES,
    FUNCTIONAL_STYLES,
};

fn main() {
    dioxus::launch(App);
}

// ── Tag type ────────────────────────────────────────────────────────────

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

// ── App ─────────────────────────────────────────────────────────────────

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
                    "Drag pills to reorder. Cherry is locked."
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
                    TagInputUI {}
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

// ── Tag input UI with sortable pills ────────────────────────────────────

#[component]
fn TagInputUI() -> Element {
    let mut ctx = use_context::<TagInputState<FruitTag>>();

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
                        if let Some(from) = tags.iter().position(|t| t.id() == e.item_id.0.as_str()) {
                            // Compute insertion index in the list after removing the dragged item
                            let mut ids: Vec<DragId> = tags.iter().map(|t| DragId::new(t.id())).collect();
                            ids.remove(from);
                            let to = e.to_index.min(ids.len());
                            drop(tags);
                            ctx.move_tag(from, to);
                        }
                    },

                    div { class: "flex flex-wrap items-center gap-2",
                        for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                            {
                                let is_locked = tag.is_locked();
                                let key = tag.id().to_string();
                                let name = tag.name().to_string();
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
                            if let Some(tag) = tags.iter().find(|t| t.id() == active.data.id.0.as_str()) {
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

                tag_input::Input::<FruitTag> {
                    class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm mt-2",
                }

                if *ctx.is_at_limit.read() {
                    span { class: "text-xs text-amber-400 ml-1 mt-2", "(limit reached)" }
                }
            }

            // Dropdown
            tag_input::Dropdown::<FruitTag> {
                class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                for (i, suggestion) in ctx.filtered_suggestions.read().iter().cloned().enumerate() {
                    {
                        let name = suggestion.name().to_string();
                        rsx! {
                            tag_input::Option {
                                key: "{suggestion.id()}",
                                tag: suggestion,
                                index: i,
                                class: "px-3 py-2 text-sm cursor-pointer transition-colors hover:bg-slate-700",
                                "{name}"
                            }
                        }
                    }
                }
            }
        }

        tag_input::LiveRegion::<FruitTag> {}

        p {
            class: "mt-3 text-xs text-slate-500",
            "Drag to reorder.  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2191}\u{2193}" }
            "navigate  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
            "pills  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
            "select  "
            span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
            "remove"
        }
    }
}
