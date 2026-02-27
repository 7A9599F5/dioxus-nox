//! Sortable context provider
//!
//! Provides state and event handling for sortable lists.
//! When standalone, SortableContext creates its own DragContextProvider.
//! When inside a SortableGroup, it uses DropZone and lets the group handle drops.
//! When nested inside another SortableContext within a group, it registers as
//! a nested container (dual registration: item in parent + container for children).

use dioxus::prelude::*;
use dioxus_core::Task;

use crate::collision::CollisionStrategy;
use crate::context::{ActiveDrag, DragContext, DragContextProvider};
use crate::primitives::DropZone;
use crate::sortable_projection::{compute_displacement_offset, to_filtered_index};
use crate::types::{DragId, DragType, DropEvent, DropLocation, Orientation, Rect, ReorderEvent};
use crate::utils::{extract_attribute, filter_class_style};

use super::group::SortableGroupContext;
use super::indicator::DropIndicator;
use super::item::{IndicatorPosition, NextAnimationFrame};

// ============================================================================
// Sortable State
// ============================================================================

/// State for a sortable list
///
/// This is provided via context to child SortableItems so they can
/// access the container information and orientation. All fields are
/// signal-backed and stay in sync with the parent SortableContext props.
#[derive(Clone, Copy)]
pub struct SortableState {
    /// ID of the sortable container
    pub container_id: Signal<DragId>,
    /// Ordered list of item IDs in this sortable
    pub items: Signal<Vec<DragId>>,
    /// Layout orientation
    pub orientation: Signal<Orientation>,
    /// Optional callback to render a ghost preview in the drop indicator gap.
    /// When provided, SortableItem passes the result as children to DropIndicator.
    pub drop_preview: Option<Callback<ActiveDrag, Element>>,
}

// ============================================================================
// SortableContext Props
// ============================================================================

/// Props for the SortableContext component
#[derive(Props, Clone)]
pub struct SortableContextProps {
    /// Unique ID for this sortable container
    #[props(into)]
    pub id: DragId,

    /// The ordered list of item IDs
    pub items: Vec<DragId>,

    /// Layout orientation
    #[props(default)]
    pub orientation: Orientation,

    /// Called when order changes within this container
    /// If not provided and inside a SortableGroup, uses group's handler
    #[props(default)]
    pub on_reorder: EventHandler<ReorderEvent>,

    /// Types this container accepts (empty = all)
    ///
    /// Use this to create type-filtered sortable lists where different
    /// item types can only be dropped in containers that accept them.
    #[props(default)]
    pub accepts: Vec<DragType>,

    /// Collision detection strategy (default: Sortable)
    ///
    /// Override this to use different collision detection algorithms.
    #[props(default)]
    pub collision_detection: CollisionStrategy,

    /// Optional callback to render a ghost preview in the drop indicator gap.
    /// When provided, the active drag item is passed to the callback and the
    /// returned Element is rendered inside the DropIndicator instead of the thin line.
    #[props(default)]
    pub drop_preview: Option<Callback<ActiveDrag, Element>>,

    /// Whether items displace to create gaps (true, default) or stay in place
    /// with line indicators (false). Only affects standalone mode — when inside
    /// a SortableGroup, the group's setting takes precedence via DragContext.
    #[props(default = true)]
    pub gap_displacement: bool,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Forwarded to the container element. In standalone mode, forwarded to
    /// the inner DropZone div. In group mode, forwarded to the DropZone.
    /// In nested mode, forwarded to the outer group wrapper div.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Children (should contain SortableItems)
    pub children: Element,
}

/// Always returns `false`: props contain [`Element`] (children), [`EventHandler`]
/// (on_reorder), [`Callback`] (drop_preview), and [`Attribute`]s — none of which support
/// meaningful equality comparison. Returning `false` tells Dioxus to always re-render
/// this component, which is the intended behavior for reactive signal-driven updates.
impl PartialEq for SortableContextProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// ============================================================================
// SortableContext Component
// ============================================================================

/// A context provider for sortable lists
///
/// SortableContext provides ordering semantics for reordering items.
/// It operates in three modes:
///
/// 1. **Standalone** — Creates its own `DragContextProvider`
/// 2. **In group (top-level)** — Renders as `DropZone`, group handles drops
/// 3. **Nested (inside another SortableContext in a group)** — Registers as a
///    nested container with dual registration (item zone in parent + container
///    zone for children). This is used for structural grouping (e.g., supersets).
///
/// # Standalone Example
///
/// ```ignore
/// rsx! {
///     SortableContext {
///         id: DragId::new("todos"),
///         items: item_ids,
///         on_reorder: move |e: ReorderEvent| {
///             let mut list = items.write();
///             let item = list.remove(e.from_index);
///             list.insert(e.to_index, item);
///         },
///
///         for item in items.read().iter() {
///             SortableItem {
///                 key: "{item.id}",
///                 id: DragId::new(&item.id),
///                 div { "{item.text}" }
///             }
///         }
///     }
/// }
/// ```
///
/// # Nested (Grouped) Example
///
/// ```ignore
/// rsx! {
///     SortableGroup {
///         on_reorder: move |e| { /* ... */ },
///         on_move: move |e| { /* cross-container moves */ },
///
///         SortableContext {
///             id: DragId::new("workout"),
///             items: top_level_ids,  // group IDs + standalone IDs
///
///             // Nested group container
///             SortableContext {
///                 id: DragId::new("superset-1"),
///                 items: group_member_ids,
///                 SortableItem { id: "header", /* ... */ }
///                 SortableItem { id: "member-1", /* ... */ }
///             }
///
///             // Standalone items
///             SortableItem { id: "standalone-1", /* ... */ }
///         }
///     }
/// }
/// ```
#[component]
pub fn SortableContext(props: SortableContextProps) -> Element {
    // Detect environment ONCE on mount. This must run before use_context_provider
    // registers our own SortableState — otherwise try_consume_context finds it
    // on re-renders and incorrectly marks the top-level context as nested.
    let env = use_hook(|| {
        let in_group = try_consume_context::<SortableGroupContext>().is_some();
        let parent_sortable = try_consume_context::<SortableState>();
        let is_nested = in_group && parent_sortable.is_some();
        (in_group, parent_sortable, is_nested)
    });
    let in_group = env.0;
    let parent_sortable = env.1;
    let is_nested = env.2;

    // For nested contexts, children register with a derived inner container ID
    // to avoid HashMap key conflicts (item zone ID vs container zone ID).
    let inner_container_id = DragId::new(format!("{}-container", props.id.0));
    let effective_container_id = if is_nested {
        inner_container_id.clone()
    } else {
        props.id.clone()
    };

    // Create reactive signals for all props that children need
    let mut items = use_signal(|| props.items.clone());
    let mut container_id = use_signal(|| effective_container_id.clone());
    let mut orientation = use_signal(|| props.orientation);

    // Sync signals when props change
    if *items.peek() != props.items {
        items.set(props.items.clone());
    }
    if *container_id.peek() != effective_container_id {
        container_id.set(effective_container_id.clone());
    }
    if *orientation.peek() != props.orientation {
        orientation.set(props.orientation);
    }

    // Inherit drop_preview from parent SortableState when nested and not explicitly set.
    // This ensures nested containers (groups) show the same preview as the parent.
    let effective_drop_preview = props.drop_preview.or_else(|| {
        if is_nested {
            parent_sortable.and_then(|s| s.drop_preview)
        } else {
            None
        }
    });

    // Provide sortable state via context (created once, signals stay stable)
    use_context_provider(|| SortableState {
        container_id,
        items,
        orientation,
        drop_preview: effective_drop_preview,
    });

    // Signal for nested container's mounted element (used for rect measurement)
    let mut nested_node_ref: Signal<Option<MountedEvent>> = use_signal(|| None);

    // Track spawned nested registration task for cancellation
    let mut nested_task: Signal<Option<Task>> = use_signal(|| None);

    // Get DragContext if available (exists when in_group or nested)
    let drag_ctx = try_consume_context::<DragContext>();

    // --- Nested container registration effect ---
    // Called unconditionally (hooks must be in consistent order) but only
    // does work when is_nested is true.
    {
        let item_id = props.id.clone();
        let inner_id = inner_container_id.clone();
        let parent_cid = parent_sortable.map(|s| s.container_id.peek().clone());
        let accepts = props.accepts.clone();
        let props_orientation = props.orientation;

        use_effect(move || {
            if !is_nested {
                return;
            }
            let Some(parent_cid) = parent_cid.clone() else {
                return;
            };
            let Some(ctx) = drag_ctx else {
                return;
            };

            let mounted = nested_node_ref.read().clone();
            let item_id = item_id.clone();
            let inner_id = inner_id.clone();
            let accepts = accepts.clone();

            // Re-register when items change (rect may change)
            let _items = items.read();

            // Read measure generation so rects refresh on each drag start.
            let gen_sig = ctx.measure_generation_signal();
            let _gen = gen_sig.read();

            // Cancel any in-flight registration before spawning a new one
            if let Some(prev) = *nested_task.peek() {
                prev.cancel();
            }

            let task = spawn(async move {
                if let Some(mounted) = mounted {
                    // Wait for browser layout to complete
                    NextAnimationFrame::new().await;
                    NextAnimationFrame::new().await;

                    if let Ok(rect) = mounted.get_client_rect().await {
                        let owned_rect = Rect::new(
                            rect.origin.x,
                            rect.origin.y,
                            rect.size.width,
                            rect.size.height,
                        );

                        ctx.register_nested_container(
                            item_id,
                            parent_cid,
                            inner_id,
                            owned_rect,
                            accepts,
                            props_orientation,
                        );
                    }
                }
            });
            nested_task.set(Some(task));
        });
    }

    // --- Nested container cleanup ---
    {
        let cleanup_item_id = props.id.clone();
        let cleanup_inner_id = inner_container_id.clone();

        use_drop(move || {
            if let Some(task) = *nested_task.peek() {
                task.cancel();
            }
            if !is_nested {
                return;
            }
            if let Some(ctx) = drag_ctx {
                ctx.unregister(&cleanup_item_id);
                ctx.unregister(&cleanup_inner_id);
            }
        });
    }

    // --- Displacement transform for nested container ---
    // When nested, the container div needs to displace like a SortableItem
    // in the parent container (shift up/down when items are dragged around it).
    let nested_displacement = if is_nested {
        if let (Some(ctx), Some(parent)) = (drag_ctx, parent_sortable) {
            compute_nested_displacement(ctx, parent, &props.id)
        } else {
            "none".to_string()
        }
    } else {
        "none".to_string()
    };

    // Copy values for use in on_drop handler
    let handler_container_id = props.id.clone();
    let on_reorder = props.on_reorder;

    let orientation_attr = match props.orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    };

    // Extract consumer attributes for forwarding to the container element
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");
    let remaining_attrs = filter_class_style(props.attributes);

    if is_nested {
        // Nested inside another SortableContext in a group:
        // Render a plain div with manual registration via register_nested_container.
        // The div participates in parent-level displacement and provides a
        // container boundary for its children.

        // Container growth: when an item is dragged INTO this nested container
        // from outside, grow padding-bottom to accommodate the incoming item.
        // CSS transform doesn't affect layout, so we use inline padding-bottom.
        // Computed inline — signal reads subscribe the component to changes.
        let accepts_list = props.accepts.clone();
        let (growth, is_dragging_out) = drag_ctx
            .map(|ctx| {
                let active_sig = ctx.active_signal();
                let active = active_sig.read();
                let Some(active_drag) = active.as_ref() else {
                    return (0.0, false);
                };

                // Check if an item this container REJECTS is being dragged from within.
                // When true, the item is definitionally leaving (e.g., a group header
                // with type "group-header" leaving a container that only accepts "sortable").
                // Hide the container so the DragOverlay ghost is the only visual.
                let dragging_out = active_drag
                    .source_container_id
                    .as_ref()
                    .is_some_and(|src| *src == inner_container_id)
                    && !active_drag.data.has_any_type(&accepts_list);

                let target_sig = ctx.target_signal();
                let target = target_sig.read();

                // Container growth: target inside THIS container + source OUTSIDE
                // Skip in indicator mode — no displacement means no layout growth needed
                let growth_val = if !ctx.gap_displacement() {
                    0.0
                } else {
                    target
                        .as_ref()
                        .and_then(|drop_loc| {
                            if drop_loc.container_id() != inner_container_id {
                                return None;
                            }
                            if active_drag.source_container_id.as_ref() == Some(&inner_container_id)
                            {
                                return None;
                            }
                            ctx.get_zone_height(&active_drag.data.id)
                        })
                        .unwrap_or(0.0)
                };

                (growth_val, dragging_out)
            })
            .unwrap_or((0.0, false));

        let padding_style = if growth > 0.0 {
            let prop = match props.orientation {
                Orientation::Horizontal => "padding-right",
                Orientation::Vertical => "padding-bottom",
            };
            format!("{prop}: {}px;", 8.0 + growth)
        } else {
            String::new()
        };

        // Detect whether to show boundary indicators outside the group container.
        // This renders DropIndicator lines as siblings of the overflow:hidden container,
        // so they aren't clipped by border-radius overflow.
        let orientation = props.orientation;
        // Detect boundary indicators: external (parent-level Before/After the group)
        // vs internal (within-group Before first / After last item).
        // External = "between group and neighbor item" (line + glow).
        // Internal = "into top/bottom of group" (edge highlight only, no line).
        let (before_external, before_internal, after_external, after_internal) = drag_ctx
            .map(|ctx| {
                let target_sig = ctx.target_signal();
                let drop_loc = target_sig.read();
                let items_list = items.read();
                // For boundary detection with AtIndex, we need to know:
                // - External: Is the parent-level AtIndex targeting our group's position?
                //   We find our position by looking in the parent sortable's items.
                // - Internal: Is AtIndex targeting inner_container at index 0 or len?
                let my_pos_in_parent = parent_sortable.and_then(|ps| {
                    let parent_items = ps.items.read();
                    parent_items.iter().position(|id| *id == props.id)
                });
                let parent_cid = parent_sortable.map(|ps| ps.container_id.read().clone());

                let before_ext = match (drop_loc.as_ref(), &parent_cid, my_pos_in_parent) {
                    // Parent-level: AtIndex at this group's position in parent
                    (
                        Some(DropLocation::AtIndex {
                            container_id,
                            index,
                        }),
                        Some(pcid),
                        Some(my_pos),
                    ) if container_id == pcid && *index == my_pos => true,
                    _ => false,
                };

                let before_int = match drop_loc.as_ref() {
                    // Within-group top boundary: AtIndex { inner_cid, 0 }
                    Some(DropLocation::AtIndex {
                        container_id,
                        index,
                    }) if *container_id == inner_container_id && *index == 0 => true,
                    _ => false,
                };

                let after_ext = match (drop_loc.as_ref(), &parent_cid, my_pos_in_parent) {
                    // Parent-level: AtIndex at this group's position + 1 in parent
                    (
                        Some(DropLocation::AtIndex {
                            container_id,
                            index,
                        }),
                        Some(pcid),
                        Some(my_pos),
                    ) if container_id == pcid && *index == my_pos + 1 => true,
                    _ => false,
                };

                let after_int = match drop_loc.as_ref() {
                    // Within-group bottom boundary: AtIndex { inner_cid, child_count }
                    Some(DropLocation::AtIndex {
                        container_id,
                        index,
                    }) if *container_id == inner_container_id && *index == items_list.len() => true,
                    _ => false,
                };

                (before_ext, before_int, after_ext, after_int)
            })
            .unwrap_or((false, false, false, false));

        let wrapper_drop_pos = if before_external {
            "before"
        } else if after_external {
            "after"
        } else if before_internal {
            "into-top"
        } else if after_internal {
            "into-bottom"
        } else {
            ""
        };

        let group_state = if is_dragging_out { "dragging-out" } else { "" };

        let merged_class = consumer_class.unwrap_or_default();

        rsx! {
            div {
                class: "{merged_class}",
                style: "{nested_displacement}",
                "data-dnd-group-wrapper": "",
                "data-drop-position": "{wrapper_drop_pos}",
                ..remaining_attrs,

                // External (parent-level) drops get the full DropIndicator line
                // between the group and its neighbor item.
                // Internal (within-group) drops only get the edge highlight
                // via CSS — no external line to avoid "between items" visual.
                if before_external {
                    DropIndicator { orientation, position: IndicatorPosition::Before }
                }

                div {
                    style: "{padding_style}",
                    "data-dnd-container": "",
                    "data-dnd-group": "",
                    "data-orientation": "{orientation_attr}",
                    "data-state": "{group_state}",
                    role: "list",
                    onmounted: move |data| {
                        nested_node_ref.set(Some(data.clone()));
                    },

                    {props.children}
                }

                if after_external {
                    DropIndicator { orientation, position: IndicatorPosition::After }
                }
            }
        }
    } else if in_group {
        // Inside SortableGroup (top-level): use DropZone, let group handle drops
        let merged_class = consumer_class.unwrap_or_default();
        let merged_style = consumer_style.unwrap_or_default();
        rsx! {
            DropZone {
                id: props.id.clone(),
                accepts: props.accepts.clone(),
                orientation: props.orientation,
                class: "{merged_class}",
                style: "{merged_style}",
                "data-dnd-container": "",
                "data-orientation": "{orientation_attr}",
                role: "list",
                on_drop: move |event: DropEvent| {
                    let current_items = items.read();
                    let from_index = current_items
                        .iter()
                        .position(|id| id == &event.dragged.id);

                    if let Some(from) = from_index {
                        let to_index = event.location.resolve_drop_index(&current_items);
                        if from != to_index {
                            on_reorder.call(ReorderEvent {
                                container_id: handler_container_id.clone(),
                                from_index: from,
                                to_index,
                                item_id: event.dragged.id,
                            });
                        }
                    }
                },

                {props.children}
            }
        }
    } else {
        // Standalone: create our own DragContextProvider with specified collision detection
        let collision_strategy = if props.collision_detection == CollisionStrategy::default() {
            CollisionStrategy::Sortable
        } else {
            props.collision_detection
        };

        let merged_class = consumer_class.unwrap_or_default();
        let merged_style = consumer_style.unwrap_or_default();
        rsx! {
            DragContextProvider {
                collision_detection: collision_strategy,
                gap_displacement: props.gap_displacement,
                on_drop: move |event: DropEvent| {
                    let current_items = items.read();

                    let from_index = current_items
                        .iter()
                        .position(|id| id == &event.dragged.id);

                    if let Some(from) = from_index {
                        let to_index = event.location.resolve_drop_index(&current_items);
                        if from != to_index {
                            on_reorder.call(ReorderEvent {
                                container_id: handler_container_id.clone(),
                                from_index: from,
                                to_index,
                                item_id: event.dragged.id,
                            });
                        }
                    }
                },

                DropZone {
                    id: props.id.clone(),
                    accepts: props.accepts.clone(),
                    orientation: props.orientation,
                    class: "{merged_class}",
                    style: "{merged_style}",
                    "data-dnd-container": "",
                    "data-orientation": "{orientation_attr}",
                    role: "list",
                    {props.children}
                }
            }
        }
    }
}

// ============================================================================
// Nested Displacement Helper
// ============================================================================

/// Compute displacement style for a nested container.
/// Calculates how much this nested container should shift to make room for
/// a dragged item being reordered in the parent container.
///
/// Returns transform + transition duration variable for nested displacement.
fn compute_nested_displacement(
    ctx: DragContext,
    parent: SortableState,
    my_item_id: &DragId,
) -> String {
    let style_with_duration = |transform: String, instant: bool| {
        let duration = if instant {
            "0ms"
        } else {
            "var(--dxdnd-transition-displacement-duration)"
        };
        format!("transform: {transform}; --dxdnd-displacement-duration: {duration}")
    };

    let active_signal = ctx.active_signal();

    // Extract dragged ID and source container from active drag signal
    let (dragged_id_opt, source_container_opt) = {
        let active = active_signal.read();
        (
            active.as_ref().map(|d| d.data.id.clone()),
            active.as_ref().and_then(|d| d.source_container_id.clone()),
        )
    };

    // Use projected (pending-preferred) target so nested displacement stays
    // in sync with collision/displacement during hysteresis windows.
    let current_drop_loc = ctx.projected_drop_location();
    let parent_items = parent.items.read();

    // If this group is being dragged, no displacement
    if let Some(ref dragged_id) = dragged_id_opt {
        if dragged_id == my_item_id {
            return style_with_duration("none".to_string(), false);
        }
    }

    // Indicator mode: no displacement for nested containers either
    if !ctx.gap_displacement() {
        return style_with_duration("none".to_string(), false);
    }

    // Read traversal signals for continuous displacement.
    let traversal_id_sig = ctx.traversal_item_signal();
    let traversal_id = traversal_id_sig.read();
    let is_traversal = traversal_id.as_ref() == Some(my_item_id);

    // Check if this nested container just exited traversal (snap window).
    let is_exiting_traversal = ctx
        .previous_traversal_signal()
        .peek()
        .as_ref()
        .map_or(false, |(id, _)| id == my_item_id);

    let snap_style =
        |transform: String| -> String { style_with_duration(transform, is_exiting_traversal) };

    // ONLY the traversal item reads fraction (60fps subscription — one subscriber).
    let traversal_frac = if is_traversal {
        *ctx.traversal_fraction_signal().read()
    } else {
        0.0
    };

    // Orientation-aware axis: "Y" for vertical, "X" for horizontal
    let orientation = *parent.orientation.read();
    let axis = match orientation {
        Orientation::Vertical => "Y",
        Orientation::Horizontal => "X",
    };

    // Non-reactive peek for own size along orientation axis.
    let my_size = ctx.get_zone_size(my_item_id).unwrap_or(0.0);

    let my_index = parent_items.iter().position(|id| id == my_item_id);

    if let (Some(dragged_id), Some(my_idx)) = (dragged_id_opt, my_index) {
        let direct_source = parent_items.iter().position(|id| id == &dragged_id);
        let parent_container = parent.container_id.read().clone();

        // Resolve effective source for nested container drags (group drag).
        let (source_index, effective_size) = if direct_source.is_some() {
            (direct_source, ctx.get_zone_size(&dragged_id))
        } else {
            let nested_source = source_container_opt
                .as_ref()
                .and_then(|src_cid| ctx.find_nested_parent(src_cid))
                .and_then(|parent_id| {
                    parent_items
                        .iter()
                        .position(|id| id == &parent_id)
                        .map(|idx| (idx, parent_id))
                });
            if let Some((idx, parent_id)) = nested_source {
                (Some(idx), ctx.get_zone_size(&parent_id))
            } else {
                (None, ctx.get_zone_size(&dragged_id))
            }
        };

        let full_shift = |negative: bool| -> String {
            snap_style(match effective_size {
                Some(h) => {
                    let px = if negative { -h } else { h };
                    format!("translate{axis}({px}px)")
                }
                None => "none".to_string(),
            })
        };

        let has_any_target = current_drop_loc.is_some();

        // Determine target index in parent container
        let target_index = if let Some(ref loc) = current_drop_loc {
            if loc.container_id() == parent_container {
                Some(loc.resolve_drop_index(&parent_items))
            } else {
                None
            }
        } else {
            None
        };

        // Continuous traversal: this nested container is being smoothly traversed.
        // Invert fraction for upward drags (same logic as compute_displacement).
        if is_traversal {
            if let Some(src) = source_index {
                let h = effective_size.unwrap_or(my_size);
                // Normalize to entry→exit direction, apply easing when merge enabled
                let progress = if src < my_idx {
                    traversal_frac // DOWN/RIGHT: already 0→1
                } else {
                    1.0 - traversal_frac // UP/LEFT: invert to 0→1
                };
                let eased = if ctx.is_merge_enabled() {
                    super::item::ease_traversal(progress)
                } else {
                    progress
                };
                let px = if src < my_idx { -h * eased } else { h * eased };
                return style_with_duration(format!("translate{axis}({px}px)"), true);
            }
        }

        // Check if target is inside a nested child container (for parent-level expansion)
        let nested_group_idx = if has_any_target {
            current_drop_loc.as_ref().and_then(|loc| {
                let target_cid = loc.container_id();
                if target_cid != parent_container {
                    ctx.find_nested_parent(&target_cid)
                        .and_then(|pid| parent_items.iter().position(|id| id == &pid))
                } else {
                    None
                }
            })
        } else {
            None
        };

        // Nested-sibling drag-out combines collapse + expansion; keep this
        // separate from canonical projection.
        if let (Some(src), None) = (source_index, target_index) {
            if has_any_target {
                if let Some(group_idx) = nested_group_idx {
                    let collapse = my_idx > src;
                    let expansion = my_idx > group_idx;
                    match (collapse, expansion) {
                        (true, false) => return full_shift(true),
                        (false, true) => return full_shift(false),
                        _ => {}
                    }
                }
            }
        }

        // Canonical projection shared with collision detection.
        let dragged_size = effective_size.unwrap_or(my_size);
        let canonical_offset = match (source_index, target_index) {
            (Some(src), Some(tgt)) => {
                let my_filtered = to_filtered_index(my_idx, src);
                compute_displacement_offset(
                    my_filtered,
                    Some(src),
                    Some(tgt),
                    false,
                    my_size,
                    dragged_size,
                )
            }
            (Some(src), None) if has_any_target && nested_group_idx.is_none() => {
                let my_filtered = to_filtered_index(my_idx, src);
                compute_displacement_offset(
                    my_filtered,
                    Some(src),
                    None,
                    false,
                    my_size,
                    dragged_size,
                )
            }
            (None, Some(tgt)) => {
                compute_displacement_offset(my_idx, None, Some(tgt), false, my_size, dragged_size)
            }
            _ => 0.0,
        };

        if canonical_offset.abs() > f64::EPSILON {
            return snap_style(format!("translate{axis}({canonical_offset}px)"));
        }

        // Remaining fallback for expansion-only paths.
        match (source_index, target_index) {
            // (Some(_), None) handled by canonical projection.
            (None, None) => {
                if let Some(group_idx) = nested_group_idx {
                    if my_idx > group_idx {
                        return full_shift(false);
                    }
                }
            }
            _ => {}
        }
    }

    if is_exiting_traversal {
        return style_with_duration("none".to_string(), true);
    }

    style_with_duration("none".to_string(), false)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DragType, DropLocation};

    #[test]
    fn test_resolve_drop_index_at_index() {
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];

        let location = DropLocation::AtIndex {
            container_id: DragId::new("container"),
            index: 1,
        };

        assert_eq!(location.resolve_drop_index(&items), 1);
    }

    #[test]
    fn test_resolve_drop_index_at_index_middle() {
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];

        // AtIndex { index: 1 } means "insert at position 1"
        let location = DropLocation::AtIndex {
            container_id: DragId::new("container"),
            index: 1,
        };

        assert_eq!(location.resolve_drop_index(&items), 1);
    }

    #[test]
    fn test_resolve_drop_index_at_index_end() {
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];

        // AtIndex { index: 3 } means "insert after all items"
        let location = DropLocation::AtIndex {
            container_id: DragId::new("container"),
            index: 3,
        };

        assert_eq!(location.resolve_drop_index(&items), 3);
    }

    #[test]
    fn test_resolve_drop_index_into_container() {
        let items = vec![DragId::new("a"), DragId::new("b")];

        let location = DropLocation::IntoContainer {
            container_id: DragId::new("container"),
        };

        assert_eq!(location.resolve_drop_index(&items), 2);
    }

    #[test]
    fn test_resolve_drop_index_into_item() {
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];

        // IntoItem resolves to the item's position
        let location = DropLocation::IntoItem {
            container_id: DragId::new("container"),
            item_id: DragId::new("b"),
        };

        assert_eq!(location.resolve_drop_index(&items), 1);
    }

    #[test]
    fn test_resolve_drop_index_into_item_not_found() {
        let items = vec![DragId::new("a"), DragId::new("b")];

        let location = DropLocation::IntoItem {
            container_id: DragId::new("container"),
            item_id: DragId::new("not_found"),
        };

        // Not found defaults to items.len()
        assert_eq!(location.resolve_drop_index(&items), 2);
    }

    #[test]
    fn test_sortable_context_accepts_type_filtering() {
        let accepts = vec![DragType::new("image"), DragType::new("document")];
        let image_data = crate::types::DragData::with_types(
            "item-1",
            vec![DragType::new("sortable"), DragType::new("image")],
        );
        let text_data = crate::types::DragData::new("item-2", "text");

        assert!(
            image_data.has_any_type(&accepts),
            "image item should be accepted"
        );
        assert!(
            !text_data.has_any_type(&accepts),
            "text item should be rejected"
        );
    }

    #[test]
    fn test_inner_container_id_derivation() {
        // Verify the naming convention for nested container IDs
        let id = DragId::new("superset-1");
        let inner_id = DragId::new(format!("{}-container", id.0));
        assert_eq!(inner_id, DragId::new("superset-1-container"));
    }
}
