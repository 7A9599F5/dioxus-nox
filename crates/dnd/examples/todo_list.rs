//! Simple sortable todo list example
//!
//! Run with: dx serve --example todo_list

use dioxus::prelude::*;
use dioxus_nox_dnd::{
    DragId, DragOverlay, ReorderEvent, SortableContext, SortableItem, FUNCTIONAL_STYLES,
    THEME_STYLES,
};

fn main() {
    dioxus::launch(app);
}

/// A todo item with a stable ID
#[derive(Clone, Debug, PartialEq)]
struct TodoItem {
    id: String,
    text: String,
}

fn app() -> Element {
    // Use stable IDs for each item
    let items = use_signal(|| {
        vec![
            TodoItem {
                id: "a".to_string(),
                text: "Buy groceries".to_string(),
            },
            TodoItem {
                id: "b".to_string(),
                text: "Walk the dog".to_string(),
            },
            TodoItem {
                id: "c".to_string(),
                text: "Write code".to_string(),
            },
            TodoItem {
                id: "d".to_string(),
                text: "Review PRs".to_string(),
            },
        ]
    });

    // Create DragIds from stable IDs
    let item_ids: Vec<DragId> = items
        .read()
        .iter()
        .map(|item| item.id.clone().into())
        .collect();

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {TODO_STYLES} }

        h1 { "Sortable Todo List" }

        SortableContext {
            id: "todos",
            items: item_ids,
            on_reorder: move |e: ReorderEvent| {
                // Use the new helper to apply reordering
                e.apply_single(items, |item| item.id.clone().into());
            },

            div { class: "todo-list",
                for item in items.read().iter() {
                    SortableItem {
                        key: "{item.id}",
                        id: item.id.clone(), // Ergonomic: automatic conversion

                        div { class: "todo-item", "{item.text}" }
                    }
                }
            }

            DragOverlay {
                div { class: "todo-item dragging", "Moving..." }
            }
        }
    }
}

const TODO_STYLES: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        max-width: 600px;
        margin: 0 auto;
        background: var(--dxdnd-bg-subtle);
        color: var(--dxdnd-text);
    }

    h1 {
        margin-bottom: 24px;
        color: var(--dxdnd-text);
    }

    .todo-list {
        background: var(--dxdnd-bg);
        padding: 16px;
        border-radius: 8px;
        box-shadow: var(--dxdnd-shadow);
    }
"#;
