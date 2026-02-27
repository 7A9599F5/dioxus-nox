//! Allow/Deny privilege selector example
//!
//! Demonstrates a permission management UI where items can be dragged
//! between "Allow" and "Deny" lists using explicit handlers.
//!
//! Run with: dx serve --example allow_deny

use dioxus::prelude::*;
use dioxus_nox_dnd::{
    DragId, DragOverlay, MoveEvent, ReorderEvent, SortableContext, SortableGroup, SortableItem,
    FUNCTIONAL_STYLES, THEME_STYLES,
};

// Container ID constants to prevent typos
const ALLOWED_ID: &str = "allowed";
const DENIED_ID: &str = "denied";

fn main() {
    dioxus::launch(app);
}

/// A privilege/permission item with a stable ID
#[derive(Clone, Debug, PartialEq)]
struct Privilege {
    id: String,
    name: String,
    description: String,
}

impl Privilege {
    fn drag_id(&self) -> DragId {
        DragId::new(&self.id)
    }
}

/// A reusable privilege list component
#[component]
fn PrivilegeList(
    id: DragId,
    title: String,
    items: Signal<Vec<Privilege>>,
    list_type: ListType,
) -> Element {
    let (icon, header_class, list_class) = match list_type {
        ListType::Allow => ("✓", "list-header allow", "privilege-list allow"),
        ListType::Deny => ("✗", "list-header deny", "privilege-list deny"),
    };

    // Read signal once and reuse - avoids multiple subscription overhead
    let items_value = items.read();
    let item_ids: Vec<DragId> = items_value.iter().map(|p| p.drag_id()).collect();
    let items_count = items_value.len();

    rsx! {
        div { class: "privilege-list-wrapper",
            div {
                class: "{header_class}",
                span { class: "list-icon", "{icon}" }
                h2 { "{title}" }
                span { class: "item-count", "{items_count}" }
            }

            SortableContext {
                id: id,
                items: item_ids,

                div {
                    class: "{list_class}",

                    for priv_item in items_value.iter() {
                        SortableItem {
                            key: "{priv_item.id}",
                            id: DragId::new(&priv_item.id),

                            div { class: "privilege-item",
                                div { class: "privilege-name", "{priv_item.name}" }
                                div { class: "privilege-desc", "{priv_item.description}" }
                            }
                        }
                    }

                    if items_value.is_empty() {
                        div { class: "empty-state",
                            "Drag privileges here to {title.to_lowercase()}"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ListType {
    Allow,
    Deny,
}

fn app() -> Element {
    // Allowed privileges
    let allowed = use_signal(|| {
        vec![
            Privilege {
                id: "read".to_string(),
                name: "Read".to_string(),
                description: "View files and data".to_string(),
            },
            Privilege {
                id: "write".to_string(),
                name: "Write".to_string(),
                description: "Create and modify files".to_string(),
            },
        ]
    });

    // Denied privileges
    let denied = use_signal(|| {
        vec![
            Privilege {
                id: "delete".to_string(),
                name: "Delete".to_string(),
                description: "Remove files permanently".to_string(),
            },
            Privilege {
                id: "admin".to_string(),
                name: "Admin".to_string(),
                description: "Full system access".to_string(),
            },
            Privilege {
                id: "share".to_string(),
                name: "Share".to_string(),
                description: "Share with external users".to_string(),
            },
        ]
    });

    // Container map for .apply() methods
    // Clone for each closure since move closures consume the value
    let containers_for_reorder = [
        (DragId::new(ALLOWED_ID), allowed),
        (DragId::new(DENIED_ID), denied),
    ];
    let containers_for_move = containers_for_reorder.clone();

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {ALLOW_DENY_STYLES} }

        div { class: "container",
            h1 { "Permission Manager" }
            p { class: "instructions",
                "Drag privileges between lists to allow or deny access"
            }

            SortableGroup {
                // Use .apply() for concise same-container reordering
                on_reorder: move |e: ReorderEvent| {
                    e.apply(&containers_for_reorder, |p: &Privilege| p.drag_id());
                },

                // Use .apply() for concise cross-container moves
                on_move: move |e: MoveEvent| {
                    e.apply(&containers_for_move, |p: &Privilege| p.drag_id());
                },

                div { class: "lists-container",
                    PrivilegeList {
                        id: DragId::new(ALLOWED_ID),
                        title: "Allow".to_string(),
                        items: allowed,
                        list_type: ListType::Allow,
                    }

                    div { class: "divider",
                        div { class: "divider-line" }
                        span { class: "divider-icon", "⇄" }
                        div { class: "divider-line" }
                    }

                    PrivilegeList {
                        id: DragId::new(DENIED_ID),
                        title: "Deny".to_string(),
                        items: denied,
                        list_type: ListType::Deny,
                    }
                }

                DragOverlay {
                    div { class: "privilege-item dragging", "Moving privilege..." }
                }
            }

            div { class: "summary",
                h3 { "Current Configuration" }
                div { class: "summary-content",
                    div { class: "summary-section allow",
                        strong { "Allowed: " }
                        span {
                            {allowed.read().iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")}
                        }
                    }
                    div { class: "summary-section deny",
                        strong { "Denied: " }
                        span {
                            {denied.read().iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")}
                        }
                    }
                }
            }
        }
    }
}

const ALLOW_DENY_STYLES: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        background: var(--dxdnd-bg-subtle);
        color: var(--dxdnd-text);
        margin: 0;
    }

    .container {
        max-width: 800px;
        margin: 0 auto;
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
        gap: 16px;
        align-items: flex-start;
    }

    .privilege-list-wrapper {
        flex: 1;
        min-width: 280px;
    }

    .list-header {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 12px 16px;
        border-radius: 8px 8px 0 0;
        border: 2px solid;
        border-bottom: none;
    }

    .list-header.allow {
        background: #e8f5e9;
        border-color: #4caf50;
        color: #1b5e20;
    }

    .list-header.deny {
        background: #ffebee;
        border-color: #f44336;
        color: #b71c1c;
    }

    .list-header h2 {
        font-size: 16px;
        font-weight: 600;
        margin: 0;
        flex: 1;
        color: inherit;
    }

    .list-icon {
        font-size: 18px;
        font-weight: bold;
        color: inherit;
    }

    .item-count {
        background: rgba(0,0,0,0.1);
        padding: 2px 8px;
        border-radius: 12px;
        font-size: 12px;
        font-weight: 600;
        color: inherit;
    }

    .privilege-list {
        background: var(--dxdnd-bg);
        border-radius: 0 0 8px 8px;
        padding: 8px;
        min-height: 250px;
        border: 2px solid;
        border-top: 1px solid var(--dxdnd-border);
    }

    .privilege-list.allow {
        border-color: #4caf50;
    }

    .privilege-list.deny {
        border-color: #f44336;
    }

    .privilege-name {
        font-weight: 600;
        margin-bottom: 4px;
        color: var(--dxdnd-text);
    }

    .privilege-desc {
        font-size: 12px;
        color: var(--dxdnd-text-muted);
    }

    .empty-state {
        color: var(--dxdnd-text-muted);
        text-align: center;
        padding: 40px 20px;
        font-style: italic;
    }

    .divider {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        padding-top: 60px;
    }

    .divider-line {
        width: 2px;
        height: 40px;
        background: var(--dxdnd-border);
    }

    .divider-icon {
        font-size: 20px;
        color: var(--dxdnd-text-muted);
    }

    .summary {
        margin-top: 32px;
        background: var(--dxdnd-bg);
        border-radius: 8px;
        padding: 16px 20px;
        box-shadow: var(--dxdnd-shadow);
    }

    .summary h3 {
        margin: 0 0 12px 0;
        font-size: 14px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--dxdnd-text-muted);
    }

    .summary-content {
        display: flex;
        gap: 24px;
    }

    .summary-section {
        flex: 1;
        padding: 8px 12px;
        border-radius: 4px;
    }

    .summary-section.allow {
        background: #e8f5e9;
    }

    .summary-section.deny {
        background: #ffebee;
    }

    .summary-section strong {
        display: block;
        margin-bottom: 4px;
        font-size: 12px;
        text-transform: uppercase;
    }

    .summary-section.allow strong {
        color: #2e7d32;
    }

    .summary-section.deny strong {
        color: #c62828;
    }

    /* Dark mode overrides for semantic colors */
    @media (prefers-color-scheme: dark) {
        .list-header.allow {
            background: rgba(76, 175, 80, 0.25);
            border-color: #66bb6a;
            color: #a5d6a7;
        }

        .list-header.deny {
            background: rgba(244, 67, 54, 0.25);
            border-color: #ef5350;
            color: #ef9a9a;
        }

        .privilege-list.allow {
            border-color: #66bb6a;
        }

        .privilege-list.deny {
            border-color: #ef5350;
        }

        .item-count {
            background: rgba(255,255,255,0.15);
        }

        .summary-section.allow {
            background: rgba(76, 175, 80, 0.2);
        }

        .summary-section.deny {
            background: rgba(244, 67, 54, 0.2);
        }

        .summary-section.allow strong {
            color: #81c784;
        }

        .summary-section.deny strong {
            color: #e57373;
        }
    }
"#;
