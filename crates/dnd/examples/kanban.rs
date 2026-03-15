//! Kanban board with drag between columns
//!
//! Demonstrates SortableGroup for cross-container dragging.
//! Items can be dragged within a column (reorder) or between columns (move).
//!
//! Run with: dx serve --example kanban

use dioxus::prelude::*;
use dioxus_nox_dnd::{
    DragId, DragOverlay, FUNCTIONAL_STYLES, MoveEvent, ReorderEvent, SortableContext,
    SortableGroup, SortableItem, THEME_STYLES,
};

fn main() {
    dioxus::launch(app);
}

/// A task with a stable ID
#[derive(Clone, Debug, PartialEq)]
struct Task {
    id: String,
    text: String,
}

/// Column identifiers in display order
const COLUMN_ORDER: [&str; 3] = ["todo", "doing", "done"];

/// Column display titles
fn column_title(id: &str) -> &'static str {
    match id {
        "todo" => "To Do",
        "doing" => "In Progress",
        "done" => "Done",
        _ => "Unknown",
    }
}

fn app() -> Element {
    // State: HashMap of column_id -> Vec<Task> (with stable IDs)
    let mut columns = use_signal(|| {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "todo".to_string(),
            vec![
                Task {
                    id: "t1".to_string(),
                    text: "Task 1".to_string(),
                },
                Task {
                    id: "t2".to_string(),
                    text: "Task 2".to_string(),
                },
            ],
        );
        map.insert(
            "doing".to_string(),
            vec![Task {
                id: "t3".to_string(),
                text: "Task 3".to_string(),
            }],
        );
        map.insert(
            "done".to_string(),
            vec![
                Task {
                    id: "t4".to_string(),
                    text: "Task 4".to_string(),
                },
                Task {
                    id: "t5".to_string(),
                    text: "Task 5".to_string(),
                },
            ],
        );
        map
    });

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {KANBAN_STYLES} }

        div { class: "kanban-app",
            h1 { "Kanban Board" }
            p { class: "instructions",
                "Drag tasks within a column to reorder, or between columns to move."
            }

            SortableGroup {
                on_move: move |e: MoveEvent| {
                    let mut cols = columns.write();
                    // Find and remove from source container by item_id
                    let item = if let Some(source_col) = cols.get_mut(&e.from_container.0) {
                        let idx = source_col.iter().position(|t| DragId::new(&t.id) == e.item_id);
                        idx.map(|i| source_col.remove(i))
                    } else {
                        None
                    };

                    // Add to target container
                    if let Some(item) = item {
                        if let Some(target_col) = cols.get_mut(&e.to_container.0) {
                            let insert_idx = e.to_index.min(target_col.len());
                            target_col.insert(insert_idx, item);
                        }
                    }
                },
                on_reorder: move |e: ReorderEvent| {
                    let mut cols = columns.write();
                    if let Some(col) = cols.get_mut(&e.container_id.0) {
                        // Find the item by ID (from_index may not be reliable)
                        if let Some(from) = col.iter().position(|t| DragId::new(&t.id) == e.item_id) {
                            let item = col.remove(from);
                            // Adjust target index after removal
                            let to = if e.to_index > from {
                                (e.to_index - 1).min(col.len())
                            } else {
                                e.to_index.min(col.len())
                            };
                            col.insert(to, item);
                        }
                    }
                },

                div { class: "kanban-board",
                    // Render columns in fixed order
                    for col_id in COLUMN_ORDER.iter() {
                        {
                            let col_items: Vec<Task> = columns
                                .read()
                                .get(*col_id)
                                .cloned()
                                .unwrap_or_default();
                            rsx! {
                                KanbanColumn {
                                    key: "{col_id}",
                                    id: DragId::new(*col_id),
                                    title: column_title(col_id).to_string(),
                                    items: col_items,
                                }
                            }
                        }
                    }
                }

                DragOverlay {
                    div { class: "task-card", style: "opacity: 0.9;", "Moving task..." }
                }
            }
        }
    }
}

/// A single kanban column containing sortable items
#[component]
fn KanbanColumn(id: DragId, title: String, items: Vec<Task>) -> Element {
    // Generate DragIds from stable task IDs
    let item_ids: Vec<DragId> = items.iter().map(|task| DragId::new(&task.id)).collect();

    rsx! {
        div { class: "kanban-column",
            div { class: "column-header",
                h2 { "{title}" }
                span { class: "item-count", "{items.len()}" }
            }

            SortableContext {
                id: id.clone(),
                items: item_ids.clone(),
                // on_reorder inherited from SortableGroup - no need to specify!

                div { class: "column-content",
                    for task in items.iter() {
                        SortableItem {
                            key: "{task.id}",
                            id: DragId::new(&task.id),

                            div { class: "task-card",
                                "{task.text}"
                            }
                        }
                    }

                    // Empty state placeholder
                    if items.is_empty() {
                        div { class: "empty-column",
                            "Drop tasks here"
                        }
                    }
                }
            }
        }
    }
}

/// Custom styles for the Kanban board
const KANBAN_STYLES: &str = r#"
/* ==========================================================================
   Kanban App Layout
   ========================================================================== */

.kanban-app {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
}

.kanban-app h1 {
    margin: 0 0 0.5rem;
    color: #1e293b;
}

.instructions {
    color: #64748b;
    margin-bottom: 2rem;
}

/* ==========================================================================
   Kanban Board
   ========================================================================== */

.kanban-board {
    display: flex;
    gap: 1.5rem;
    align-items: flex-start;
}

/* ==========================================================================
   Kanban Columns
   ========================================================================== */

.kanban-column {
    flex: 1;
    min-width: 280px;
    max-width: 350px;
    background: #f1f5f9;
    border-radius: 0.75rem;
    padding: 1rem;
}

.column-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 2px solid #e2e8f0;
}

.column-header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.item-count {
    background: #cbd5e1;
    color: #475569;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.25rem 0.5rem;
    border-radius: 9999px;
}

.column-content {
    min-height: 200px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

/* ==========================================================================
   Task Cards
   ========================================================================== */

.task-card {
    background: white;
    border-radius: 0.5rem;
    padding: 1rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    border: 1px solid #e2e8f0;
    font-size: 0.9375rem;
    color: #334155;
    transition: box-shadow 0.2s ease, transform 0.2s ease;
}

.task-card:hover {
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}

.sortable-item.dragging .task-card {
    opacity: 0.5;
    transform: scale(0.98);
}

/* ==========================================================================
   Empty State
   ========================================================================== */

.empty-column {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100px;
    border: 2px dashed #cbd5e1;
    border-radius: 0.5rem;
    color: #94a3b8;
    font-size: 0.875rem;
}

/* ==========================================================================
   Responsive Design
   ========================================================================== */

@media (max-width: 900px) {
    .kanban-board {
        flex-direction: column;
    }

    .kanban-column {
        max-width: none;
        width: 100%;
    }
}
"#;
