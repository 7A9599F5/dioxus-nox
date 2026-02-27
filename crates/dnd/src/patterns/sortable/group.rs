//! Sortable group component
//!
//! Enables dragging between multiple sortable containers (Kanban boards, etc.)
//! SortableGroup creates a shared DragContextProvider and provides shared handlers
//! to child SortableContext components via context.
//!
//! ## Architecture
//!
//! SortableGroup creates a single DragContextProvider that:
//! - Provides shared drag state across all containers (required for cross-container drag)
//! - Handles collision detection via sortable algorithm
//! - Routes drops to the appropriate handler (on_reorder or on_move)
//!
//! Child SortableContext components detect which container received the drop
//! and whether it's a same-container reorder or cross-container move.

use dioxus::prelude::*;

use crate::collision::CollisionStrategy;
use crate::context::DragContextProvider;
use crate::types::{DropEvent, DropLocation, MergeEvent, MoveEvent, ReorderEvent};
use crate::utils::{extract_attribute, filter_class_style};

// ============================================================================
// Sortable Group Context
// ============================================================================

/// Context provided by SortableGroup to child SortableContexts
///
/// This allows child SortableContext components to inherit shared handlers
/// and know they're inside a group (so they don't create their own DragContextProvider).
#[derive(Clone)]
pub struct SortableGroupContext {
    /// Shared reorder handler for all containers in the group
    pub on_reorder: EventHandler<ReorderEvent>,
    /// Shared move handler for cross-container moves
    pub on_move: EventHandler<MoveEvent>,
    /// Shared merge handler for item merge operations
    pub on_merge: EventHandler<MergeEvent>,
}

// ============================================================================
// SortableGroup Props
// ============================================================================

/// Props for the SortableGroup component
#[derive(Props, Clone)]
pub struct SortableGroupProps {
    /// Called when item reorders within same container
    #[props(default)]
    pub on_reorder: EventHandler<ReorderEvent>,

    /// Called when item moves between containers
    #[props(default)]
    pub on_move: EventHandler<MoveEvent>,

    /// Called when item is dropped onto another item (merge operation)
    /// Only fires when `enable_merge` is true and drop is in center zone
    #[props(default)]
    pub on_merge: EventHandler<MergeEvent>,

    /// Enable merge zones (30/40/30 split instead of 50/50)
    /// When true, collision detection uses 3 zones: Before/IntoItem/After
    #[props(default = false)]
    pub enable_merge: bool,

    /// Whether items displace to create gaps (true, default) or stay in place
    /// with line indicators (false). When `enable_merge` is true and this is
    /// not explicitly set, defaults to false (indicator mode) since merge
    /// works better without displacement gaps.
    #[props(default = true)]
    pub gap_displacement: bool,

    /// Collision detection strategy (default: Sortable, or SortableWithMerge if enable_merge)
    #[props(default)]
    pub collision_detection: CollisionStrategy,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Forwarded to the underlying DragContextProvider's wrapper div.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Children (should contain SortableContext components)
    pub children: Element,
}

/// Always returns `false`: props contain [`Element`] (children), [`EventHandler`]s
/// (on_reorder, on_move, on_merge), and [`Attribute`]s — none of which support meaningful
/// equality comparison. Returning `false` tells Dioxus to always re-render this
/// component, which is the intended behavior for reactive signal-driven updates.
impl PartialEq for SortableGroupProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// ============================================================================
// Drop Routing
// ============================================================================

#[derive(Clone, Debug)]
enum GroupDropAction {
    Reorder(ReorderEvent),
    Move(MoveEvent),
    Merge(MergeEvent),
    Ignore,
}

fn route_group_drop(event: &DropEvent, merge_enabled: bool) -> GroupDropAction {
    // IntoItem is only meaningful when merge is enabled. If an IntoItem arrives
    // while merge is disabled (or source metadata is missing), ignore defensively
    // instead of mutating list order unexpectedly.
    if let DropLocation::IntoItem {
        container_id: target_container_id,
        item_id: target_id,
    } = &event.location
    {
        if !merge_enabled {
            return GroupDropAction::Ignore;
        }

        let Some(from_container) = event.source_container.clone() else {
            return GroupDropAction::Ignore;
        };

        return GroupDropAction::Merge(MergeEvent {
            from_container,
            to_container: target_container_id.clone(),
            item_id: event.dragged.id.clone(),
            target_id: target_id.clone(),
        });
    }

    let target_container = event.location.container_id();
    let source_container = event
        .source_container
        .clone()
        .unwrap_or_else(|| target_container.clone());
    let to_index = match &event.location {
        DropLocation::AtIndex { index, .. } => *index,
        _ => 0,
    };

    if source_container == target_container {
        GroupDropAction::Reorder(ReorderEvent {
            container_id: target_container,
            from_index: event.source_index.unwrap_or(0),
            to_index,
            item_id: event.dragged.id.clone(),
        })
    } else {
        GroupDropAction::Move(MoveEvent {
            item_id: event.dragged.id.clone(),
            from_container: source_container,
            from_index: event.source_index.unwrap_or(0),
            to_container: target_container,
            to_index,
        })
    }
}

// ============================================================================
// SortableGroup Component
// ============================================================================

/// A group of sortable containers that can share items between them
///
/// SortableGroup creates a shared `DragContextProvider` for all child containers,
/// enabling cross-container drag-and-drop. It also provides shared handlers via
/// context that child `SortableContext` components inherit.
///
/// ## How it works
///
/// 1. SortableGroup creates a DragContextProvider (shared drag state)
/// 2. SortableGroup provides SortableGroupContext with shared handlers
/// 3. Child SortableContext components detect they're in a group and don't create
///    their own DragContextProvider
/// 4. When a drop occurs, SortableGroup routes to on_reorder or on_move
///
/// # Example
///
/// ```ignore
/// rsx! {
///     SortableGroup {
///         on_move: move |e: MoveEvent| {
///             // Handle cross-container moves
///         },
///         on_reorder: move |e: ReorderEvent| {
///             // Handle same-container reorders
///         },
///
///         // Kanban columns - handlers inherited from SortableGroup
///         SortableContext {
///             id: DragId::new("todo"),
///             items: todo_ids,
///             // No on_reorder needed - inherited from group!
///             // ...items
///         }
///         SortableContext {
///             id: DragId::new("doing"),
///             items: doing_ids,
///             // ...items
///         }
///
///         DragOverlay {
///             // Overlay content
///         }
///     }
/// }
/// ```
#[component]
pub fn SortableGroup(props: SortableGroupProps) -> Element {
    // Provide group context so children know they're in a group and can inherit handlers
    use_context_provider(|| SortableGroupContext {
        on_reorder: props.on_reorder,
        on_move: props.on_move,
        on_merge: props.on_merge,
    });

    // Copy handlers for use in on_drop closure
    let on_move = props.on_move;
    let on_reorder = props.on_reorder;
    let on_merge = props.on_merge;
    // SortableGroup's merge contract is controlled by enable_merge.
    let merge_enabled = props.enable_merge;

    // Use SortableWithMerge when enable_merge is true, otherwise use provided or default
    let collision_strategy = if merge_enabled {
        CollisionStrategy::SortableWithMerge
    } else if props.collision_detection == CollisionStrategy::default() {
        CollisionStrategy::Sortable
    } else if props.collision_detection == CollisionStrategy::SortableWithMerge {
        // Coerce explicit SortableWithMerge to Sortable when merge is disabled.
        // This keeps merge behavior behind a single, explicit prop.
        CollisionStrategy::Sortable
    } else {
        props.collision_detection
    };

    // Auto-derive gap_displacement: when merge is enabled, force indicator mode
    // (items stay in place, line indicators). Merge works better without displacement
    // gaps since IntoItem targets don't need to be displaced first.
    let effective_gap_displacement = if merge_enabled {
        false
    } else {
        props.gap_displacement
    };

    // Extract consumer attributes and forward to a wrapper div
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");
    let remaining_attrs = filter_class_style(props.attributes);
    let merged_class = consumer_class.unwrap_or_default();
    let merged_style = consumer_style.unwrap_or_default();

    // Create a shared DragContextProvider for all containers
    // This is required for cross-container drag to work
    rsx! {
        div {
            class: "{merged_class}",
            style: "display: contents; {merged_style}",
            ..remaining_attrs,

            DragContextProvider {
                collision_detection: collision_strategy,
                gap_displacement: effective_gap_displacement,
                on_drop: move |event: DropEvent| {
                    match route_group_drop(&event, merge_enabled) {
                        GroupDropAction::Reorder(e) => on_reorder.call(e),
                        GroupDropAction::Move(e) => on_move.call(e),
                        GroupDropAction::Merge(e) => on_merge.call(e),
                        GroupDropAction::Ignore => {}
                    }
                },

                {props.children}
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{route_group_drop, GroupDropAction};
    use crate::types::{DragData, DragId, DropEvent, DropLocation};

    fn make_drop_event(
        item_id: &str,
        location: DropLocation,
        source_container: Option<&str>,
        source_index: Option<usize>,
    ) -> DropEvent {
        DropEvent {
            dragged: DragData::new(item_id, "sortable"),
            location,
            source: DragId::new(item_id),
            source_container: source_container.map(DragId::new),
            source_index,
        }
    }

    #[test]
    fn test_sortable_group_props_partial_eq() {
        // Compile-time check that PartialEq is implemented for SortableGroupProps.
        // If this test compiles, the derive is working correctly.
    }

    #[test]
    fn test_route_group_drop_same_container_at_index_reorder() {
        let event = make_drop_event(
            "item-a",
            DropLocation::AtIndex {
                container_id: DragId::new("list"),
                index: 2,
            },
            Some("list"),
            Some(0),
        );

        let action = route_group_drop(&event, false);
        match action {
            GroupDropAction::Reorder(e) => {
                assert_eq!(e.container_id, DragId::new("list"));
                assert_eq!(e.item_id, DragId::new("item-a"));
                assert_eq!(e.from_index, 0);
                assert_eq!(e.to_index, 2);
            }
            _ => panic!("expected Reorder action"),
        }
    }

    #[test]
    fn test_route_group_drop_cross_container_at_index_move() {
        let event = make_drop_event(
            "item-a",
            DropLocation::AtIndex {
                container_id: DragId::new("dst"),
                index: 1,
            },
            Some("src"),
            Some(3),
        );

        let action = route_group_drop(&event, false);
        match action {
            GroupDropAction::Move(e) => {
                assert_eq!(e.item_id, DragId::new("item-a"));
                assert_eq!(e.from_container, DragId::new("src"));
                assert_eq!(e.to_container, DragId::new("dst"));
                assert_eq!(e.from_index, 3);
                assert_eq!(e.to_index, 1);
            }
            _ => panic!("expected Move action"),
        }
    }

    #[test]
    fn test_route_group_drop_into_item_with_merge_enabled_routes_merge() {
        let event = make_drop_event(
            "item-a",
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-b"),
            },
            Some("list"),
            Some(0),
        );

        let action = route_group_drop(&event, true);
        match action {
            GroupDropAction::Merge(e) => {
                assert_eq!(e.item_id, DragId::new("item-a"));
                assert_eq!(e.target_id, DragId::new("item-b"));
                assert_eq!(e.from_container, DragId::new("list"));
                assert_eq!(e.to_container, DragId::new("list"));
            }
            _ => panic!("expected Merge action"),
        }
    }

    #[test]
    fn test_route_group_drop_into_item_with_merge_disabled_ignores() {
        let event = make_drop_event(
            "item-a",
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-b"),
            },
            Some("list"),
            Some(0),
        );

        let action = route_group_drop(&event, false);
        assert!(matches!(action, GroupDropAction::Ignore));
    }

    #[test]
    fn test_route_group_drop_into_item_merge_enabled_missing_source_ignores() {
        let event = make_drop_event(
            "item-a",
            DropLocation::IntoItem {
                container_id: DragId::new("list"),
                item_id: DragId::new("item-b"),
            },
            None,
            Some(0),
        );

        let action = route_group_drop(&event, true);
        assert!(matches!(action, GroupDropAction::Ignore));
    }
}
