//! Core types for dioxus-nox-master-detail.

use dioxus::prelude::*;

/// Context shared between master-detail compound components.
#[derive(Clone)]
pub struct MasterDetailContext {
    /// Whether the detail panel is open.
    pub detail_open: bool,
    /// Handler to close the detail panel.
    pub on_detail_close: EventHandler<()>,
}
