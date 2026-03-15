//! Horizontal sortable pill list example
//!
//! Demonstrates horizontal orientation for SortableContext.
//! Run with: dx serve --example horizontal_pills

use dioxus::prelude::*;
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    DragId, DragOverlay, FUNCTIONAL_STYLES, ReorderEvent, SortableContext, SortableItem,
    THEME_STYLES,
};

fn main() {
    dioxus::launch(app);
}

#[derive(Clone, Debug, PartialEq)]
struct Tag {
    id: String,
    label: String,
    color: String,
}

fn app() -> Element {
    let items = use_signal(|| {
        vec![
            Tag {
                id: "rust".into(),
                label: "Rust".into(),
                color: "#dea584".into(),
            },
            Tag {
                id: "dioxus".into(),
                label: "Dioxus".into(),
                color: "#84c7de".into(),
            },
            Tag {
                id: "wasm".into(),
                label: "WASM".into(),
                color: "#654ff0".into(),
            },
            Tag {
                id: "dnd".into(),
                label: "Drag & Drop".into(),
                color: "#4caf50".into(),
            },
            Tag {
                id: "horizontal".into(),
                label: "Horizontal".into(),
                color: "#ff9800".into(),
            },
        ]
    });

    let item_ids: Vec<DragId> = items.read().iter().map(|t| t.id.clone().into()).collect();

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {PILL_STYLES} }

        div { class: "container",
            h1 { "Horizontal Sortable Pills" }
            p { class: "subtitle", "Drag to reorder tags" }

            SortableContext {
                id: "pills",
                items: item_ids,
                orientation: Orientation::Horizontal,
                on_reorder: move |e: ReorderEvent| {
                    e.apply_single(items, |t| t.id.clone().into());
                },

                div { class: "pill-row",
                    for tag in items.read().iter() {
                        SortableItem {
                            key: "{tag.id}",
                            id: tag.id.clone(),

                            div {
                                class: "pill",
                                style: "background: {tag.color};",
                                "{tag.label}"
                            }
                        }
                    }
                }

                DragOverlay {
                    div { class: "pill pill-overlay", "Moving..." }
                }
            }
        }
    }
}

const PILL_STYLES: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        background: var(--dxdnd-bg-subtle);
        color: var(--dxdnd-text);
        margin: 0;
        padding: 40px 20px;
    }

    .container {
        max-width: 700px;
        margin: 0 auto;
    }

    h1 {
        margin-bottom: 4px;
    }

    .subtitle {
        color: var(--dxdnd-text-muted);
        margin-bottom: 24px;
    }

    .pill-row {
        background: var(--dxdnd-bg);
        padding: 16px;
        border-radius: 12px;
        box-shadow: var(--dxdnd-shadow);
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }

    .pill {
        padding: 8px 16px;
        border-radius: 20px;
        color: white;
        font-weight: 600;
        font-size: 14px;
        white-space: nowrap;
        cursor: grab;
        user-select: none;
    }

    .pill-overlay {
        background: var(--dxdnd-primary);
        box-shadow: var(--dxdnd-shadow-drag);
    }

    /* Override default sortable-item styles for pills */
    .pill-row .sortable-item > * {
        background: none;
        border: none;
        border-radius: 20px;
        padding: 0;
        box-shadow: none;
    }

    .pill-row .sortable-item > *:hover {
        box-shadow: none;
    }

    .pill-row .sortable-item.dragging > * {
        background: none;
        border: none;
        box-shadow: none;
    }
"#;
