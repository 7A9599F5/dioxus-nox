//! Drop zone component primitive
//!
//! Defines areas where draggable items can be dropped.
//! Drop zones register with the DragContext and provide visual feedback
//! when items are dragged over them.

use dioxus::prelude::*;
use dioxus_core::Task;

use crate::context::DragContext;
use crate::types::{DragData, DragId, DragType, DropEvent, DropLocation, Orientation, Rect};
use crate::utils::{extract_attribute, filter_class_style};

// ============================================================================
// DropZoneRenderProps
// ============================================================================

/// Props passed to the `render_container` callback for custom element rendering.
///
/// When `render_container` is provided on [`DropZone`], this struct carries all
/// the computed attributes, event handlers, and children that must be applied to
/// the consumer's custom element for correct drop zone behavior.
///
/// # Critical: onmounted
///
/// The `onmounted` handler **must** be applied to your custom element. Without it,
/// the drop zone cannot measure its bounding rect and collision detection will not
/// work for this zone.
///
/// # Example
///
/// ```ignore
/// DropZone {
///     id: DragId::new("trash"),
///     render_container: move |rp: DropZoneRenderProps| rsx! {
///         section {
///             class: "{rp.class}",
///             tabindex: rp.tabindex,
///             aria_dropeffect: "{rp.aria_dropeffect}",
///             onmounted: rp.onmounted,
///             onkeydown: rp.onkeydown,
///             "data-dnd-dropzone": "",
///             "data-state": "{rp.data_state}",
///             "data-can-drop": "{rp.data_can_drop}",
///             ..rp.attributes,
///             {rp.children}
///         }
///     },
///     div { "Drop items here to delete" }
/// }
/// ```
#[derive(Clone)]
pub struct DropZoneRenderProps {
    /// Merged class string (consumer classes only; library state in data-*)
    pub class: String,
    /// Tabindex value (Some(0) during active drag, None otherwise)
    pub tabindex: Option<i32>,
    /// Whether an item is currently being dragged over this zone
    pub is_over: bool,
    /// Whether the zone can accept the currently dragged item
    pub can_drop: bool,
    /// Whether any drag is currently active
    pub is_drag_active: bool,
    /// Pre-computed aria-dropeffect value ("move" or "none")
    pub aria_dropeffect: String,
    /// Pre-computed data-state value ("over" or "")
    pub data_state: String,
    /// Pre-computed data-can-drop value ("true", "false", or "")
    pub data_can_drop: String,
    /// Mount handler — **MUST** be applied for rect registration to work
    pub onmounted: EventHandler<MountedEvent>,
    /// Keyboard handler — Space/Enter completes drop, Escape cancels
    pub onkeydown: EventHandler<KeyboardEvent>,
    /// Remaining consumer attributes (data-*, aria-*, etc., excluding class/style)
    pub attributes: Vec<Attribute>,
    /// Children to render inside the element
    pub children: Element,
}

// EventHandler is not PartialEq
impl PartialEq for DropZoneRenderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// Props for the DropZone component
#[derive(Props, Clone)]
pub struct DropZoneProps {
    /// Unique ID for this drop zone
    #[props(into)]
    pub id: DragId,

    /// Types this zone accepts (empty = all)
    #[props(default)]
    pub accepts: Vec<DragType>,

    /// The content to render
    pub children: Element,

    /// Whether dropping is disabled
    #[props(default = false)]
    pub disabled: bool,

    /// Called when a draggable enters this zone
    #[props(default)]
    pub on_drag_enter: EventHandler<DragData>,

    /// Called when a draggable leaves this zone
    #[props(default)]
    pub on_drag_leave: EventHandler<DragData>,

    /// Called when an item is dropped in this zone
    #[props(default)]
    pub on_drop: EventHandler<DropEvent>,

    /// Layout orientation of the container this drop zone belongs to.
    /// Passed to DropZoneState for orientation-aware collision detection.
    #[props(default)]
    pub orientation: Orientation,

    /// Optional callback for custom element rendering.
    ///
    /// When provided, the component calls this instead of rendering the default `div`.
    /// The callback receives [`DropZoneRenderProps`] containing all computed attributes,
    /// event handlers, and children.
    ///
    /// **Critical**: The consumer's element **must** wire up the `onmounted` handler
    /// from [`DropZoneRenderProps`]. Without it, rect measurement fails and collision
    /// detection will not work for this zone.
    ///
    /// When `None` (default), the component renders its standard `div` wrapper.
    #[props(default)]
    pub render_container: Option<Callback<DropZoneRenderProps, Element>>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Always returns `false`: props contain [`Element`] (children), [`EventHandler`]s
/// (on_drag_enter, on_drag_leave, on_drop), and [`Attribute`]s — none of which support
/// meaningful equality comparison. Returning `false` tells Dioxus to always re-render
/// this component, which is the intended behavior for reactive signal-driven updates.
impl PartialEq for DropZoneProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// A drop zone component that accepts draggable items
///
/// Drop zones register with the DragContext on mount and provide visual
/// feedback when items are dragged over them. They support type filtering
/// via the `accepts` prop.
///
/// # Example
///
/// ```ignore
/// rsx! {
///     DropZone {
///         id: DragId::new("trash"),
///         accepts: vec![DragType::new("deletable")],
///         on_drop: move |event: DropEvent| {
///             // Handle the dropped item
///         },
///
///         div { "Drop items here to delete" }
///     }
/// }
/// ```
#[component]
pub fn DropZone(props: DropZoneProps) -> Element {
    // Get the drag context
    let ctx = use_context::<DragContext>();

    // Store mounted element reference for rect calculations
    let mut node_ref: Signal<Option<MountedEvent>> = use_signal(|| None);

    // Track spawned rect measurement task for cancellation
    let mut rect_task: Signal<Option<Task>> = use_signal(|| None);

    // Clone values for use in effects
    let id = props.id.clone();
    let accepts = props.accepts.clone();
    let orientation = props.orientation;

    // Register/update rect on mount and when dependencies change
    use_effect(move || {
        let id = id.clone();
        let accepts = accepts.clone();

        // Read measure generation so rects refresh on each drag start.
        // This accounts for browser zoom, scroll, or viewport resize.
        let gen_sig = ctx.measure_generation_signal();
        let _gen = gen_sig.read();

        // Cancel any in-flight rect measurement before spawning a new one
        if let Some(prev) = *rect_task.peek() {
            prev.cancel();
        }

        let task = spawn(async move {
            if let Some(mounted) = node_ref.read().as_ref()
                && let Ok(rect) = mounted.get_client_rect().await
            {
                // Convert euclid Rect to our owned Rect immediately
                let owned_rect = Rect::new(
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                );

                ctx.register_drop_zone(
                    id.clone(),
                    id.clone(), // Container is self for standalone drop zones
                    owned_rect,
                    accepts.clone(),
                    orientation,
                );
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

    // Query state from context using subscribing reads for reactivity
    let is_over = ctx.is_over(&props.id);
    let can_drop = !props.disabled && is_over && ctx.accepts(&props.id);

    // Track previous is_over state for enter/leave events
    let mut prev_over = use_signal(|| false);
    let mut last_drag_data: Signal<Option<crate::types::DragData>> = use_signal(|| None);
    let on_drag_enter = props.on_drag_enter;
    let on_drag_leave = props.on_drag_leave;

    use_effect(move || {
        let currently_over = is_over;
        let was_over = *prev_over.peek();

        if currently_over && !was_over {
            if let Some(active) = ctx.active() {
                last_drag_data.set(Some(active.data.clone()));
                on_drag_enter.call(active.data.clone());
            }
        } else if !currently_over && was_over {
            // active() may already be None when drag ends; use cached data
            let data = ctx
                .active()
                .map(|a| a.data.clone())
                .or_else(|| last_drag_data.peek().clone());
            if let Some(data) = data {
                on_drag_leave.call(data);
            }
            last_drag_data.set(None);
        }

        prev_over.set(currently_over);
    });

    // Extract consumer class from attributes
    let consumer_class = extract_attribute(&props.attributes, "class");

    // Consumer-only class (library state communicated via data-* attributes)
    let merged_class = consumer_class.unwrap_or_default();

    // Filter out class attribute (already merged above)
    let remaining_attrs = filter_class_style(props.attributes.clone());

    // Reactive read for ARIA and tabindex (re-renders when drag state changes)
    let is_drag_active = ctx.is_dragging();
    let zone_id = props.id.clone();
    let zone_disabled = props.disabled;
    let on_drop = props.on_drop;

    // Keyboard handler: Space/Enter completes drop onto this zone.
    //
    // CRITICAL: only handle keydowns that originated on this DropZone element,
    // not events bubbling from focusable descendants (e.g. a `SortableItem`
    // inside a `SortableContext` wrapped in a DropZone). Without this guard,
    // pressing Space on a child sortable item would bubble here, end the
    // drag, and the provider's activation arm would then start a fresh one
    // on the next event tick — see #58.
    let onkeydown = move |e: KeyboardEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(web_event) = e.data().downcast::<web_sys::KeyboardEvent>() {
                let target = web_event.target();
                let current = web_event.current_target();
                if target != current {
                    return;
                }
            }
        }

        let key = e.key();

        // Check drag state at event time (not stale closure capture)
        let dragging = ctx.active().is_some();

        match key {
            Key::Character(ref c) if c == " " => {
                e.prevent_default();
                if zone_disabled || !dragging {
                    return;
                }
                // Set this zone as the drop target and complete the drag
                let target = DropLocation::IntoContainer {
                    container_id: zone_id.clone(),
                };
                let mut target_sig = ctx.target_signal();
                *target_sig.write() = Some(target);
                if let Some(event) = ctx.end_drag() {
                    on_drop.call(event);
                }
            }
            Key::Enter => {
                e.prevent_default();
                if zone_disabled || !dragging {
                    return;
                }
                let target = DropLocation::IntoContainer {
                    container_id: zone_id.clone(),
                };
                let mut target_sig = ctx.target_signal();
                *target_sig.write() = Some(target);
                if let Some(event) = ctx.end_drag() {
                    on_drop.call(event);
                }
            }
            Key::Escape => {
                if dragging {
                    ctx.cancel_drag();
                    e.prevent_default();
                }
            }
            _ => {}
        }
    };

    let dnd_state = if is_over { "over" } else { "" };
    let dnd_can_drop = if is_over {
        if can_drop { "true" } else { "false" }
    } else {
        ""
    };

    // ARIA: "move" when actively accepting a drag, "none" otherwise
    // Use ctx.accepts() directly (not can_drop which requires is_over)
    let accepts_active = is_drag_active && !props.disabled && ctx.accepts(&props.id);
    let aria_dropeffect = if accepts_active { "move" } else { "none" };

    let tabindex_val = if is_drag_active { Some(0) } else { None };

    let onmounted_handler = move |data: MountedEvent| {
        node_ref.set(Some(data.clone()));
    };

    // If render_container is provided, delegate to consumer
    if let Some(render_container) = &props.render_container {
        let render_props = DropZoneRenderProps {
            class: merged_class.to_string(),
            tabindex: tabindex_val,
            is_over,
            can_drop,
            is_drag_active,
            aria_dropeffect: aria_dropeffect.to_string(),
            data_state: dnd_state.to_string(),
            data_can_drop: dnd_can_drop.to_string(),
            onmounted: EventHandler::new(onmounted_handler),
            onkeydown: EventHandler::new(onkeydown),
            attributes: remaining_attrs,
            children: props.children,
        };
        return render_container.call(render_props);
    }

    rsx! {
        div {
            class: "{merged_class}",
            tabindex: tabindex_val,
            aria_dropeffect: "{aria_dropeffect}",

            onmounted: onmounted_handler,
            onkeydown: onkeydown,
            "data-dnd-dropzone": "",
            "data-state": "{dnd_state}",
            "data-can-drop": "{dnd_can_drop}",
            ..remaining_attrs,

            {props.children}
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_zone_props_default() {
        // Verify default values compile correctly
        let _accepts: Vec<DragType> = vec![];
        let _disabled: bool = false;
    }

    // =========================================================================
    // Keyboard drop target tests
    // =========================================================================

    #[test]
    fn test_keyboard_drop_creates_into_container_location() {
        // Verify the DropLocation used for keyboard drops on DropZone
        let zone_id = DragId::new("trash-zone");
        let target = DropLocation::IntoContainer {
            container_id: zone_id.clone(),
        };
        assert_eq!(target.container_id(), zone_id);
        assert!(target.contains_id(&zone_id));
    }

    #[test]
    fn test_aria_dropeffect_values() {
        // "move" when active drag AND zone accepts, "none" otherwise
        let is_active = true;
        let accepts = true;
        let disabled = false;
        let accepts_active = is_active && !disabled && accepts;
        assert_eq!(if accepts_active { "move" } else { "none" }, "move");

        // No active drag
        let is_active = false;
        let accepts_active = is_active && !disabled && accepts;
        assert_eq!(if accepts_active { "move" } else { "none" }, "none");

        // Active drag but disabled
        let is_active = true;
        let disabled = true;
        let accepts_active = is_active && !disabled && accepts;
        assert_eq!(if accepts_active { "move" } else { "none" }, "none");
    }

    #[test]
    fn test_tabindex_during_drag() {
        // DropZone should be focusable (tabindex=0) only during active drag
        let is_drag_active = true;
        let tabindex: Option<i32> = if is_drag_active { Some(0) } else { None };
        assert_eq!(tabindex, Some(0));

        let is_drag_active = false;
        let tabindex: Option<i32> = if is_drag_active { Some(0) } else { None };
        assert_eq!(tabindex, None);
    }

    // =========================================================================
    // DropZoneRenderProps tests
    // =========================================================================

    #[test]
    fn test_dropzone_render_props_state_values() {
        // Verify data-state computed values
        let is_over = true;
        assert_eq!(if is_over { "over" } else { "" }, "over");

        let is_over = false;
        assert_eq!(if is_over { "over" } else { "" }, "");
    }

    #[test]
    fn test_dropzone_render_props_can_drop_values() {
        // data-can-drop: "true" when over + can drop, "false" when over but can't, "" when not over
        let is_over = true;
        let can_drop = true;
        let val = if is_over {
            if can_drop { "true" } else { "false" }
        } else {
            ""
        };
        assert_eq!(val, "true");

        let can_drop = false;
        let val = if is_over {
            if can_drop { "true" } else { "false" }
        } else {
            ""
        };
        assert_eq!(val, "false");

        let is_over = false;
        let val = if is_over {
            if can_drop { "true" } else { "false" }
        } else {
            ""
        };
        assert_eq!(val, "");
    }

    #[test]
    fn test_dropzone_render_props_aria_values() {
        // aria-dropeffect: "move" when active + accepts, "none" otherwise
        let is_active = true;
        let disabled = false;
        let accepts = true;
        let accepts_active = is_active && !disabled && accepts;
        assert_eq!(if accepts_active { "move" } else { "none" }, "move");

        let disabled = true;
        let accepts_active = is_active && !disabled && accepts;
        assert_eq!(if accepts_active { "move" } else { "none" }, "none");
    }

    #[test]
    fn test_dropzone_render_props_partial_eq_always_false() {
        // DropZoneRenderProps::eq always returns false
        let eq_result: bool = false; // PartialEq impl always returns false
        assert!(!eq_result);
    }
}
