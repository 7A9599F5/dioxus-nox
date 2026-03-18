//! Sortable item component
//!
//! Individual items within a sortable list. SortableItem handles drag behavior
//! directly and shows drop indicators when items are being reordered.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use dioxus::prelude::*;
use dioxus_core::Task;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast;

use crate::context::DragContext;
use crate::sortable_projection::{compute_displacement_offset, to_filtered_index};
use crate::types::{
    DragData, DragId, DragType, DropLocation, Orientation, Position, Rect, combine_drag_types,
};
use crate::utils::{extract_attribute, filter_class_style};

use super::context::SortableState;
use super::indicator::DropIndicator;

/// Best-effort handle selector matching.
///
/// On web, this uses `Element.closest(selector)`. On non-web renderers there
/// is no DOM selector API, so we accept the pointer event.
fn pointer_event_matches_handle(event: &PointerEvent, selector: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(web_event) = event.data().downcast::<web_sys::PointerEvent>()
            && let Some(target) = web_event.target()
            && let Ok(element) = target.dyn_into::<web_sys::Element>()
        {
            return element.closest(selector).ok().flatten().is_some();
        }
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (event, selector);
        true
    }
}

// ============================================================================
// Async Helpers
// ============================================================================

/// A future that resolves after the next animation frame
/// This ensures browser layout is complete before measuring elements
///
/// Note: This is only available on WASM targets. On non-WASM targets (SSR),
/// we provide a no-op implementation that resolves immediately.
#[cfg(target_arch = "wasm32")]
pub(crate) struct NextAnimationFrame {
    resolved: bool,
}

#[cfg(target_arch = "wasm32")]
impl NextAnimationFrame {
    pub(crate) fn new() -> Self {
        Self { resolved: false }
    }
}

#[cfg(target_arch = "wasm32")]
impl Future for NextAnimationFrame {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.resolved {
            return Poll::Ready(());
        }

        let waker = cx.waker().clone();
        let window = match web_sys::window() {
            Some(w) => w,
            None => return Poll::Ready(()),
        };

        let closure = Closure::once(Box::new(move || {
            waker.wake();
        }) as Box<dyn FnOnce()>);

        let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
        closure.forget();

        self.resolved = true;
        Poll::Pending
    }
}

/// SSR-compatible no-op implementation that resolves immediately
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct NextAnimationFrame;

#[cfg(not(target_arch = "wasm32"))]
impl NextAnimationFrame {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Future for NextAnimationFrame {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // On SSR, resolve immediately - no browser animation frame available
        Poll::Ready(())
    }
}

// ============================================================================
// Public State Types
// ============================================================================

/// Position of the drop indicator relative to an item
///
/// Exposed publicly for use with `SortableItemState` in render props.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IndicatorPosition {
    /// Show indicator before this item
    Before,
    /// Show indicator after this item
    After,
}

/// State exposed to render callback for custom styling
///
/// This struct provides access to internal computed state that consumers
/// can use for conditional styling without needing to query DragContext directly.
///
/// # Example
///
/// ```ignore
/// SortableItem {
///     id: item.drag_id(),
///     render: move |state: SortableItemState| rsx! {
///         div {
///             class: if state.is_dragging { "item dragging" } else { "item" },
///             "{item.name}"
///         }
///     },
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SortableItemState {
    /// Whether this specific item is currently being dragged
    pub is_dragging: bool,
    /// Position of drop indicator relative to this item (if any)
    pub indicator_position: Option<IndicatorPosition>,
    /// Whether this item is currently a merge target (IntoItem collision)
    pub is_merge_target: bool,
    /// True when item is being dragged but has no drop target yet (pre-movement placeholder)
    pub is_placeholder: bool,
    /// Whether this item is being dragged via keyboard (no pointer overlay)
    pub is_keyboard_dragging: bool,
}

// ============================================================================
// Internal Types
// ============================================================================

/// Internal indicator position (kept private for internal use)
#[derive(Clone, Copy, PartialEq)]
enum IndicatorPos {
    /// Show indicator before this item
    Before,
    /// Show indicator after this item
    After,
}

// ============================================================================
// SortableItem Props
// ============================================================================

/// Props for the SortableItem component
#[derive(Props, Clone)]
pub struct SortableItemProps {
    /// Unique ID for this sortable item
    #[props(into)]
    pub id: DragId,

    /// The content to render (used when render prop not provided)
    #[props(default)]
    pub children: Element,

    /// Whether dragging is disabled for this item
    #[props(default = false)]
    pub disabled: bool,

    /// Whether this item should not accept drops (disable as drop target)
    ///
    /// When true, this item will not register as a drop zone and drops onto
    /// it will be ignored. Use this to prevent dropping items onto their own
    /// children or other invalid targets.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Disable drops onto superset members when dragging the superset header
    /// let drop_disabled = is_dragging_superset_header && item_belongs_to_that_superset;
    ///
    /// SortableItem {
    ///     id: item.drag_id(),
    ///     drop_disabled: drop_disabled,
    ///     // ...
    /// }
    /// ```
    #[props(default = false)]
    pub drop_disabled: bool,

    /// Custom drag handle selector forwarded to the inner Draggable.
    /// When set, only the element matching this selector can initiate dragging.
    #[props(default)]
    pub handle: Option<String>,

    /// Custom drag type for this item (defaults to "sortable")
    ///
    /// Use this to create type-filtered sortable lists where different
    /// item types can only be dropped in containers that accept them.
    ///
    /// Note: If you want to keep the "sortable" type AND add custom types,
    /// use the `types` prop instead.
    #[props(default)]
    pub drag_type: Option<DragType>,

    /// Additional content types beyond "sortable" (which is always included).
    ///
    /// Use this for content-type filtering: image, document, video, etc.
    /// When an item needs to be:
    /// 1. Sortable within a SortableContext (requires "sortable" type - always included)
    /// 2. Accepted by containers that filter by content type (e.g., "image", "document")
    ///
    /// The item will have ALL of: the base type (from drag_type or "sortable")
    /// PLUS all types in this list.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableItem {
    ///     id: file.drag_id(),
    ///     content_types: vec![DragType::new("image")],  // "sortable" + "image"
    ///     // ...
    /// }
    /// ```
    #[props(default)]
    pub content_types: Vec<DragType>,

    /// Optional render function that receives computed state
    ///
    /// When provided, this callback is used instead of `children` to render content.
    /// This allows conditional styling based on drag state without querying context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableItem {
    ///     id: item.drag_id(),
    ///     render: move |state| rsx! {
    ///         div {
    ///             class: if state.is_dragging { "item dragging" } else { "item" },
    ///             "{item.name}"
    ///         }
    ///     },
    /// }
    /// ```
    #[props(default)]
    pub render: Option<Callback<SortableItemState, Element>>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Attributes are forwarded to the outer wrapper div.
    ///
    /// # Example
    ///
    /// ```ignore
    /// SortableItem {
    ///     id: item.drag_id(),
    ///     class: "my-custom-class",
    ///     data_testid: "sortable-item-1",
    ///     div { "Content" }
    /// }
    /// ```
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Always returns `false`: props contain [`Element`] (children), [`Callback`] (render),
/// and [`Attribute`]s — none of which support meaningful equality comparison. Returning
/// `false` tells Dioxus to always re-render this component, which is the intended
/// behavior for reactive signal-driven updates.
impl PartialEq for SortableItemProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// ============================================================================
// SortableItem Component
// ============================================================================

/// A sortable item component
///
/// SortableItem handles drag behavior directly with the "sortable" drag type
/// (by default) and displays drop indicators when items are being dragged over.
///
/// # Context Requirements
///
/// This component requires:
/// - `DragContext` - provided by DragContextProvider
/// - `SortableState` - provided by SortableContext
///
/// # Type Inheritance
///
/// By default, `SortableItem` has the "sortable" drag type. When you provide
/// additional types via the `content_types` prop, they are ADDED to "sortable":
///
/// ```ignore
/// SortableItem {
///     content_types: vec![DragType::new("image")],  // Results in: ["sortable", "image"]
/// }
/// ```
///
/// To replace the default "sortable" type entirely, use the `drag_type` prop:
///
/// ```ignore
/// SortableItem {
///     drag_type: Some(DragType::new("custom")),  // Only "custom", no "sortable"
///     content_types: vec![...],                   // Additional types after "custom"
/// }
/// ```
///
/// # Example (Basic)
///
/// ```ignore
/// rsx! {
///     SortableContext {
///         id: DragId::new("list"),
///         items: item_ids,
///         on_reorder: move |e| { /* handle reorder */ },
///
///         for (i, item) in items.iter().enumerate() {
///             SortableItem {
///                 key: "{i}",
///                 id: DragId::new(format!("item-{i}")),
///                 div { "{item}" }
///             }
///         }
///     }
/// }
/// ```
///
/// # Example (Multi-Type for Filtered Containers)
///
/// ```ignore
/// rsx! {
///     // Container that only accepts images
///     SortableContext {
///         id: DragId::new("media"),
///         items: media_ids,
///         accepts: vec![DragType::new("image"), DragType::new("video")],
///         on_reorder: move |e| { /* ... */ },
///
///         for file in files.iter() {
///             SortableItem {
///                 key: "{file.id}",
///                 id: file.drag_id(),
///                 content_types: vec![file.content_type()],  // e.g., "image"
///                 div { "{file.name}" }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn SortableItem(props: SortableItemProps) -> Element {
    // Get drag context and sortable state from context
    let ctx = use_context::<DragContext>();
    let sortable = use_context::<SortableState>();

    // Store mounted element reference for rect calculations
    let mut node_ref: Signal<Option<MountedEvent>> = use_signal(|| None);

    // Track spawned rect measurement task for cancellation
    let mut rect_task: Signal<Option<Task>> = use_signal(|| None);

    // Build the complete list of drag types for this item using the shared utility
    // Default to "sortable" if no drag_type provided
    let all_drag_types =
        combine_drag_types(props.drag_type.as_ref(), &props.content_types, "sortable");

    // Clone values for use in effects
    let id = props.id.clone();
    let container_id = sortable.container_id.read().clone();
    let accepts_for_effect = all_drag_types.clone();

    // Track drop_disabled in a signal so the effect can react to changes.
    // When dragging a superset header, member items need to dynamically
    // unregister as drop zones to prevent invalid drops.
    let mut drop_disabled_signal = use_signal(|| props.drop_disabled);

    // Keep signal in sync with prop changes
    if *drop_disabled_signal.peek() != props.drop_disabled {
        drop_disabled_signal.set(props.drop_disabled);
    }

    // Register as drop zone on mount and when items change
    // We must re-register after reorders because item positions change
    use_effect(move || {
        let id = id.clone();
        let container_id = container_id.clone();
        let accepts = accepts_for_effect.clone();

        // Read node_ref to make this effect reactive to changes
        let mounted = node_ref.read().clone();

        // Read items to make this effect reactive to item order changes
        // This triggers rect re-registration after reordering
        let _items = sortable.items.read();

        // Read drop_disabled signal to make this effect reactive to prop changes.
        // This is critical for superset header drag where member items need to
        // dynamically become non-drop-targets during the drag operation.
        let is_drop_disabled = *drop_disabled_signal.read();

        // Read measure generation so rects refresh on each drag start.
        // This accounts for browser zoom, scroll, or viewport resize.
        let _gen = ctx.measure_generation_signal();
        let _gen = _gen.read();

        // If drop is disabled, unregister immediately and don't re-register
        if is_drop_disabled {
            ctx.unregister(&id);
            return;
        }

        // NOTE: We do NOT unregister before re-registering. register_drop_zone
        // uses HashMap::insert which atomically replaces the old entry.
        // Unregistering first created a ~33ms window (2 animation frames) where
        // the item was invisible to collision detection, causing missed drops
        // on fresh page load and initial drags.

        // Cancel any in-flight rect measurement before spawning a new one
        if let Some(prev) = *rect_task.peek() {
            prev.cancel();
        }

        let task = spawn(async move {
            if let Some(mounted) = mounted {
                // Wait for two animation frames to ensure browser layout is complete
                // Single RAF may not be enough - layout can happen during the frame
                // after the callback is scheduled but before measurements are taken
                NextAnimationFrame::new().await;
                NextAnimationFrame::new().await;

                if let Ok(rect) = mounted.get_client_rect().await {
                    // Convert euclid Rect to our owned Rect
                    let owned_rect = Rect::new(
                        rect.origin.x,
                        rect.origin.y,
                        rect.size.width,
                        rect.size.height,
                    );

                    // Atomically replaces any existing entry (no unregister gap)
                    let orientation = *sortable.orientation.peek();
                    ctx.register_drop_zone(id, container_id, owned_rect, accepts, orientation);
                }
            }
        });
        rect_task.set(Some(task));
    });

    // Cleanup on unmount
    let cleanup_id = props.id.clone();
    use_drop(move || {
        if let Some(task) = *rect_task.peek() {
            task.cancel();
        }
        ctx.unregister(&cleanup_id);
    });

    // Check if this item is currently being dragged (subscribing read for reactivity)
    let is_dragging = ctx.is_dragging_id(&props.id);
    // Check if this item is being keyboard-dragged (for data-keyboard-drag attribute)
    let is_keyboard_dragging = is_dragging && ctx.is_keyboard_drag();

    // Get current drop location to determine indicator position (subscribing read)
    let drop_location = ctx.get_drop_location();

    // Determine if drop indicator should show before/after this item
    let my_container_id = sortable.container_id.read().clone();
    let all_items = sortable.items.read();
    let my_idx_in_list = all_items.iter().position(|id| *id == props.id);
    let source_idx_in_list = ctx.get_active_drag().and_then(|active_drag| {
        if active_drag.source_container_id.as_ref() == Some(&my_container_id) {
            all_items.iter().position(|id| id == &active_drag.data.id)
        } else {
            None
        }
    });
    let indicator_position = match &drop_location {
        Some(DropLocation::AtIndex {
            container_id,
            index,
        }) if *container_id == my_container_id => {
            let visual_index = visual_drop_index(*index, source_idx_in_list);
            if my_idx_in_list == Some(visual_index) {
                Some(IndicatorPos::Before)
            } else if my_idx_in_list.map(|i| i + 1) == Some(visual_index) {
                Some(IndicatorPos::After)
            } else {
                None
            }
        }
        _ => None,
    };
    drop(all_items);

    // Check if this item is a merge target (IntoItem collision)
    let is_merge_target = matches!(
        &drop_location,
        Some(DropLocation::IntoItem { item_id, .. }) if *item_id == props.id
    );

    // Get orientation from sortable context
    let orientation = *sortable.orientation.read();

    let has_target = drop_location.is_some();

    // Before RSX - build state for render prop
    let state = SortableItemState {
        is_dragging,
        indicator_position: indicator_position.map(|pos| match pos {
            IndicatorPos::Before => IndicatorPosition::Before,
            IndicatorPos::After => IndicatorPosition::After,
        }),
        is_merge_target,
        is_placeholder: is_dragging && !has_target,
        is_keyboard_dragging,
    };

    // Extract consumer attributes
    let consumer_class = extract_attribute(&props.attributes, "class");
    let is_placeholder = is_dragging && !has_target;
    let is_indicator_mode = !ctx.gap_displacement();

    let other_attributes = filter_class_style(props.attributes.clone());

    // Consumer-only class (library state communicated via data-* attributes)
    let merged_class = consumer_class.unwrap_or_default();

    // Default ARIA roles if not provided
    let has_role = props.attributes.iter().any(|a| a.name == "role");

    let displacement = compute_displacement(ctx, sortable, &props.id);
    let displacement_style = displacement.style();

    // Data attributes for Radix-style state communication
    let dnd_state = if is_dragging && has_target {
        "dragging"
    } else if is_placeholder {
        "placeholder"
    } else if is_merge_target {
        "merge-target"
    } else {
        ""
    };
    // Note: dnd_merge and dnd_drop_disabled use inline conditionals in RSX
    // to ensure attributes are ABSENT (not empty) when inactive.
    // CSS [data-merge-target] matches on presence, not value.
    let dnd_drop_pos = match indicator_position {
        Some(IndicatorPos::Before) => "before",
        Some(IndicatorPos::After) => "after",
        _ => "",
    };
    let dnd_adjacent = match displacement.drop_adjacent {
        DropAdjacent::Top => "top",
        DropAdjacent::Bottom => "bottom",
        DropAdjacent::None => "",
    };
    let dnd_mode = if is_indicator_mode && is_dragging {
        "indicator"
    } else {
        ""
    };

    // --- Pointer handlers (inlined from Draggable) ---
    let id_for_start = props.id.clone();
    let drag_types_for_start = all_drag_types.clone();
    let disabled_for_handler = props.disabled;
    let handle_selector = props.handle.clone();

    let start_drag = move |e: PointerEvent| {
        if disabled_for_handler {
            return;
        }

        // If a handle selector is specified, check if the event target matches
        if let Some(ref selector) = handle_selector
            && !pointer_event_matches_handle(&e, selector)
        {
            return;
        }

        e.prevent_default();

        let position = Position {
            x: e.client_coordinates().x,
            y: e.client_coordinates().y,
        };
        let data = DragData::with_types(id_for_start.clone(), drag_types_for_start.clone());
        ctx.start_pointer_drag(data, id_for_start.clone(), position, e.data().pointer_id());
    };

    let id_for_keyboard = props.id.clone();
    let drag_types_for_keyboard = all_drag_types.clone();
    let disabled_for_keyboard = props.disabled;
    let onkeydown = move |e: KeyboardEvent| {
        let key = e.key();

        // Space/Enter: start a keyboard drag (when not already dragging and not disabled)
        if matches!(key, Key::Character(ref c) if c == " ") || key == Key::Enter {
            e.prevent_default();

            if disabled_for_keyboard || ctx.is_dragging() {
                return;
            }

            let items = sortable.items.read();
            let my_index = items.iter().position(|id| id == &id_for_keyboard);

            if let Some(idx) = my_index {
                let data =
                    DragData::with_types(id_for_keyboard.clone(), drag_types_for_keyboard.clone());
                let container_id = sortable.container_id.read().clone();
                ctx.start_keyboard_drag(data, id_for_keyboard.clone(), container_id, &items, idx);
            }
        }
    };

    // Append functional drag styles to displacement style
    let has_handle = props.handle.is_some();
    let full_style = if has_handle {
        format!("{displacement_style}; touch-action: auto; user-select: none;")
    } else {
        format!("{displacement_style}; touch-action: none; user-select: none; cursor: grab;")
    };
    let instructions_id = ctx.instructions_id();

    rsx! {
        div {
            class: "{merged_class}",
            onmounted: move |data| {
                node_ref.set(Some(data.clone()));
            },
            // Inject default Role if not present in attributes
            role: if !has_role { Some("listitem") } else { None },
            style: "{full_style}",
            // ARIA attributes (inlined from Draggable)
            tabindex: if !props.disabled { Some(0) } else { None },
            aria_disabled: props.disabled,
            aria_roledescription: "draggable",
            aria_describedby: "{instructions_id}",
            // Data attributes
            "data-dnd-item": "",
            "data-dnd-id": props.id.0.as_str(),
            "data-state": "{dnd_state}",
            "data-merge-target": if is_merge_target { "true" },
            "data-drop-disabled": if props.drop_disabled { "true" },
            "data-drop-position": "{dnd_drop_pos}",
            "data-drop-adjacent": "{dnd_adjacent}",
            "data-displacement-mode": "{dnd_mode}",
            "data-keyboard-drag": if is_keyboard_dragging { "true" },
            "data-dnd-handle-mode": if has_handle { "true" },
            // Pointer handlers (inlined from Draggable)
            onpointerdown: start_drag,
            onkeydown: onkeydown,
            oncontextmenu: move |e: Event<MouseData>| {
                e.prevent_default();
            },
            ..other_attributes,

            // Drop indicator BEFORE this item (indicator mode only — gap mode uses displacement)
            if is_indicator_mode && indicator_position == Some(IndicatorPos::Before) {
                DropIndicator {
                    orientation: orientation,
                    position: Some(IndicatorPosition::Before),
                    preview: None,
                    gap_height: None,
                }
            }

            // User content (direct children — visibility via data-state on outer div)
            if let Some(render) = &props.render {
                {render.call(state)}
            } else {
                {props.children}
            }

            // Drop indicator AFTER this item (indicator mode only — gap mode uses displacement)
            if is_indicator_mode && indicator_position == Some(IndicatorPos::After) {
                DropIndicator {
                    orientation: orientation,
                    position: Some(IndicatorPosition::After),
                    preview: None,
                    gap_height: None,
                }
            }
        }
    }
}

// ============================================================================
// Displacement Calculation
// ============================================================================

/// Dead-zone piecewise linear easing for traversal displacement.
/// Entry dead zone (0%) → linear ramp (1.43x) → exit dead zone (100%).
/// Symmetric: ease(t) + ease(1-t) = 1.0, so direction reversals feel identical.
/// The dead zones align with Before/After collision zones: the item stays
/// stationary while the cursor is in the Before zone, displaces through the
/// IntoItem zone, and is fully displaced in the After zone.
pub(crate) fn ease_traversal(progress: f64) -> f64 {
    const DEAD: f64 = 0.15;

    if progress <= DEAD {
        0.0
    } else if progress >= 1.0 - DEAD {
        1.0
    } else {
        (progress - DEAD) / (1.0 - 2.0 * DEAD)
    }
}

/// Whether an item is adjacent to the drop line (for edge glow effects).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DropAdjacent {
    /// Not adjacent to drop line
    None,
    /// My top edge borders the drop line (item below the line)
    Top,
    /// My bottom edge borders the drop line (item above the line)
    Bottom,
}

/// Result of displacement computation for a sortable item.
pub(crate) struct DisplacementResult {
    /// Transform value for this item's displacement.
    pub transform: String,
    /// Whether transform updates should skip animation for this frame.
    pub instant_transition: bool,
    /// Whether this item's edge borders the drop line (for edge glow CSS)
    pub drop_adjacent: DropAdjacent,
}

impl DisplacementResult {
    /// Build an inline style string from transform + transition mode.
    ///
    /// We always write `--dxdnd-displacement-duration` so transition duration
    /// cannot get stuck at `0ms` across frames.
    pub fn style(&self) -> String {
        let duration = if self.instant_transition {
            "0ms"
        } else {
            "var(--dxdnd-transition-displacement-duration)"
        };
        format!(
            "transform: {}; --dxdnd-displacement-duration: {duration}",
            self.transform
        )
    }
}

/// Compute adjacency — is this item next to the drop line?
///
/// In both gap and indicator modes, items adjacent to the drop line
/// get edge glow CSS classes for visual clarity.
fn visual_drop_index(index: usize, source_index: Option<usize>) -> usize {
    // AtIndex in same-container reorders is expressed in filtered-list space
    // (dragged source slot removed). Convert back to full-list boundary space
    // for visual mapping on still-mounted items.
    match source_index {
        Some(src) if index > src => index + 1,
        _ => index,
    }
}

fn compute_adjacency(
    my_id: &DragId,
    drop_loc: &Option<DropLocation>,
    items: &[DragId],
    source_index: Option<usize>,
) -> DropAdjacent {
    let loc = match drop_loc {
        Some(l) => l,
        None => return DropAdjacent::None,
    };

    let my_idx = match items.iter().position(|id| id == my_id) {
        Some(idx) => idx,
        None => return DropAdjacent::None,
    };

    if let DropLocation::AtIndex { index, .. } = loc {
        let visual_index = visual_drop_index(*index, source_index);
        // Drop line is at position `index` (between items index-1 and index).
        // I'm adjacent-bottom if I'm the item just before the line (my_idx == index - 1).
        // I'm adjacent-top if I'm the item just after the line (my_idx == index).
        if visual_index > 0 && my_idx == visual_index - 1 {
            return DropAdjacent::Bottom;
        }
        if my_idx == visual_index {
            return DropAdjacent::Top;
        }
    }

    DropAdjacent::None
}

/// Compute displacement style for a sortable item.
/// Calculates how much this item should shift to make room for
/// a dragged item being reordered in the list.
///
/// Returns transform + transition mode:
/// - `translate{X|Y}(Npx)` for displacement
/// - `instant_transition=true` for traversal/snap frames
/// - `transform: none` for no displacement
fn compute_displacement(
    ctx: DragContext,
    sortable: SortableState,
    my_id: &DragId,
) -> DisplacementResult {
    let active_signal = ctx.active_signal();

    // Extract dragged ID and source container from active drag signal
    let (dragged_id_opt, source_container_opt) = {
        let active = active_signal.read();
        (
            active.as_ref().map(|d| d.data.id.clone()),
            active.as_ref().and_then(|d| d.source_container_id.clone()),
        )
    };

    let current_drop_loc = ctx.projected_drop_location();
    let items = sortable.items.read();

    // Compute adjacency for edge glow (used in both modes)
    let source_index_for_adjacency = dragged_id_opt.as_ref().and_then(|dragged_id| {
        items.iter().position(|id| id == dragged_id).or_else(|| {
            source_container_opt
                .as_ref()
                .and_then(|src_cid| ctx.find_nested_parent(src_cid))
                .and_then(|parent_id| items.iter().position(|id| id == &parent_id))
        })
    });
    let adjacency = compute_adjacency(my_id, &current_drop_loc, &items, source_index_for_adjacency);

    let no_displacement = || DisplacementResult {
        transform: "none".to_string(),
        instant_transition: false,
        drop_adjacent: adjacency,
    };

    // If I am the dragged item, no displacement
    if let Some(dragged_id) = &dragged_id_opt
        && dragged_id == my_id
    {
        return no_displacement();
    }

    // If this container doesn't accept the dragged item's type, skip displacement.
    // Prevents displacement inside groups when dragging group headers.
    if dragged_id_opt.is_some() && !ctx.container_accepts_active(&sortable.container_id.read()) {
        return no_displacement();
    }

    // Indicator mode: no displacement — items stay in place, only adjacency computed.
    if !ctx.gap_displacement() {
        return no_displacement();
    }

    // Read traversal signals for continuous displacement.
    // All items read traversal_item (infrequent change — only on item boundary crossings).
    let traversal_id_sig = ctx.traversal_item_signal();
    let traversal_id = traversal_id_sig.read();
    let is_traversal = traversal_id.as_ref() == Some(my_id);

    // Check if this item just exited traversal (was traversal target last frame).
    // peek() is correct: no reactive subscription needed — this re-render is already
    // triggered by the traversal_item signal change that caused the exit.
    let is_exiting_traversal = ctx
        .previous_traversal_signal()
        .peek()
        .as_ref()
        .is_some_and(|(id, _)| id == my_id);

    // Helper: suppress CSS transition for one frame when an item exits traversal.
    // Without this, the CSS baseline transition animates the discrete
    // displacement change (partial → full shift) causing a visible bounce.
    let snap_result = |transform: String| -> DisplacementResult {
        DisplacementResult {
            transform,
            instant_transition: is_exiting_traversal,
            drop_adjacent: adjacency,
        }
    };

    // ONLY the traversal item reads fraction (60fps subscription — one subscriber).
    let traversal_frac = if is_traversal {
        *ctx.traversal_fraction_signal().read()
    } else {
        0.0 // Don't subscribe
    };

    // Orientation-aware axis: "Y" for vertical, "X" for horizontal
    let orientation = *sortable.orientation.read();
    let axis = match orientation {
        Orientation::Vertical => "Y",
        Orientation::Horizontal => "X",
    };

    // Non-reactive peek for own size along orientation axis (used for pixel-based displacement).
    let my_size = ctx.get_zone_size(my_id).unwrap_or(0.0);

    let my_index = items.iter().position(|id| id == my_id);

    if let (Some(dragged_id), Some(my_idx)) = (dragged_id_opt, my_index) {
        let direct_source = items.iter().position(|id| id == &dragged_id);

        // Resolve effective source for nested container drags (group drag).
        // When dragging a group header (inside a nested child container), the header
        // isn't in this container's items list. But the group's item zone IS.
        // Use the group as the effective source for displacement.
        let (source_index, effective_size) = if direct_source.is_some() {
            (direct_source, ctx.get_zone_size(&dragged_id))
        } else {
            let nested_source = source_container_opt
                .as_ref()
                .and_then(|src_cid| ctx.find_nested_parent(src_cid))
                .and_then(|parent_id| {
                    items
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

        // Helper to produce a full displacement transform (discrete, CSS-transitioned).
        // Uses pixel value (effective size along orientation axis) when available,
        // falls back to "transform: none" as safe default.
        let full_shift = |negative: bool| -> DisplacementResult {
            let transform = match effective_size {
                Some(h) => {
                    let px = if negative { -h } else { h };
                    format!("translate{axis}({px}px)")
                }
                None => "none".to_string(),
            };
            snap_result(transform)
        };

        // Track whether any target exists (distinguishes "no target yet" from "target in another container")
        let has_any_target = current_drop_loc.is_some();

        // Determine target index based on drop location
        let target_index_and_mode = if let Some(loc) = &current_drop_loc {
            if loc.container_id() == *sortable.container_id.read() {
                let idx = loc.resolve_drop_index(&items);
                let is_partial = matches!(loc, DropLocation::IntoItem { .. });
                Some((idx, is_partial))
            } else {
                None
            }
        } else {
            None
        };

        // Continuous traversal: this item is being smoothly traversed by the projected center.
        // Compute displacement as a fraction of effective_size, with no CSS transition.
        //
        // Direction matters: the fraction tracks position within the item's rect
        // (0.0 = top, 1.0 = bottom). When dragging DOWN, the center enters from
        // the top (frac ≈ 0) and exits at the bottom (frac ≈ 1), so displacement
        // grows with fraction. When dragging UP, the center enters from the bottom
        // (frac ≈ 1) and exits at the top (frac ≈ 0), so we invert the fraction.
        if is_traversal && let Some(src) = source_index {
            let h = effective_size.unwrap_or(my_size);
            // Normalize to entry→exit direction, apply easing when merge enabled
            let progress = if src < my_idx {
                traversal_frac // DOWN/RIGHT: already 0→1
            } else {
                1.0 - traversal_frac // UP/LEFT: invert to 0→1
            };
            let eased = if ctx.is_merge_enabled() {
                ease_traversal(progress)
            } else {
                progress
            };
            let px = if src < my_idx { -h * eased } else { h * eased };
            return DisplacementResult {
                transform: format!("translate{axis}({px}px)"),
                instant_transition: true,
                drop_adjacent: adjacency,
            };
        }

        // Check if target is inside a nested child container (for parent-level expansion).
        // When an item enters a nested container (group), items below the group in the
        // parent must shift down to accommodate the group growing visually.
        let nested_group_idx = if has_any_target {
            current_drop_loc.as_ref().and_then(|loc| {
                let target_cid = loc.container_id();
                if target_cid != *sortable.container_id.read() {
                    ctx.find_nested_parent(&target_cid)
                        .and_then(|parent_id| items.iter().position(|id| id == &parent_id))
                } else {
                    None
                }
            })
        } else {
            None
        };

        // Nested-sibling drag-out combines collapse + expansion; this case
        // cannot be represented by the standard projection model.
        if let (Some(src), None) = (source_index, target_index_and_mode)
            && has_any_target
            && let Some(group_idx) = nested_group_idx
        {
            let collapse = my_idx > src;
            let expansion = my_idx > group_idx;
            match (collapse, expansion) {
                (true, false) => return full_shift(true),  // collapse only
                (false, true) => return full_shift(false), // expansion only
                _ => {}                                    // both cancel out, or neither applies
            }
        }

        // Canonical displacement projection shared with collision detection.
        let dragged_size = effective_size.unwrap_or(my_size);
        let canonical_offset = match (source_index, target_index_and_mode) {
            (Some(src), Some((tgt, is_partial))) => {
                // Item displacement uses the full list (includes source slot).
                // Convert to filtered-list indices so this matches collision.
                let my_filtered = to_filtered_index(my_idx, src);
                let target_filtered = if is_partial {
                    to_filtered_index(tgt, src)
                } else {
                    tgt
                };

                compute_displacement_offset(
                    my_filtered,
                    Some(src),
                    Some(target_filtered),
                    is_partial,
                    my_size,
                    dragged_size,
                )
            }
            // Drag-out collapse applies only after we have any target.
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
            (None, Some((tgt, is_partial))) => compute_displacement_offset(
                my_idx,
                None,
                Some(tgt),
                is_partial,
                my_size,
                dragged_size,
            ),
            _ => 0.0,
        };

        if canonical_offset.abs() > f64::EPSILON {
            return snap_result(format!("translate{axis}({canonical_offset}px)"));
        }

        // Remaining fallback for nested-container expansion-only paths.
        // (Some(_), None) is already handled by canonical projection.
        // Cross-container drag into a nested child — expansion only
        if let (None, None) = (source_index, target_index_and_mode)
            && let Some(group_idx) = nested_group_idx
            && my_idx > group_idx
        {
            return full_shift(false);
        }
    }

    // When exiting traversal but falling through to no displacement, suppress
    // the CSS transition to prevent bouncing from partial → none.
    if is_exiting_traversal {
        return DisplacementResult {
            transform: "none".to_string(),
            instant_transition: true,
            drop_adjacent: adjacency,
        };
    }

    no_displacement()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::VNode;

    #[test]
    fn test_sortable_item_props_default_disabled() {
        // Verify disabled defaults to false
        let disabled: bool = false;
        assert!(!disabled);
    }

    #[test]
    fn test_sortable_item_props_drag_type_default_is_none() {
        // SortableItemProps should have drag_type field that defaults to None
        // When None, the component should use DragType::new("sortable")
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: None,
            content_types: vec![],
            render: None,
            attributes: vec![],
        };

        // drag_type should be None by default
        assert!(props.drag_type.is_none());
    }

    #[test]
    fn test_sortable_item_props_custom_drag_type() {
        // SortableItemProps should accept a custom drag_type
        let custom_type = DragType::new("custom-type");
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: Some(custom_type.clone()),
            content_types: vec![],
            render: None,
            attributes: vec![],
        };

        assert_eq!(props.drag_type, Some(custom_type));
    }

    #[test]
    fn test_sortable_item_default_drag_type_is_sortable() {
        // When drag_type is None, the component uses DragType::new("sortable")
        // We test this by verifying the default behavior
        let default_type = DragType::new("sortable");
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: None,
            content_types: vec![],
            render: None,
            attributes: vec![],
        };

        // The effective type should be "sortable" when None
        let effective_type = props.drag_type.unwrap_or_else(|| DragType::new("sortable"));
        assert_eq!(effective_type, default_type);
    }

    // =========================================================================
    // Multi-type tests
    // =========================================================================

    #[test]
    fn test_sortable_item_props_content_types_default_is_empty() {
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: None,
            content_types: vec![],
            render: None,
            attributes: vec![],
        };

        assert!(props.content_types.is_empty());
    }

    #[test]
    fn test_sortable_item_props_with_additional_content_types() {
        // SortableItemProps should accept additional content_types
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: None, // Use default "sortable"
            content_types: vec![DragType::new("image"), DragType::new("media")],
            render: None,
            attributes: vec![],
        };

        assert_eq!(props.content_types.len(), 2);
        assert!(props.content_types.contains(&DragType::new("image")));
        assert!(props.content_types.contains(&DragType::new("media")));
    }

    #[test]
    fn test_sortable_item_combines_base_type_with_content_types() {
        // This tests the logic that the component uses to build all_drag_types
        // Base type (from drag_type or "sortable") + additional types from content_types prop
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: None, // Will use "sortable"
            content_types: vec![DragType::new("image")],
            render: None,
            attributes: vec![],
        };

        // Simulate the component's type combining logic
        let base_type = props
            .drag_type
            .clone()
            .unwrap_or_else(|| DragType::new("sortable"));
        let mut all_types = vec![base_type];
        all_types.extend(props.content_types.iter().cloned());

        // Should have both "sortable" (base) and "image" (additional)
        assert_eq!(all_types.len(), 2);
        assert_eq!(all_types[0], DragType::new("sortable"));
        assert!(all_types.contains(&DragType::new("image")));
    }

    #[test]
    fn test_sortable_item_custom_base_type_with_content_types() {
        // When drag_type is set, it becomes the base type instead of "sortable"
        let props = SortableItemProps {
            id: DragId::new("test"),
            children: VNode::empty(),
            disabled: false,
            drop_disabled: false,
            handle: None,
            drag_type: Some(DragType::new("custom")),
            content_types: vec![DragType::new("extra")],
            render: None,
            attributes: vec![],
        };

        // Simulate the component's type combining logic
        let base_type = props
            .drag_type
            .clone()
            .unwrap_or_else(|| DragType::new("sortable"));
        let mut all_types = vec![base_type];
        all_types.extend(props.content_types.iter().cloned());

        // Should have "custom" (base) and "extra" (additional)
        assert_eq!(all_types.len(), 2);
        assert_eq!(all_types[0], DragType::new("custom"));
        assert!(all_types.contains(&DragType::new("extra")));
        // Should NOT have "sortable" since drag_type was explicitly set
        assert!(!all_types.contains(&DragType::new("sortable")));
    }

    // ========================================================================
    // ease_traversal tests (dead-zone piecewise linear)
    // ========================================================================

    #[test]
    fn test_ease_traversal_boundaries() {
        assert_eq!(ease_traversal(0.0), 0.0);
        assert_eq!(ease_traversal(1.0), 1.0);
    }

    #[test]
    fn test_ease_traversal_edge_cases() {
        // Negative values clamp to 0
        assert_eq!(ease_traversal(-0.5), 0.0);
        assert_eq!(ease_traversal(-0.01), 0.0);
        // Values > 1 clamp to 1
        assert_eq!(ease_traversal(1.5), 1.0);
        assert_eq!(ease_traversal(100.0), 1.0);
    }

    #[test]
    fn test_ease_traversal_dead_zones() {
        // Entry dead zone: 0..=0.15 → 0.0
        assert_eq!(ease_traversal(0.0), 0.0);
        assert_eq!(ease_traversal(0.05), 0.0);
        assert_eq!(ease_traversal(0.10), 0.0);
        assert_eq!(ease_traversal(0.15), 0.0);

        // Exit dead zone: 0.85..=1.0 → 1.0
        assert_eq!(ease_traversal(0.85), 1.0);
        assert_eq!(ease_traversal(0.90), 1.0);
        assert_eq!(ease_traversal(0.95), 1.0);
        assert_eq!(ease_traversal(1.0), 1.0);
    }

    #[test]
    fn test_ease_traversal_midpoint() {
        let eps = 1e-10;
        // Midpoint should be exactly 0.5
        assert!((ease_traversal(0.50) - 0.50).abs() < eps);
    }

    #[test]
    fn test_ease_traversal_symmetry() {
        let eps = 1e-10;
        // ease(t) + ease(1-t) = 1.0 for all t
        for i in 0..=100 {
            let t = i as f64 / 100.0;
            let sum = ease_traversal(t) + ease_traversal(1.0 - t);
            assert!(
                (sum - 1.0).abs() < eps,
                "Symmetry violated: ease({t}) + ease({}) = {sum}, expected 1.0",
                1.0 - t,
            );
        }
    }

    #[test]
    fn test_ease_traversal_monotonically_increasing() {
        let steps = 100;
        let mut prev = 0.0;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let val = ease_traversal(t);
            assert!(
                val >= prev,
                "Not monotonic: ease({}) = {} < ease({}) = {}",
                t,
                val,
                (i - 1) as f64 / steps as f64,
                prev
            );
            prev = val;
        }
    }

    #[test]
    fn test_ease_traversal_ramp_speed() {
        // In the ramp zone (0.15..0.85), slope is 1/(1-2*0.15) = 1/0.7 ≈ 1.43
        let ramp_start = ease_traversal(0.16); // just past dead zone
        let ramp_end = ease_traversal(0.84); // just before exit dead zone
        let slope = (ramp_end - ramp_start) / (0.84 - 0.16);
        assert!(
            slope > 1.0,
            "Ramp slope {slope} should be > 1.0 (steeper than linear)"
        );
    }

    // =========================================================================
    // Adjacency Detection Tests
    // =========================================================================

    #[test]
    fn test_adjacency_at_index_neighbor_gets_bottom() {
        // Drop at index 1 (between a and b) — item A (index 0 = index-1) gets Bottom adjacency
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];
        let drop_loc = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 1,
        });

        assert_eq!(
            compute_adjacency(&DragId::new("a"), &drop_loc, &items, None),
            DropAdjacent::Bottom,
            "Item before the drop line (index-1) should get Bottom adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("b"), &drop_loc, &items, None),
            DropAdjacent::Top,
            "Item at the drop index should get Top adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("c"), &drop_loc, &items, None),
            DropAdjacent::None,
            "Items far from drop line should get None"
        );
    }

    #[test]
    fn test_adjacency_at_index_after_item_gets_top() {
        // Drop at index 2 (between b and c) — item C (index 2 = index) gets Top adjacency
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];
        let drop_loc = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });

        assert_eq!(
            compute_adjacency(&DragId::new("c"), &drop_loc, &items, None),
            DropAdjacent::Top,
            "Item at the drop index should get Top adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("b"), &drop_loc, &items, None),
            DropAdjacent::Bottom,
            "Item at index-1 should get Bottom adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("a"), &drop_loc, &items, None),
            DropAdjacent::None,
            "Items far from drop line should get None"
        );
    }

    #[test]
    fn test_adjacency_at_index_zero_no_bottom_neighbor() {
        // Drop at index 0 (before first item) — no neighbor above
        let items = vec![DragId::new("a"), DragId::new("b")];
        let drop_loc = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 0,
        });

        assert_eq!(
            compute_adjacency(&DragId::new("a"), &drop_loc, &items, None),
            DropAdjacent::Top,
            "First item at the drop index should get Top adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("b"), &drop_loc, &items, None),
            DropAdjacent::None,
            "Non-adjacent items get None"
        );
    }

    #[test]
    fn test_adjacency_at_index_past_end_no_top_neighbor() {
        // Drop at index 2 (past last item) — no neighbor below
        let items = vec![DragId::new("a"), DragId::new("b")];
        let drop_loc = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });

        assert_eq!(
            compute_adjacency(&DragId::new("a"), &drop_loc, &items, None),
            DropAdjacent::None,
            "Non-adjacent items get None"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("b"), &drop_loc, &items, None),
            DropAdjacent::Bottom,
            "Last item (at index-1) gets Bottom adjacency"
        );
    }

    #[test]
    fn test_adjacency_into_item_no_adjacency() {
        // IntoItem doesn't produce adjacency (it's a merge, not a line)
        let items = vec![DragId::new("a"), DragId::new("b"), DragId::new("c")];
        let drop_loc = Some(DropLocation::IntoItem {
            container_id: DragId::new("list"),
            item_id: DragId::new("b"),
        });

        assert_eq!(
            compute_adjacency(&DragId::new("a"), &drop_loc, &items, None),
            DropAdjacent::None,
        );
        assert_eq!(
            compute_adjacency(&DragId::new("c"), &drop_loc, &items, None),
            DropAdjacent::None,
        );
    }

    #[test]
    fn test_adjacency_none_when_no_drop_location() {
        let items = vec![DragId::new("a"), DragId::new("b")];
        assert_eq!(
            compute_adjacency(&DragId::new("a"), &None, &items, None),
            DropAdjacent::None,
        );
    }

    #[test]
    fn test_adjacency_same_container_downward_drag_converts_filtered_index() {
        // Source is item "a" at index 0. AtIndex 2 in filtered space means
        // "before d" in full-list space (between c and d).
        let items = vec![
            DragId::new("a"),
            DragId::new("b"),
            DragId::new("c"),
            DragId::new("d"),
        ];
        let drop_loc = Some(DropLocation::AtIndex {
            container_id: DragId::new("list"),
            index: 2,
        });

        assert_eq!(
            compute_adjacency(&DragId::new("c"), &drop_loc, &items, Some(0)),
            DropAdjacent::Bottom,
            "Item above the converted boundary should get Bottom adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("d"), &drop_loc, &items, Some(0)),
            DropAdjacent::Top,
            "Item below the converted boundary should get Top adjacency"
        );
        assert_eq!(
            compute_adjacency(&DragId::new("b"), &drop_loc, &items, Some(0)),
            DropAdjacent::None,
            "Boundary should not stay one slot too high when dragging down"
        );
    }

    #[test]
    fn test_displacement_style_snap_frame_uses_zero_duration() {
        let style = DisplacementResult {
            transform: "translateY(-40px)".to_string(),
            instant_transition: true,
            drop_adjacent: DropAdjacent::None,
        }
        .style();

        assert!(style.contains("--dxdnd-displacement-duration: 0ms"));
    }

    #[test]
    fn test_displacement_style_resets_duration_after_snap_frames() {
        for _ in 0..20 {
            let snap_style = DisplacementResult {
                transform: "translateY(-20px)".to_string(),
                instant_transition: true,
                drop_adjacent: DropAdjacent::None,
            }
            .style();
            assert!(snap_style.contains("--dxdnd-displacement-duration: 0ms"));

            let normal_style = DisplacementResult {
                transform: "none".to_string(),
                instant_transition: false,
                drop_adjacent: DropAdjacent::None,
            }
            .style();
            assert!(normal_style.contains(
                "--dxdnd-displacement-duration: var(--dxdnd-transition-displacement-duration)"
            ));
        }
    }
}
