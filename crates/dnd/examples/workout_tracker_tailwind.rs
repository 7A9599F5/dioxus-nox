//! Workout tracker with Tailwind CSS styling
//!
//! Same functionality as workout_tracker.rs but uses Tailwind utility classes
//! instead of custom CSS for app-level styling. Functional + feedback CSS
//! layers are loaded — theme CSS is omitted since Tailwind handles item visuals.
//!
//! Run with: dx serve --example workout_tracker_tailwind

use dioxus::prelude::*;
use dioxus_nox_dnd::grouped::{
    GroupedItem, active_group_header, grouped_merge, grouped_position, grouped_reorder_default,
    grouped_style_info,
};
use dioxus_nox_dnd::styles::{GROUPED_FEEDBACK_STYLES, GROUPED_FUNCTIONAL_STYLES};
use dioxus_nox_dnd::{
    ActiveDrag, DragContext, DragId, DragOverlay, FEEDBACK_STYLES, FUNCTIONAL_STYLES, MergeEvent,
    ReorderEvent, SortableContext, SortableGroup, SortableItem,
};

fn main() {
    dioxus::launch(app);
}

// ============================================================================
// Data Model
// ============================================================================

/// A workout item - either an exercise or a superset header
#[derive(Clone, Debug, PartialEq)]
enum WorkoutItem {
    Exercise {
        id: String,
        name: String,
        sets: u32,
        reps: u32,
        /// None = standalone, Some(id) = grouped in superset
        superset_id: Option<String>,
    },
    SupersetHeader {
        /// ID that matches superset_id of member exercises
        id: String,
    },
}

impl GroupedItem for WorkoutItem {
    type GroupId = String;

    fn drag_id(&self) -> DragId {
        match self {
            WorkoutItem::Exercise { id, .. } => DragId::new(id),
            WorkoutItem::SupersetHeader { id } => DragId::new(format!("header-{}", id)),
        }
    }

    fn group_id(&self) -> Option<&String> {
        match self {
            WorkoutItem::Exercise { superset_id, .. } => superset_id.as_ref(),
            WorkoutItem::SupersetHeader { id } => Some(id),
        }
    }

    fn is_group_header(&self) -> bool {
        matches!(self, WorkoutItem::SupersetHeader { .. })
    }

    fn set_group_id(&mut self, group_id: Option<String>) {
        if let WorkoutItem::Exercise { superset_id, .. } = self {
            *superset_id = group_id;
        }
    }

    fn make_group_header(group_id: String) -> Self {
        WorkoutItem::SupersetHeader { id: group_id }
    }
}

// ============================================================================
// App Component
// ============================================================================

fn app() -> Element {
    let workout: Signal<Vec<WorkoutItem>> = use_signal(|| {
        vec![
            WorkoutItem::Exercise {
                id: "1".into(),
                name: "Bench Press".into(),
                sets: 3,
                reps: 10,
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "2".into(),
                name: "Incline Press".into(),
                sets: 3,
                reps: 10,
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "3".into(),
                name: "Chest Fly".into(),
                sets: 3,
                reps: 12,
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "4".into(),
                name: "Tricep Dips".into(),
                sets: 3,
                reps: 15,
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "5".into(),
                name: "Overhead Extension".into(),
                sets: 3,
                reps: 12,
                superset_id: None,
            },
        ]
    });

    rsx! {
        // Library CSS: functional (required) + feedback (DnD visual indicators)
        style { {FUNCTIONAL_STYLES} }
        style { {GROUPED_FUNCTIONAL_STYLES} }
        style { {FEEDBACK_STYLES} }
        style { {GROUPED_FEEDBACK_STYLES} }

        // Tailwind CSS
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        // CSS variable overrides
        style { {THEME_OVERRIDES} }

        div { class: "max-w-lg mx-auto p-8 bg-slate-50 min-h-screen font-sans dark:bg-slate-900",
            h1 { class: "text-2xl font-bold text-slate-800 mb-2 dark:text-slate-100",
                "Workout Tracker"
            }
            p { class: "text-sm text-slate-500 mb-8 leading-relaxed dark:text-slate-400",
                "Drag exercises to reorder. Drag onto the center of another exercise to create a superset. "
                "Drag the superset header to move the entire group."
            }

            SortableGroup {
                enable_merge: true,
                on_reorder: move |e: ReorderEvent| {
                    grouped_reorder_default(&workout, &e);
                },

                on_merge: move |e: MergeEvent| {
                    grouped_merge(&workout, &e);
                },

                WorkoutList {
                    items: workout,
                }
            }
        }
    }
}

// ============================================================================
// WorkoutList Component (for DragContext access)
// ============================================================================

/// Inner component that renders the workout list with access to DragContext
#[component]
fn WorkoutList(items: Signal<Vec<WorkoutItem>>) -> Element {
    let ctx = use_context::<DragContext>();

    let drag_ids: Vec<DragId> = items.read().iter().map(|item| item.drag_id()).collect();

    let items_read = items.read();
    let active_header = active_group_header(&ctx, &items_read);

    rsx! {
        div { class: "flex flex-col gap-2",
            SortableContext {
                id: DragId::new("workout"),
                items: drag_ids,

                for (index, item) in items_read.iter().enumerate() {
                    {
                        let position = grouped_position(&items_read, index);
                        let style_info = grouped_style_info(position);
                        let group_id = item.group_id().cloned();

                        let is_group_drag_active = active_header
                            .as_ref()
                            .map(|active| group_id.as_ref() == Some(&active.group_id))
                            .unwrap_or(false);

                        let drop_disabled = is_group_drag_active && group_id.is_some();
                        let drag_id = item.drag_id();
                        let key = format!("{}__{}", drag_id.0, style_info.data_group_role.unwrap_or("standalone"));

                        let content = match item {
                            WorkoutItem::SupersetHeader { .. } => {
                                rsx! {
                                    div {
                                        class: "superset-header",
                                        "data-group-role": "header",
                                        span { class: "opacity-70 text-base", "\u{22ee}\u{22ee}" }
                                        span { "SUPERSET" }
                                    }
                                }
                            }
                            WorkoutItem::Exercise { name, sets, reps, .. } => {
                                rsx! {
                                    div {
                                        "data-group-role": if style_info.data_group_role.is_some() { style_info.data_group_role } else { None },
                                        div { class: "flex justify-between items-center w-full gap-4",
                                            div { class: "font-medium text-slate-800 dark:text-slate-100",
                                                "{name}"
                                            }
                                            div { class: "text-sm font-medium text-slate-500 dark:text-slate-400",
                                                "{sets} \u{00d7} {reps}"
                                            }
                                        }
                                    }
                                }
                            }
                        };

                        rsx! {
                            SortableItem {
                                key: "{key}",
                                id: drag_id,
                                drop_disabled: drop_disabled,
                                {content}
                            }
                        }
                    }
                }
            }

            DragOverlay {
                render: move |active_drag: ActiveDrag| {
                    let items_read = items.read();
                    if let Some((index, item)) = items_read.iter().enumerate().find(|(_, item)| item.drag_id() == active_drag.data.id) {
                        let position = grouped_position(&items_read, index);
                        let style_info = grouped_style_info(position);

                        match item {
                            WorkoutItem::SupersetHeader { .. } => {
                                rsx! {
                                    div {
                                        class: "superset-header",
                                        "data-group-role": "header",
                                        span { class: "opacity-70 text-base", "\u{22ee}\u{22ee}" }
                                        span { "SUPERSET" }
                                    }
                                }
                            }
                            WorkoutItem::Exercise { name, sets, reps, .. } => {
                                rsx! {
                                    div {
                                        "data-group-role": if style_info.data_group_role.is_some() { style_info.data_group_role } else { None },
                                        div { class: "flex justify-between items-center w-full gap-4",
                                            div { class: "font-medium text-slate-800 dark:text-slate-100",
                                                "{name}"
                                            }
                                            div { class: "text-sm font-medium text-slate-500 dark:text-slate-400",
                                                "{sets} \u{00d7} {reps}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        VNode::empty()
                    }
                }
            }
        }
    }
}

// ============================================================================
// Theme Overrides
// ============================================================================

const THEME_OVERRIDES: &str = r#"
:root {
    --dxdnd-grouped-header-bg: linear-gradient(135deg, #4f46e5, #7c3aed);
    --dxdnd-grouped-header-color: #ffffff;
    --dxdnd-grouped-member-bg: #ffffff;
    --dxdnd-grouped-border: #e2e8f0;
    --dxdnd-grouped-radius: 0.75rem;
    --dxdnd-grouped-collapse-duration: 180ms;
}

body {
    margin: 0;
}

[data-dnd-item][data-state="dragging"] > *:not([data-dnd-indicator]):not([data-dnd-preview]) {
    opacity: 0.5;
}

[data-dnd-indicator] {
    height: 3px;
    background: #4f46e5;
    border-radius: 2px;
    margin: 0.25rem 0;
}

.superset-header {
    font-weight: 600;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: grab;
}

.superset-header:active {
    cursor: grabbing;
}

@media (prefers-color-scheme: dark) {
    :root {
        --dxdnd-grouped-header-bg: linear-gradient(135deg, #4338ca, #6d28d9);
        --dxdnd-grouped-header-color: #e2e8f0;
        --dxdnd-grouped-member-bg: #1e293b;
        --dxdnd-grouped-border: #334155;
    }
}
"#;
