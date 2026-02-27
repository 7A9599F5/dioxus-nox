//! Drop indicator component
//!
//! Visual indicator showing where items will be dropped. Shows a horizontal
//! line for vertical lists and a vertical line for horizontal lists.

use dioxus::prelude::*;

use crate::types::Orientation;
use crate::utils::{extract_attribute, filter_class_style};

use super::item::IndicatorPosition;

// ============================================================================
// DropIndicator Component
// ============================================================================

/// Props for the DropIndicator component
#[derive(Props, Clone)]
pub struct DropIndicatorProps {
    /// Layout orientation of the list
    pub orientation: Orientation,

    /// Position of the indicator relative to the item (before or after)
    #[props(default)]
    pub position: Option<IndicatorPosition>,

    /// Optional preview content to render instead of the thin line indicator.
    /// When provided, renders a ghost preview card in the displacement gap.
    #[props(default)]
    pub preview: Option<Element>,

    /// Height of the displacement gap (dragged item's height).
    /// When provided with a preview, sizes the preview to fill the gap.
    #[props(default)]
    pub gap_height: Option<f64>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Always returns `false`: props contain [`Element`] (preview) and [`Attribute`]s — neither
/// of which supports meaningful equality comparison. Returning `false` tells Dioxus to
/// always re-render this component, which is the intended behavior for reactive
/// signal-driven updates.
impl PartialEq for DropIndicatorProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// A visual indicator showing where an item will be dropped
///
/// The indicator displays as:
/// - A horizontal line for vertical lists (items stacked top to bottom)
/// - A vertical line for horizontal lists (items arranged left to right)
///
/// # Example
///
/// ```ignore
/// // For a vertical list
/// rsx! {
///     DropIndicator { orientation: Orientation::Vertical }
/// }
///
/// // For a horizontal list
/// rsx! {
///     DropIndicator { orientation: Orientation::Horizontal }
/// }
/// ```
#[component]
pub fn DropIndicator(props: DropIndicatorProps) -> Element {
    let orientation = props.orientation;
    let position = props.position;
    let preview = props.preview;
    let gap_height = props.gap_height;
    // Determine CSS class based on orientation
    // For vertical lists, the indicator is horizontal (spans width)
    // For horizontal lists, the indicator is vertical (spans height)
    let orientation_class = match orientation {
        Orientation::Vertical => "horizontal",
        Orientation::Horizontal => "vertical",
    };

    let position_class = match position {
        Some(IndicatorPosition::Before) => "before",
        Some(IndicatorPosition::After) => "after",
        None => "",
    };

    // Extract consumer class and style from attributes
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");
    let remaining_attrs = filter_class_style(props.attributes);

    if let Some(preview_content) = preview {
        // Render preview card sized to fill the displacement gap
        // For vertical lists, gap is vertical (min-height). For horizontal, gap is horizontal (min-width).
        let base_style = gap_height
            .map(|h| match orientation {
                Orientation::Vertical => format!("min-height: {}px;", h),
                Orientation::Horizontal => format!("min-width: {}px;", h),
            })
            .unwrap_or_default();
        let merged_style = match consumer_style {
            Some(s) if !s.is_empty() => format!("{} {}", base_style, s),
            _ => base_style,
        };
        let merged_class = consumer_class.unwrap_or_default();
        rsx! {
            div {
                class: "{merged_class}",
                style: "{merged_style}",
                "data-dnd-preview": "",
                "data-indicator-position": "{position_class}",
                aria_hidden: "true",
                ..remaining_attrs,
                {preview_content}
            }
        }
    } else {
        // Default thin line indicator
        let merged_class = consumer_class.unwrap_or_default();
        let merged_style = consumer_style.unwrap_or_default();
        rsx! {
            div {
                class: "{merged_class}",
                style: "{merged_style}",
                "data-dnd-indicator": "",
                "data-orientation": "{orientation_class}",
                "data-indicator-position": "{position_class}",
                aria_hidden: "true",
                ..remaining_attrs,
            }
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
    fn test_orientation_default_is_vertical() {
        assert_eq!(Orientation::default(), Orientation::Vertical);
    }
}
