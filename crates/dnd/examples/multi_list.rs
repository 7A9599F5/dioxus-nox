//! Cross-container sortable example with two lists
//!
//! Demonstrates dragging items between multiple sortable containers
//! using explicit handlers (the recommended pattern).
//!
//! Run with: dx serve --example multi_list

use dioxus::prelude::*;
use dioxus_nox_dnd::{
    DragId, DragOverlay, MoveEvent, ReorderEvent, SortableContext, SortableGroup, SortableItem,
    FUNCTIONAL_STYLES, THEME_STYLES,
};

// Container ID constants to prevent typos
const LIST_A_ID: &str = "list-a";
const LIST_B_ID: &str = "list-b";

fn main() {
    dioxus::launch(app);
}

/// A task item with a stable ID
#[derive(Clone, Debug, PartialEq)]
struct Task {
    id: String,
    text: String,
}

impl Task {
    fn drag_id(&self) -> DragId {
        DragId::new(&self.id)
    }
}

/// A reusable task list component
#[component]
fn TaskList(id: DragId, title: String, items: Signal<Vec<Task>>) -> Element {
    // Read signal once and reuse - avoids multiple subscription overhead
    let items_value = items.read();
    let item_ids: Vec<DragId> = items_value.iter().map(|t| t.drag_id()).collect();

    rsx! {
        div { class: "list-wrapper",
            h2 { "{title}" }

            SortableContext {
                id: id,
                items: item_ids,

                div { class: "task-list",
                    for task in items_value.iter() {
                        SortableItem {
                            key: "{task.id}",
                            id: DragId::new(&task.id),

                            div { class: "task-item", "{task.text}" }
                        }
                    }

                    if items_value.is_empty() {
                        div { class: "empty-state", "Drop tasks here" }
                    }
                }
            }
        }
    }
}

fn app() -> Element {
    // Two separate lists of tasks
    let list_a = use_signal(|| {
        vec![
            Task {
                id: "a1".to_string(),
                text: "Design mockups".to_string(),
            },
            Task {
                id: "a2".to_string(),
                text: "Write specs".to_string(),
            },
            Task {
                id: "a3".to_string(),
                text: "Review PRs".to_string(),
            },
        ]
    });

    let list_b = use_signal(|| {
        vec![
            Task {
                id: "b1".to_string(),
                text: "Fix bug #123".to_string(),
            },
            Task {
                id: "b2".to_string(),
                text: "Update docs".to_string(),
            },
        ]
    });

    // Container map for .apply() methods
    // Clone for each closure since move closures consume the value
    let containers_for_reorder = [
        (DragId::new(LIST_A_ID), list_a),
        (DragId::new(LIST_B_ID), list_b),
    ];
    let containers_for_move = containers_for_reorder.clone();

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {MULTI_LIST_STYLES} }

        h1 { "Cross-Container Sortable" }
        p { class: "instructions", "Drag tasks between lists or reorder within a list" }

        SortableGroup {
            // Use .apply() for concise same-container reordering
            on_reorder: move |e: ReorderEvent| {
                e.apply(&containers_for_reorder, |t: &Task| t.drag_id());
            },

            // Use .apply() for concise cross-container moves
            on_move: move |e: MoveEvent| {
                e.apply(&containers_for_move, |t: &Task| t.drag_id());
            },

            div { class: "lists-container",
                TaskList {
                    id: DragId::new(LIST_A_ID),
                    title: "Backlog".to_string(),
                    items: list_a,
                }

                TaskList {
                    id: DragId::new(LIST_B_ID),
                    title: "In Progress".to_string(),
                    items: list_b,
                }
            }

            DragOverlay {
                div { class: "task-item dragging", "Moving..." }
            }
        }
    }
}

const MULTI_LIST_STYLES: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        background: var(--dxdnd-bg-subtle);
        color: var(--dxdnd-text);
    }

    h1 {
        margin-bottom: 8px;
        color: var(--dxdnd-text);
    }

    .instructions {
        color: var(--dxdnd-text-muted);
        margin-bottom: 24px;
    }

    .lists-container {
        display: flex;
        gap: 24px;
    }

    .list-wrapper {
        flex: 1;
        min-width: 250px;
        max-width: 350px;
    }

    .list-wrapper h2 {
        font-size: 14px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--dxdnd-text-muted);
        margin-bottom: 12px;
    }

    .task-list {
        background: var(--dxdnd-bg);
        border-radius: 8px;
        padding: 8px;
        min-height: 200px;
        box-shadow: var(--dxdnd-shadow);
    }

    .empty-state {
        color: var(--dxdnd-text-muted);
        text-align: center;
        padding: 40px 20px;
        font-style: italic;
    }
"#;
