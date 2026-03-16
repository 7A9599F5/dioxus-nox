//! Core types for dioxus-nox-toggle-group.

use dioxus::prelude::*;

/// Layout orientation for the toggle group.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    /// Returns the `aria-orientation` attribute value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Context shared between toggle group compound components.
#[derive(Clone)]
pub struct ToggleGroupContext {
    /// Currently active value(s) — comma-separated for multi-select.
    pub value: String,
    /// Change handler.
    pub on_value_change: EventHandler<String>,
    /// Whether multi-select is enabled.
    pub multi_select: bool,
    /// Layout orientation.
    pub orientation: Orientation,
}
