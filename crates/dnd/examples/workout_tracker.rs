//! Workout tracker with superset grouping via nested containers
//!
//! Demonstrates dx-dnd nested SortableContext for structural grouping:
//! - Reorder exercises in workout
//! - Create supersets by merging exercises (drag onto center)
//! - Reorder exercises within supersets (stays in group)
//! - Drag exercise below superset → becomes standalone (exits group)
//! - Drag exercise into superset → joins group
//! - Auto-dissolve supersets with < 2 members
//!
//! Run with: dx serve --example workout_tracker

use dioxus::prelude::*;
use dioxus_nox_dnd::grouped::{
    GroupedItem, TopLevelEntry, grouped_merge, grouped_move_default, grouped_position,
    grouped_reorder_default, grouped_style_info, partition_grouped_items,
};
use dioxus_nox_dnd::styles::{GROUPED_FUNCTIONAL_STYLES, GROUPED_THEME_STYLES};
use dioxus_nox_dnd::{
    ActiveDrag, DragContext, DragId, DragOverlay, DragType, FUNCTIONAL_STYLES, MergeEvent,
    MoveEvent, ReorderEvent, SortableContext, SortableGroup, SortableItem, THEME_STYLES,
};

fn main() {
    dioxus::launch(app);
}

// ============================================================================
// Data Model
// ============================================================================

/// A single set within an exercise (e.g., "10 reps @ 135 lbs")
#[derive(Clone, Debug, PartialEq)]
struct ExerciseSet {
    reps: u32,
    weight: f32,
}

/// A workout item - either an exercise or a superset header
#[derive(Clone, Debug, PartialEq)]
enum WorkoutItem {
    Exercise {
        id: String,
        name: String,
        sets: Vec<ExerciseSet>,
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
                sets: vec![
                    ExerciseSet {
                        reps: 10,
                        weight: 135.0,
                    },
                    ExerciseSet {
                        reps: 8,
                        weight: 155.0,
                    },
                    ExerciseSet {
                        reps: 6,
                        weight: 175.0,
                    },
                ],
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "2".into(),
                name: "Incline Press".into(),
                sets: vec![
                    ExerciseSet {
                        reps: 10,
                        weight: 115.0,
                    },
                    ExerciseSet {
                        reps: 8,
                        weight: 125.0,
                    },
                    ExerciseSet {
                        reps: 8,
                        weight: 125.0,
                    },
                ],
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "3".into(),
                name: "Chest Fly".into(),
                sets: vec![
                    ExerciseSet {
                        reps: 12,
                        weight: 30.0,
                    },
                    ExerciseSet {
                        reps: 12,
                        weight: 30.0,
                    },
                ],
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "4".into(),
                name: "Tricep Dips".into(),
                sets: vec![
                    ExerciseSet {
                        reps: 15,
                        weight: 0.0,
                    },
                    ExerciseSet {
                        reps: 12,
                        weight: 0.0,
                    },
                    ExerciseSet {
                        reps: 10,
                        weight: 10.0,
                    },
                    ExerciseSet {
                        reps: 8,
                        weight: 25.0,
                    },
                ],
                superset_id: None,
            },
            WorkoutItem::Exercise {
                id: "5".into(),
                name: "Overhead Extension".into(),
                sets: vec![ExerciseSet {
                    reps: 12,
                    weight: 40.0,
                }],
                superset_id: None,
            },
        ]
    });

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {GROUPED_FUNCTIONAL_STYLES} }
        style { {GROUPED_THEME_STYLES} }
        style { {WORKOUT_STYLES} }

        div { class: "workout-app",
            h1 { "Workout Tracker" }
            p { class: "instructions",
                "Drag exercises to reorder. Drag onto the center of another exercise to create a superset. "
                "Drag an exercise out of a superset to ungroup it."
            }

            SortableGroup {
                enable_merge: true,

                on_reorder: move |e: ReorderEvent| {
                    grouped_reorder_default(&workout, &e);
                },

                on_move: move |e: MoveEvent| {
                    grouped_move_default(&workout, &e);
                },

                on_merge: move |e: MergeEvent| {
                    grouped_merge(&workout, &e);
                },

                WorkoutList { items: workout }
            }
        }
    }
}

// ============================================================================
// WorkoutList Component (for DragContext access)
// ============================================================================

/// Inner component that renders the workout list with access to DragContext.
/// Partitions flat item list into nested SortableContexts for groups.
#[component]
fn WorkoutList(items: Signal<Vec<WorkoutItem>>) -> Element {
    let _ctx = use_context::<DragContext>();
    let items_signal = items;

    // Partition items into top-level entries (groups and standalone)
    let entries = partition_grouped_items(&items_signal.read());
    let top_level_ids: Vec<DragId> = entries.iter().map(|e| e.drag_id()).collect();

    rsx! {
        div { class: "workout-list",
            SortableContext {
                id: DragId::new("workout"),
                items: top_level_ids,
                drop_preview: Callback::new(move |active_drag: ActiveDrag| {
                    let items_read = items_signal.read();
                    if let Some(item) = items_read.iter().find(|i| i.drag_id() == active_drag.data.id) {
                        match item {
                            WorkoutItem::Exercise { name, sets, .. } => {
                                let sets_clone = sets.clone();
                                rsx! {
                                    div { class: "exercise-card ghost-preview",
                                        div { class: "exercise-name", "{name}" }
                                        div { class: "exercise-sets",
                                            for (i, set) in sets_clone.iter().enumerate() {
                                                {
                                                    let num = i + 1;
                                                    let reps = set.reps;
                                                    let weight = set.weight;
                                                    rsx! {
                                                        div { class: "set-row",
                                                            span { class: "set-number", "Set {num}" }
                                                            if weight > 0.0 {
                                                                span { class: "set-details", "{reps} reps @ {weight} lbs" }
                                                            } else {
                                                                span { class: "set-details", "{reps} reps (bodyweight)" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            WorkoutItem::SupersetHeader { id } => {
                                let members: Vec<_> = items_read.iter()
                                    .filter(|i| i.group_id() == Some(id) && !i.is_group_header())
                                    .collect();
                                rsx! {
                                    div { class: "group-container-preview ghost-preview",
                                        div { class: "superset-header ghost-preview",
                                            span { class: "drag-handle", "\u{22ee}\u{22ee}" }
                                            span { "SUPERSET" }
                                        }
                                        for member in members.iter() {
                                            if let WorkoutItem::Exercise { name, sets, .. } = member {
                                                {
                                                    let sets_clone = sets.clone();
                                                    rsx! {
                                                        div { class: "exercise-card ghost-preview",
                                                            div { class: "exercise-name", "{name}" }
                                                            div { class: "exercise-sets",
                                                                for (i, set) in sets_clone.iter().enumerate() {
                                                                    {
                                                                        let num = i + 1;
                                                                        let reps = set.reps;
                                                                        let weight = set.weight;
                                                                        rsx! {
                                                                            div { class: "set-row",
                                                                                span { class: "set-number", "Set {num}" }
                                                                                if weight > 0.0 {
                                                                                    span { class: "set-details", "{reps} reps @ {weight} lbs" }
                                                                                } else {
                                                                                    span { class: "set-details", "{reps} reps (bodyweight)" }
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
                                    }
                                }
                            }
                        }
                    } else {
                        VNode::empty()
                    }
                }),

                for entry in entries.iter() {
                    {render_top_level_entry(entry, items_signal)}
                }
            }

            DragOverlay {
                render: move |active_drag: ActiveDrag| {
                    let items_read = items_signal.read();
                    if let Some((index, item)) = items_read
                        .iter()
                        .enumerate()
                        .find(|(_, item)| item.drag_id() == active_drag.data.id)
                    {
                        let position = grouped_position(&items_read, index);
                        let style_info = grouped_style_info(position);

                        match item {
                            WorkoutItem::SupersetHeader { .. } => {
                                rsx! {
                                    div {
                                        class: "superset-header dragging-overlay",
                                        "data-group-role": "header",
                                        span { class: "drag-handle", "\u{22ee}\u{22ee}" }
                                        span { "SUPERSET" }
                                    }
                                }
                            }
                            WorkoutItem::Exercise {
                                name, sets, ..
                            } => {
                                let n = sets.len();
                                rsx! {
                                    div {
                                        class: "exercise-card dragging-overlay",
                                        "data-group-role": if style_info.data_group_role.is_some() { style_info.data_group_role } else { None },
                                        div { class: "exercise-name", "{name}" }
                                        div { class: "exercise-details", "{n} sets" }
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

/// Render a top-level entry (group or standalone item)
fn render_top_level_entry(
    entry: &TopLevelEntry<WorkoutItem>,
    items_signal: Signal<Vec<WorkoutItem>>,
) -> Element {
    match entry {
        TopLevelEntry::Group { group_id, items } => {
            let group_item_ids: Vec<DragId> = items.iter().map(|i| i.drag_id()).collect();
            let group_id_key = group_id.clone();

            rsx! {
                SortableContext {
                    key: "{group_id_key}",
                    id: DragId::new(group_id),
                    items: group_item_ids,
                    // Only accept "sortable" items — reject "group-header" type.
                    // This prevents group headers from being dropped into any group.
                    accepts: vec![DragType::new("sortable")],

                    for item in items.iter() {
                        {render_workout_item(item, items_signal)}
                    }
                }
            }
        }
        TopLevelEntry::Standalone(item) => render_workout_item(item, items_signal),
    }
}

/// Render a single workout item as a SortableItem
fn render_workout_item(item: &WorkoutItem, items_signal: Signal<Vec<WorkoutItem>>) -> Element {
    let items_read = items_signal.read();
    let index = items_read
        .iter()
        .position(|i| i.drag_id() == item.drag_id())
        .unwrap_or(0);
    let position = grouped_position(&items_read, index);
    let style_info = grouped_style_info(position);

    let drag_id = item.drag_id();
    let key = format!(
        "{}__{}",
        drag_id.as_str(),
        style_info.data_group_role.unwrap_or("standalone")
    );

    let content = match item {
        WorkoutItem::SupersetHeader { .. } => {
            rsx! {
                div {
                    class: "superset-header",
                    "data-group-role": "header",
                    span { class: "drag-handle", "\u{22ee}\u{22ee}" }
                    span { "SUPERSET" }
                }
            }
        }
        WorkoutItem::Exercise { name, sets, .. } => {
            let sets_clone = sets.clone();
            rsx! {
                div {
                    class: "exercise-card",
                    "data-group-role": if style_info.data_group_role.is_some() { style_info.data_group_role } else { None },
                    div { class: "exercise-name", "{name}" }
                    div { class: "exercise-sets",
                        for (i, set) in sets_clone.iter().enumerate() {
                            {
                                let num = i + 1;
                                let reps = set.reps;
                                let weight = set.weight;
                                rsx! {
                                    div { class: "set-row",
                                        span { class: "set-number", "Set {num}" }
                                        if weight > 0.0 {
                                            span { class: "set-details", "{reps} reps @ {weight} lbs" }
                                        } else {
                                            span { class: "set-details", "{reps} reps (bodyweight)" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    // Group headers get "group-header" type so nested containers can reject them.
    // This prevents headers from being dropped into any group (no displacement, no drop preview).
    let is_header = item.is_group_header();

    rsx! {
        SortableItem {
            key: "{key}",
            id: drag_id,
            drag_type: if is_header { Some(DragType::new("group-header")) } else { None },
            {content}
        }
    }
}

// ============================================================================
// Styles
// ============================================================================

const WORKOUT_STYLES: &str = r#"
/* ==========================================================================
   Workout App Layout
   ========================================================================== */

.workout-app {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    max-width: 500px;
    margin: 0 auto;
    padding: 2rem;
    background: #f8fafc;
    min-height: 100vh;
    /* Override grouped list theme tokens */
    --dxdnd-grouped-header-bg: linear-gradient(135deg, #4f46e5, #7c3aed);
    --dxdnd-grouped-header-color: #ffffff;
    --dxdnd-grouped-member-bg: #ffffff;
    --dxdnd-grouped-border: #e2e8f0;
    --dxdnd-grouped-radius: 0.75rem;
    --dxdnd-grouped-collapse-duration: 180ms;
}

.workout-app * {
    box-sizing: border-box;
}

body {
    background: #f8fafc;
    margin: 0;
}

.workout-app h1 {
    margin: 0 0 0.5rem;
    color: #1e293b;
}

.instructions {
    color: #64748b;
    margin-bottom: 2rem;
    font-size: 0.9rem;
    line-height: 1.5;
}

/* ==========================================================================
   Workout List
   ========================================================================== */

.workout-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

/* ==========================================================================
   Exercise Cards
   ========================================================================== */

.exercise-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 0.25rem;
}

.exercise-card:hover {
    transform: translateY(-1px);
}

.exercise-card:active {
    cursor: grabbing;
}

.exercise-name {
    font-weight: 500;
    color: #1e293b;
}

.exercise-details {
    color: #64748b;
    font-size: 0.9rem;
    font-weight: 500;
}

.exercise-sets {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 0.25rem;
}

.set-row {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0.5rem;
    font-size: 0.85rem;
    color: #64748b;
    background: #f1f5f9;
    border-radius: 0.25rem;
}

.set-number {
    font-weight: 500;
    color: #475569;
}

.set-details {
    color: #64748b;
}

/* ==========================================================================
   Superset Styling
   ========================================================================== */

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

.drag-handle {
    opacity: 0.7;
    font-size: 1rem;
}

/* ==========================================================================
   Dragging States
   ========================================================================== */

[data-dnd-item][data-state="dragging"] > *:not([data-dnd-indicator]):not([data-dnd-preview]) {
    opacity: 0.5;
}

.dragging-overlay {
    opacity: 0.95;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    transform: rotate(2deg);
}

/* ==========================================================================
   Ghost Preview (full-card ghost in displacement gap)
   ========================================================================== */

.ghost-preview {
    width: 100%;
}

.ghost-preview .set-row {
    background: rgba(241, 245, 249, 0.5);
}

.group-container-preview {
    display: flex;
    flex-direction: column;
    gap: var(--dxdnd-group-gap);
    padding: var(--dxdnd-group-padding);
    background: var(--dxdnd-bg-subtle);
    border: 1px solid var(--dxdnd-grouped-border);
    border-radius: var(--dxdnd-grouped-radius);
}

.group-container-preview .exercise-card {
    background: var(--dxdnd-bg);
    border: 1px solid var(--dxdnd-border);
    border-radius: var(--dxdnd-item-radius);
    padding: var(--dxdnd-item-padding);
}

/* ==========================================================================
   Drop Indicator
   ========================================================================== */

[data-dnd-indicator] {
    height: 3px;
    background: #4f46e5;
    border-radius: 2px;
    margin: 0.25rem 0;
}

@media (prefers-color-scheme: dark) {
    body {
        background: #0f172a;
    }

    .workout-app {
        background: #0f172a;
        --dxdnd-grouped-header-bg: linear-gradient(135deg, #4338ca, #6d28d9);
        --dxdnd-grouped-header-color: #e2e8f0;
        --dxdnd-grouped-member-bg: #1e293b;
        --dxdnd-grouped-border: #334155;
    }

    .workout-app h1 {
        color: #f1f5f9;
    }

    .instructions {
        color: #94a3b8;
    }

    .exercise-name {
        color: #f1f5f9;
    }

    .exercise-details {
        color: #94a3b8;
    }

    .set-row {
        background: #1e293b;
        color: #94a3b8;
    }

    .set-number {
        color: #cbd5e1;
    }

    .set-details {
        color: #94a3b8;
    }
}
"#;
