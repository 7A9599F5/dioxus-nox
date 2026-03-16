//! Core types for dioxus-nox-drawer.

use dioxus::prelude::*;

/// Which edge the drawer slides from.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DrawerSide {
    Left,
    #[default]
    Right,
    Bottom,
    Top,
}

impl DrawerSide {
    /// Returns the `data-side` attribute value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Top => "top",
        }
    }
}

/// Context shared between drawer compound components via Dioxus context API.
#[derive(Clone)]
pub struct DrawerContext {
    /// Whether the drawer is currently open.
    pub open: bool,
    /// Close handler.
    pub on_close: EventHandler<()>,
    /// Whether to close on overlay click.
    pub close_on_overlay: bool,
    /// Which edge the drawer slides from.
    pub side: DrawerSide,
    /// Auto-generated unique ID for this drawer instance.
    pub instance_id: u32,
}
