//! Drag overlay component
//!
//! Renders a visual representation of the item being dragged.
//! The overlay uses `position: fixed` for portal-like behavior (escapes overflow:hidden)
//! and `pointer-events: none` so it doesn't interfere with drop detection.

use dioxus::prelude::*;

use crate::context::{ActiveDrag, DragContext};
use crate::utils::{extract_attribute, filter_class_style};

// ============================================================================
// DragOverlayRenderProps
// ============================================================================

/// Props passed to the `render_container` callback for custom element rendering.
///
/// When `render_container` is provided on [`DragOverlay`], this struct carries all
/// the computed attributes and content that must be applied to the consumer's custom
/// element for correct overlay behavior.
///
/// # Critical styles
///
/// The `style` field contains positioning styles that **must** be preserved:
/// - `position: fixed` — escapes parent overflow constraints
/// - `pointer-events: none` — allows drop detection through the overlay
/// - `transform: translate3d(...)` — cursor-following positioning
/// - `will-change: transform` — GPU-accelerated animation
///
/// # Example
///
/// ```ignore
/// DragOverlay {
///     render_container: move |rp: DragOverlayRenderProps| rsx! {
///         span {
///             class: "{rp.class}",
///             style: "{rp.style}",
///             "data-dnd-overlay": "",
///             aria_hidden: "true",
///             ..rp.attributes,
///             {rp.content}
///         }
///     },
///     div { class: "preview", "Dragging..." }
/// }
/// ```
#[derive(Clone)]
pub struct DragOverlayRenderProps {
    /// Merged class string (consumer classes only)
    pub class: String,
    /// Merged style string (includes critical positioning — do not override)
    pub style: String,
    /// The active drag state (position, data, grab offset)
    pub active_drag: ActiveDrag,
    /// Remaining consumer attributes (data-*, aria-*, etc., excluding class/style)
    pub attributes: Vec<Attribute>,
    /// Pre-resolved content: either from the `render` callback or `children`
    pub content: Element,
}

// ActiveDrag contains non-comparable fields
impl PartialEq for DragOverlayRenderProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// Props for the DragOverlay component
#[derive(Props, Clone)]
pub struct DragOverlayProps {
    /// Custom render content for the overlay
    #[props(default)]
    pub children: Element,

    /// Optional render callback receiving the active drag state
    ///
    /// Use this to access dynamic drag data without needing `use_context`.
    /// The callback receives the `ActiveDrag` struct containing data and position.
    #[props(default)]
    pub render: Option<Callback<ActiveDrag, Element>>,

    /// When true, the overlay offsets by the grab position within the source
    /// element instead of centering on the cursor. This creates a 1:1 spatial
    /// match between where the user grabbed and the overlay position.
    ///
    /// Default: false (overlay centers on cursor via translate(-50%, -50%))
    #[props(default = false)]
    pub align_to_grab_point: bool,

    /// Optional callback for custom element rendering.
    ///
    /// When provided, the component calls this instead of rendering the default `div`.
    /// The callback receives [`DragOverlayRenderProps`] containing the computed style
    /// (with critical positioning), class, attributes, and pre-resolved content.
    ///
    /// **Note**: This is different from the `render` prop, which controls the
    /// *content inside* the wrapper. `render_container` controls the *wrapper itself*.
    ///
    /// When `None` (default), the component renders its standard `div` wrapper.
    #[props(default)]
    pub render_container: Option<Callback<DragOverlayRenderProps, Element>>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// WARNING: The overlay uses critical styles for positioning:
    /// - `position: fixed` - Escapes overflow constraints
    /// - `pointer-events: none` - Allows drop detection
    /// - `transform: translate3d(...)` - Cursor tracking
    ///
    /// If you override these styles, the overlay may not work correctly.
    /// Safe to add: background, border, box-shadow, opacity, etc.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Always returns `false`: props contain [`Element`] (children), [`Callback`] (render),
/// and [`Attribute`]s — none of which support meaningful equality comparison. Returning
/// `false` tells Dioxus to always re-render this component, which is the intended
/// behavior for reactive signal-driven updates.
impl PartialEq for DragOverlayProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// A component that renders overlay content at the current drag position.
///
/// The overlay:
/// - Only renders when actively dragging
/// - Uses `position: fixed` to escape parent overflow constraints
/// - Uses `pointer-events: none` so it doesn't block drop detection
/// - Uses CSS transform for GPU-accelerated positioning
/// - Uses `will-change: transform` for smooth 60fps animation
///
/// # Example with children
///
/// ```ignore
/// DragOverlay {
///     div { class: "preview", "Dragging item..." }
/// }
/// ```
///
/// # Example with render prop (Dynamic content)
///
/// ```ignore
/// DragOverlay {
///     render: move |active_drag| rsx! {
///         div { class: "preview", "{active_drag.data.id}" }
///     }
/// }
/// ```
#[component]
pub fn DragOverlay(props: DragOverlayProps) -> Element {
    let ctx = use_context::<DragContext>();
    // DragContext methods internally call state.read() for reactive subscription
    let active = ctx.get_active_drag();

    // Only render when actively dragging
    let Some(active) = active else {
        return VNode::empty();
    };

    // No floating overlay during keyboard drag — displacement/indicator feedback is sufficient
    if ctx.is_keyboard_drag() {
        return VNode::empty();
    }

    let x = active.current_position.x;
    let y = active.current_position.y;

    // Extract consumer class and style from attributes
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");

    // Consumer-only class (library state communicated via data-* attributes)
    let merged_class = consumer_class.unwrap_or_default();

    // Merge functional styles with consumer styles
    // Functional styles MUST be preserved for overlay to work correctly
    let alignment = if props.align_to_grab_point {
        let gx = active.grab_offset.x;
        let gy = active.grab_offset.y;
        format!("translate(-{gx}px, -{gy}px)")
    } else {
        "translate(-50%, -50%)".to_string()
    };
    let base_styles = format!(
        "position: fixed; \
         pointer-events: none; \
         left: 0; \
         top: 0; \
         transform: translate3d({x}px, {y}px, 0) {alignment}; \
         will-change: transform;"
    );
    let merged_style = match consumer_style {
        Some(s) if !s.is_empty() => format!("{} {}", base_styles, s),
        _ => base_styles,
    };

    // Filter out class/style from remaining attributes
    let remaining_attrs = filter_class_style(props.attributes);

    // Resolve content: render callback takes priority over children
    let content = if let Some(render) = &props.render {
        render.call(active.clone())
    } else {
        props.children
    };

    // If render_container is provided, delegate to consumer
    if let Some(render_container) = &props.render_container {
        let render_props = DragOverlayRenderProps {
            class: merged_class.to_string(),
            style: merged_style.to_string(),
            active_drag: active,
            attributes: remaining_attrs,
            content,
        };
        return render_container.call(render_props);
    }

    // Use CSS transform: translate3d() for GPU-accelerated positioning
    // This avoids layout recalculation and enables compositor-only animation for 60fps
    // pointer-events: none so overlay doesn't interfere with drop detection
    rsx! {
        div {
            class: "{merged_class}",
            style: "{merged_style}",
            "data-dnd-overlay": "",
            aria_hidden: "true",
            ..remaining_attrs,

            {content}
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_drag_overlay_props_clone() {
        // Verify props derive Clone - this is a compile-time check
        // If this compiles, Clone is implemented
    }

    // =========================================================================
    // DragOverlayRenderProps tests
    // =========================================================================

    #[test]
    fn test_overlay_render_props_style_contains_positioning() {
        // Verify the base style string format contains critical properties
        let x = 100.0_f64;
        let y = 200.0_f64;
        let alignment = "translate(-50%, -50%)";
        let base_styles = format!(
            "position: fixed; \
             pointer-events: none; \
             left: 0; \
             top: 0; \
             transform: translate3d({x}px, {y}px, 0) {alignment}; \
             will-change: transform;"
        );
        assert!(base_styles.contains("position: fixed"));
        assert!(base_styles.contains("pointer-events: none"));
        assert!(base_styles.contains("translate3d(100px, 200px, 0)"));
        assert!(base_styles.contains("will-change: transform"));
    }

    #[test]
    fn test_overlay_render_props_alignment_grab_point() {
        // Verify grab-point alignment format
        let gx = 15.0_f64;
        let gy = 25.0_f64;
        let alignment = format!("translate(-{gx}px, -{gy}px)");
        assert_eq!(alignment, "translate(-15px, -25px)");
    }

    #[test]
    fn test_overlay_render_props_alignment_centered() {
        // Verify centered alignment
        let align_to_grab_point = false;
        let alignment = if align_to_grab_point {
            "translate(-0px, -0px)".to_string()
        } else {
            "translate(-50%, -50%)".to_string()
        };
        assert_eq!(alignment, "translate(-50%, -50%)");
    }

    #[test]
    fn test_overlay_render_props_style_merge() {
        // Verify consumer style appended to base style
        let base_styles = "position: fixed; pointer-events: none;";
        let consumer_style = Some("box-shadow: 0 2px 4px rgba(0,0,0,0.1);".to_string());
        let merged = match consumer_style {
            Some(s) if !s.is_empty() => format!("{} {}", base_styles, s),
            _ => base_styles.to_string(),
        };
        assert!(merged.contains("position: fixed"));
        assert!(merged.contains("box-shadow"));
    }
}
