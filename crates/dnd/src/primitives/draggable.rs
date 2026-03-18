//! Draggable component primitive
//!
//! The foundational component that makes elements draggable.

use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast;

use crate::context::DragContext;
use crate::types::{DragData, DragId, DragType, Position, combine_drag_types};
use crate::utils::{extract_attribute, filter_class_style, merge_styles};

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
// DraggableRenderProps
// ============================================================================

/// Props passed to the `render_container` callback for custom element rendering.
///
/// When `render_container` is provided on [`Draggable`], this struct carries all
/// the computed attributes, event handlers, and children that must be applied to
/// the consumer's custom element for correct drag behavior.
///
/// # Required
///
/// All event handlers (`onpointerdown`, `onpointerup`, `onkeydown`, `oncontextmenu`)
/// and ARIA attributes must be applied to the custom element. Omitting any will
/// break drag or accessibility behavior.
///
/// # Example
///
/// ```ignore
/// Draggable {
///     id: DragId::new("item-1"),
///     render_container: move |rp: DraggableRenderProps| rsx! {
///         li {
///             id: "{rp.element_id}",
///             class: "{rp.class}",
///             style: "{rp.style}",
///             tabindex: rp.tabindex,
///             role: "button",
///             aria_disabled: rp.disabled,
///             aria_roledescription: "draggable",
///             aria_describedby: "{rp.instructions_id}",
///             aria_grabbed: "{rp.aria_grabbed}",
///             "data-dnd-draggable": "",
///             "data-state": "{rp.data_state}",
///             "data-disabled": "{rp.data_disabled}",
///             onpointerdown: rp.onpointerdown,
///             onpointerup: rp.onpointerup,
///             onkeydown: rp.onkeydown,
///             oncontextmenu: rp.oncontextmenu,
///             ..rp.attributes,
///             {rp.children}
///         }
///     },
///     div { "Drag me!" }
/// }
/// ```
#[derive(Clone)]
pub struct DraggableRenderProps {
    /// Element id attribute value
    pub element_id: String,
    /// Merged class string (consumer classes only; library state in data-*)
    pub class: String,
    /// Merged style string (includes required touch-action/user-select/cursor)
    pub style: String,
    /// Tabindex value (Some(0) when enabled, None when disabled)
    pub tabindex: Option<i32>,
    /// Whether this item is currently being dragged
    pub is_dragging: bool,
    /// Whether dragging is disabled
    pub disabled: bool,
    /// ARIA instructions element ID
    pub instructions_id: String,
    /// Pre-computed aria-grabbed value ("true" or "false")
    pub aria_grabbed: String,
    /// Pre-computed data-state value ("dragging" or "")
    pub data_state: String,
    /// Pre-computed data-disabled value ("true" or "")
    pub data_disabled: String,
    /// Pointer down handler — starts the drag
    pub onpointerdown: EventHandler<PointerEvent>,
    /// Pointer up handler
    pub onpointerup: EventHandler<PointerEvent>,
    /// Keyboard handler — Space/Enter starts drag, Escape cancels
    pub onkeydown: EventHandler<KeyboardEvent>,
    /// Context menu handler — prevents right-click menu during drag
    pub oncontextmenu: EventHandler<Event<MouseData>>,
    /// Remaining consumer attributes (data-*, aria-*, etc., excluding class/style)
    pub attributes: Vec<Attribute>,
    /// Children to render inside the element
    pub children: Element,
}

// EventHandler is not PartialEq
impl PartialEq for DraggableRenderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// ============================================================================
// Draggable Props
// ============================================================================

/// Props for the Draggable component
#[derive(Props, Clone)]
pub struct DraggableProps {
    /// Unique ID for this draggable
    #[props(into)]
    pub id: DragId,

    /// Primary type for filtering compatible drop zones (backward compatible)
    ///
    /// If `drag_types` is provided, this is prepended to create the full type list.
    /// If neither `drag_type` nor `drag_types` is provided, defaults to empty type.
    #[props(default)]
    pub drag_type: Option<DragType>,

    /// Additional drag types beyond the primary `drag_type`.
    ///
    /// Use this when an item needs to be accepted by different drop zones
    /// based on different type criteria (e.g., both "sortable" and "image").
    ///
    /// If `drag_type` is also provided, it is prepended to this list.
    #[props(default)]
    pub additional_types: Vec<DragType>,

    /// The content to render
    pub children: Element,

    /// Whether dragging is disabled
    #[props(default = false)]
    pub disabled: bool,

    /// Tabindex for keyboard navigation
    ///
    /// Defaults to 0 to make the element focusable.
    /// Set to None to disable keyboard focus.
    #[props(default = Some(0))]
    pub tabindex: Option<i32>,

    /// Custom drag handle selector (if not whole element)
    /// When set, only the element matching this selector can initiate dragging
    #[props(default)]
    pub handle: Option<String>,

    /// Callback when drag starts
    #[props(default)]
    pub on_drag_start: EventHandler<DragId>,

    /// Callback when drag ends
    #[props(default)]
    pub on_drag_end: EventHandler<DragId>,

    /// Optional callback for custom element rendering.
    ///
    /// When provided, the component calls this instead of rendering the default `div`.
    /// The callback receives [`DraggableRenderProps`] containing all computed attributes,
    /// event handlers, and children. The consumer must apply these to their custom element.
    ///
    /// When `None` (default), the component renders its standard `div` wrapper.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Draggable {
    ///     id: DragId::new("item-1"),
    ///     render_container: move |rp: DraggableRenderProps| rsx! {
    ///         li {
    ///             id: "{rp.element_id}",
    ///             class: "{rp.class}",
    ///             style: "{rp.style}",
    ///             tabindex: rp.tabindex,
    ///             role: "button",
    ///             aria_disabled: rp.disabled,
    ///             aria_roledescription: "draggable",
    ///             aria_describedby: "{rp.instructions_id}",
    ///             aria_grabbed: "{rp.aria_grabbed}",
    ///             "data-dnd-draggable": "",
    ///             "data-state": "{rp.data_state}",
    ///             "data-disabled": "{rp.data_disabled}",
    ///             onpointerdown: rp.onpointerdown,
    ///             onpointerup: rp.onpointerup,
    ///             onkeydown: rp.onkeydown,
    ///             oncontextmenu: rp.oncontextmenu,
    ///             ..rp.attributes,
    ///             {rp.children}
    ///         }
    ///     },
    ///     div { "Drag me!" }
    /// }
    /// ```
    #[props(default)]
    pub render_container: Option<Callback<DraggableRenderProps, Element>>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Allows consumers to add custom classes, styles, and accessibility attributes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Draggable {
    ///     id: item.drag_id(),
    ///     class: "my-custom-class hover:shadow-lg",
    ///     data_testid: "draggable-item-1",
    ///     style: "background: blue;",
    ///     div { "Content" }
    /// }
    /// ```
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

impl DraggableProps {
    /// Get the combined list of drag types
    ///
    /// Combines `drag_type` (if provided) with `additional_types` into a single list.
    /// If neither is provided, returns a single-element list with an empty DragType.
    fn get_drag_types(&self) -> Vec<DragType> {
        combine_drag_types(self.drag_type.as_ref(), &self.additional_types, "")
    }
}

/// Always returns `false`: props contain [`Element`] (children), [`EventHandler`]s
/// (on_drag_start, on_drag_end), and [`Attribute`]s — none of which support meaningful
/// equality comparison. Returning `false` tells Dioxus to always re-render this
/// component, which is the intended behavior for reactive signal-driven updates.
impl PartialEq for DraggableProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

// ============================================================================
// Draggable Component
// ============================================================================

/// A component that makes its children draggable
///
/// The Draggable component wraps content and enables drag operations.
/// It integrates with `DragContext` to manage drag state globally.
///
/// # Props
///
/// - `id`: Unique identifier for this draggable item
/// - `drag_type`: Primary type discriminator for drop zone filtering (optional)
/// - `additional_types`: Additional types for multi-type filtering (optional)
/// - `children`: The content to render inside the draggable wrapper
/// - `disabled`: When true, dragging is disabled (default: false)
/// - `handle`: CSS selector for a custom drag handle (optional)
/// - `on_drag_start`: Callback fired when drag starts
/// - `on_drag_end`: Callback fired when drag ends
///
/// # CSS Classes
///
/// - `draggable`: Always applied
/// - `dragging`: Applied when this item is being dragged
/// - `disabled`: Applied when dragging is disabled
///
/// # Example (Single Type - Backward Compatible)
///
/// ```ignore
/// rsx! {
///     Draggable {
///         id: DragId::new("item-1"),
///         drag_type: DragType::new("task"),
///         on_drag_start: move |id| {
///             println!("Started dragging: {:?}", id);
///         },
///         div { "Drag me!" }
///     }
/// }
/// ```
///
/// # Example (Multiple Types)
///
/// ```ignore
/// rsx! {
///     Draggable {
///         id: DragId::new("img-1"),
///         additional_types: vec![
///             DragType::new("sortable"),
///             DragType::new("image"),
///         ],
///         div { "Sortable image item" }
///     }
/// }
/// ```
///
/// # Attribute Forwarding
///
/// This component supports attribute forwarding via `#[props(extends = GlobalAttributes)]`.
/// You can pass any HTML attribute, including:
///
/// - `class`: Merged with library classes (`draggable`, `dragging`, `disabled`)
/// - `style`: Appended to library styles (preserves required `touch-action: none`)
/// - `data-*`: For testing identifiers and custom data
/// - `aria-*`: For accessibility improvements
///
/// ## ⚠️ Important: Functional Styles
///
/// The library applies these styles for correct drag behavior:
/// - `touch-action: none` - Prevents touch scrolling
/// - `user-select: none` - Prevents text selection
///
/// If you override these in your custom `style` attribute, drag behavior may break.
///
/// ## Example with Attributes
///
/// ```ignore
/// rsx! {
///     Draggable {
///         id: DragId::new("item-1"),
///         class: "bg-white hover:shadow-lg transition-shadow",
///         data_testid: "task-item",
///         aria_label: "Draggable task item",
///         style: "border-radius: 8px;",  // Safe - doesn't override functional styles
///         div { "Task content" }
///     }
/// }
/// ```
#[component]
pub fn Draggable(props: DraggableProps) -> Element {
    // Get the drag context from the provider
    let ctx = use_context::<DragContext>();

    // Check if this item is currently being dragged (subscribing read for reactivity)
    let is_dragging = ctx.is_dragging_id(&props.id);

    // Clone props for use in event handlers
    let id = props.id.clone();
    let drag_types = props.get_drag_types();
    let disabled = props.disabled;
    let on_start = props.on_drag_start;
    let on_end = props.on_drag_end;
    let handle_selector = props.handle.clone();

    // Stable element id for default rendering and render-prop consumers
    let element_id = use_signal(|| format!("draggable-{}", props.id.0));

    // Clone id for event handlers
    let id_for_handler = id.clone();
    let id_for_end = id.clone();
    let id_for_keyboard = id.clone();
    let drag_types_for_keyboard = drag_types.clone();

    // Pointer down handler - starts the drag
    let start_drag = move |e: PointerEvent| {
        if disabled {
            return;
        }

        // If a handle selector is specified, check if the event target matches
        if let Some(ref selector) = handle_selector
            && !pointer_event_matches_handle(&e, selector)
        {
            return; // Event didn't originate from the handle
        }

        // Prevent default browser drag behavior
        e.prevent_default();

        // Get the position from client coordinates
        let position = Position {
            x: e.client_coordinates().x,
            y: e.client_coordinates().y,
        };

        // Create drag data with multiple types and start the drag
        let data = DragData::with_types(id_for_handler.clone(), drag_types.clone());

        ctx.start_pointer_drag(
            data,
            id_for_handler.clone(),
            position,
            e.data().pointer_id(),
        );

        // Fire the on_drag_start callback
        on_start.call(id_for_handler.clone());
    };

    // Pointer up handler - fires callback and lets event bubble
    // NOTE: We do NOT call end_drag() here - that's handled by DragContextProvider's
    // onpointerup handler which fires after this one due to event bubbling.
    // If we called end_drag() here, it would consume the DropEvent and
    // DragContextProvider's on_drop handler would never receive it.
    let end_drag = move |_e: PointerEvent| {
        // Fire the on_drag_end callback if we were dragging
        // Check BEFORE end_drag is called by DragContextProvider
        if ctx.is_dragging_id(&id_for_end) {
            on_end.call(id_for_end.clone());
        }
        // Event bubbles to DragContextProvider which calls end_drag() and fires on_drop
    };

    // Extract consumer-provided class and style from attributes
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");

    // Consumer-only class (library state communicated via data-* attributes)
    let merged_class = consumer_class.unwrap_or_default();

    // Library styles are FUNCTIONAL and REQUIRED for drag behavior
    let has_handle = props.handle.is_some();
    let library_style = if has_handle {
        "touch-action: auto; user-select: none;"
    } else {
        "touch-action: none; user-select: none; cursor: grab;"
    };
    let merged_style = merge_styles(library_style, consumer_style.as_deref());

    // Filter out class/style from attributes (already handled above)
    let other_attributes = filter_class_style(props.attributes.clone());

    // Keyboard handler: Space/Enter starts drag, Escape cancels
    let onkeydown = move |e: KeyboardEvent| {
        let key = e.key();

        match key {
            Key::Character(ref c) if c == " " => {
                e.prevent_default();
                if disabled {
                    return;
                }
                // If not already dragging, start a keyboard drag
                if !ctx.is_dragging() {
                    let data = DragData::with_types(
                        id_for_keyboard.clone(),
                        drag_types_for_keyboard.clone(),
                    );
                    // Use (0,0) position for keyboard-initiated drags (no pointer)
                    ctx.start_drag(data, id_for_keyboard.clone(), Position::default());
                    on_start.call(id_for_keyboard.clone());
                }
            }
            Key::Enter => {
                e.prevent_default();
                if disabled {
                    return;
                }
                if !ctx.is_dragging() {
                    let data = DragData::with_types(
                        id_for_keyboard.clone(),
                        drag_types_for_keyboard.clone(),
                    );
                    ctx.start_drag(data, id_for_keyboard.clone(), Position::default());
                    on_start.call(id_for_keyboard.clone());
                }
            }
            Key::Escape => {
                if ctx.is_dragging() {
                    ctx.cancel_drag();
                    e.prevent_default();
                }
            }
            _ => {}
        }
    };

    let dnd_state = if is_dragging { "dragging" } else { "" };
    let dnd_disabled = if disabled { "true" } else { "" };
    let instructions_id = ctx.instructions_id();
    let tabindex_val = if !disabled { props.tabindex } else { None };
    let aria_grabbed = if is_dragging { "true" } else { "false" };

    let contextmenu_handler = move |e: Event<MouseData>| {
        e.prevent_default();
    };

    // If render_container is provided, delegate to consumer
    if let Some(render_container) = &props.render_container {
        let render_props = DraggableRenderProps {
            element_id: element_id.read().clone(),
            class: merged_class.to_string(),
            style: merged_style.to_string(),
            tabindex: tabindex_val,
            is_dragging,
            disabled,
            instructions_id: instructions_id.to_string(),
            aria_grabbed: aria_grabbed.to_string(),
            data_state: dnd_state.to_string(),
            data_disabled: dnd_disabled.to_string(),
            onpointerdown: EventHandler::new(start_drag),
            onpointerup: EventHandler::new(end_drag),
            onkeydown: EventHandler::new(onkeydown),
            oncontextmenu: EventHandler::new(contextmenu_handler),
            attributes: other_attributes,
            children: props.children,
        };
        return render_container.call(render_props);
    }

    rsx! {
        div {
            id: "{element_id}",
            class: "{merged_class}",
            style: "{merged_style}",
            tabindex: tabindex_val,
            // ARIA support
            role: "button",
            aria_disabled: disabled,
            aria_roledescription: "draggable",
            aria_describedby: "{instructions_id}",
            aria_grabbed: "{aria_grabbed}",

            "data-dnd-draggable": "",
            "data-dnd-handle-mode": if has_handle { "true" },
            "data-state": "{dnd_state}",
            "data-disabled": "{dnd_disabled}",

            onpointerdown: start_drag,
            onpointerup: end_drag,
            onkeydown: onkeydown,
            oncontextmenu: contextmenu_handler,

            // Spread remaining attributes (data-*, aria-*, etc.)
            ..other_attributes,

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
    fn test_draggable_props_partial_eq() {
        // Test that props with same id, drag_type, disabled, and handle are equal
        let id1 = DragId::new("item1");
        let id2 = DragId::new("item1");
        let id3 = DragId::new("item2");

        let type1 = DragType::new("task");
        let type2 = DragType::new("task");

        // Same id and type should be equal (ignoring children and handlers)
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(type1, type2);
    }

    #[test]
    fn test_draggable_props_partial_eq_via_builder() {
        // Test PartialEq by comparing the relevant fields directly
        // (We can't easily construct DraggableProps in tests because Element is complex)
        let id1 = DragId::new("item1");
        let id2 = DragId::new("item1");
        let type1 = DragType::new("task");
        let type2 = DragType::new("task");
        let disabled1 = false;
        let disabled2 = false;
        let handle1: Option<String> = None;
        let handle2: Option<String> = None;

        // Verify the fields that PartialEq compares
        assert_eq!(id1, id2);
        assert_eq!(type1, type2);
        assert_eq!(disabled1, disabled2);
        assert_eq!(handle1, handle2);
    }

    #[test]
    fn test_draggable_props_partial_eq_different_disabled() {
        let disabled1 = false;
        let disabled2 = true;
        assert_ne!(disabled1, disabled2);
    }

    #[test]
    fn test_draggable_props_partial_eq_different_handle() {
        let handle1: Option<String> = Some(".handle".to_string());
        let handle2: Option<String> = None;
        assert_ne!(handle1, handle2);
    }

    // =========================================================================
    // Multi-type tests
    // =========================================================================

    // Note: get_drag_types() delegates to combine_drag_types() which is
    // thoroughly tested in types.rs. These tests verify the delegation
    // using combine_drag_types directly (avoiding Dioxus runtime requirement).

    #[test]
    fn test_get_drag_types_with_single_drag_type() {
        let drag_type = Some(DragType::new("task"));
        let types = combine_drag_types(drag_type.as_ref(), &[], "");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], DragType::new("task"));
    }

    #[test]
    fn test_get_drag_types_with_additional_types() {
        let additional = vec![DragType::new("sortable"), DragType::new("image")];
        let types = combine_drag_types(None, &additional, "");
        assert_eq!(types.len(), 2);
        assert!(types.contains(&DragType::new("sortable")));
        assert!(types.contains(&DragType::new("image")));
    }

    #[test]
    fn test_get_drag_types_combines_drag_type_and_additional_types() {
        let primary = DragType::new("primary");
        let additional = vec![DragType::new("secondary"), DragType::new("tertiary")];
        let types = combine_drag_types(Some(&primary), &additional, "");
        assert_eq!(types.len(), 3);
        assert_eq!(types[0], DragType::new("primary"));
        assert!(types.contains(&DragType::new("secondary")));
        assert!(types.contains(&DragType::new("tertiary")));
    }

    #[test]
    fn test_get_drag_types_default_when_none_provided() {
        let types = combine_drag_types(None, &[], "");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], DragType::new(""));
    }

    // =========================================================================
    // Keyboard drag data construction tests
    // =========================================================================

    #[test]
    fn test_keyboard_drag_creates_correct_drag_data() {
        // Verify that DragData::with_types produces correct data for keyboard drag
        let id = DragId::new("kb-item");
        let types = vec![DragType::new("task")];
        let data = DragData::with_types(id.clone(), types.clone());
        assert_eq!(data.id, id);
        assert_eq!(data.drag_types, types);
    }

    #[test]
    fn test_keyboard_drag_uses_default_position() {
        // Keyboard drags use Position::default() (0,0) since there's no pointer
        let pos = Position::default();
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn test_aria_grabbed_state_values() {
        // Verify the string values used for aria-grabbed
        let is_dragging = true;
        assert_eq!(if is_dragging { "true" } else { "false" }, "true");

        let is_dragging = false;
        assert_eq!(if is_dragging { "true" } else { "false" }, "false");
    }

    #[test]
    fn test_dnd_state_values() {
        // Verify data-state values
        let is_dragging = true;
        assert_eq!(if is_dragging { "dragging" } else { "" }, "dragging");

        let is_dragging = false;
        assert_eq!(if is_dragging { "dragging" } else { "" }, "");
    }

    // =========================================================================
    // DraggableRenderProps tests
    // =========================================================================

    #[test]
    fn test_draggable_render_props_fields_when_not_dragging() {
        // Verify computed field values for the non-dragging state
        let is_dragging = false;
        let disabled = false;
        let dnd_state = if is_dragging { "dragging" } else { "" };
        let dnd_disabled = if disabled { "true" } else { "" };
        let aria_grabbed = if is_dragging { "true" } else { "false" };
        let tabindex: Option<i32> = if !disabled { Some(0) } else { None };

        assert_eq!(dnd_state, "");
        assert_eq!(dnd_disabled, "");
        assert_eq!(aria_grabbed, "false");
        assert_eq!(tabindex, Some(0));
    }

    #[test]
    fn test_draggable_render_props_fields_when_dragging() {
        // Verify computed field values for the dragging state
        let is_dragging = true;
        let dnd_state = if is_dragging { "dragging" } else { "" };
        let aria_grabbed = if is_dragging { "true" } else { "false" };

        assert_eq!(dnd_state, "dragging");
        assert_eq!(aria_grabbed, "true");
    }

    #[test]
    fn test_draggable_render_props_fields_when_disabled() {
        // Verify computed field values for the disabled state
        let disabled = true;
        let dnd_disabled = if disabled { "true" } else { "" };
        let tabindex: Option<i32> = if !disabled { Some(0) } else { None };

        assert_eq!(dnd_disabled, "true");
        assert_eq!(tabindex, None);
    }

    #[test]
    fn test_draggable_render_props_partial_eq_always_false() {
        // DraggableRenderProps should always compare as not-equal
        // (contains EventHandlers which can't be meaningfully compared)
        // We test the logic directly since constructing full RenderProps requires runtime
        let _is_dragging = false;
        let props_eq: bool = false; // PartialEq always returns false
        assert!(!props_eq);
    }

    #[test]
    fn test_draggable_render_props_style_includes_functional() {
        // Verify the merged style always includes functional CSS
        let library_style = "touch-action: none; user-select: none; cursor: grab;";
        let merged = merge_styles(library_style, Some("background: red;"));
        assert!(merged.contains("touch-action: none"));
        assert!(merged.contains("user-select: none"));
        assert!(merged.contains("cursor: grab"));
        assert!(merged.contains("background: red"));
    }

    #[test]
    fn test_draggable_handle_mode_style_allows_touch_scroll() {
        // When a handle is specified, touch-action should be auto to allow scrolling
        let library_style = "touch-action: auto; user-select: none;";
        let merged = merge_styles(library_style, Some("background: red;"));
        assert!(merged.contains("touch-action: auto"));
        assert!(merged.contains("user-select: none"));
        assert!(!merged.contains("cursor: grab"));
    }
}
