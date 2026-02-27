//! # dioxus-cmdk
//!
//! A headless, accessible **Command Palette** primitive for
//! [Dioxus 0.7](https://dioxuslabs.com/), inspired by [cmdk](https://cmdk.paco.me/).
//!
//! ## Features
//!
//! - **Fuzzy search** powered by [nucleo](https://crates.io/crates/nucleo-matcher)
//! - **Accessible** — ARIA roles, keyboard navigation, screen-reader announcements
//! - **Composable** — headless components you style yourself
//! - **Dialog mode** — built-in modal with focus trap and Cmd/Ctrl+K shortcut
//! - **Sheet mode** — mobile bottom-sheet with drag gestures and snap points
//! - **Pages** — multi-step drill-in navigation with `CommandPage` and `use_command_pages`
//! - **Custom filters** — bring your own scoring function
//! - **Match highlighting** — `CommandHighlight` for fuzzy-match character highlighting
//! - **Scoring strategies** — pluggable post-nucleo score adjustment
//! - **Modes** — prefix-based mode filtering (e.g., `>` for commands)
//! - **Global shortcuts** — document-level shortcuts with chord support
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//! use dioxus_nox_cmdk::*;
//!
//! #[component]
//! fn App() -> Element {
//!     let palette = use_command_palette(true); // Cmd/Ctrl+K shortcut
//!
//!     rsx! {
//!         CommandDialog { open: palette.open,
//!             CommandRoot { on_select: move |val: String| { /* handle */ },
//!                 CommandInput { placeholder: "Type a command..." }
//!                 CommandList {
//!                     CommandEmpty { "No results." }
//!                     CommandGroup { id: "actions", heading: "Actions",
//!                         CommandItem { id: "new", label: "New File",
//!                             "New File"
//!                         }
//!                     }
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Feature flags
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `web` | yes | Enables the Dioxus web renderer |
//! | `desktop` | no | Enables the Dioxus desktop renderer |
//! | `mobile` | no | Enables the Dioxus mobile renderer |
//!
//! See the [README](https://github.com/7A9599F5/dioxus-cmdk) for full documentation.

mod components;
mod context;
pub(crate) mod helpers;
mod hook;
pub(crate) mod navigation;
pub(crate) mod placement;
pub(crate) mod scoring;
mod shortcut;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API
pub use components::{
    CommandAction, CommandActionPanel, CommandAnchor, CommandDialog, CommandEmpty, CommandForm,
    CommandFormField, CommandGroup, CommandHighlight, CommandInput, CommandItem, CommandList,
    CommandLoading, CommandModeIndicator, CommandPage, CommandPalette, CommandPreview,
    CommandQuickInput, CommandRoot, CommandSeparator, CommandSheet, CommandShortcut,
};
pub use context::{CommandContext, use_command_context};
pub use hook::{
    AdaptivePaletteHandle, CommandHistoryHandle, CommandModesHandle, CommandPagesHandle,
    CommandPaletteHandle, GlobalShortcutHandle, use_adaptive_palette, use_async_commands,
    use_command_history, use_command_modes, use_command_pages, use_command_palette,
    use_command_palette_handle, use_command_sheet, use_global_shortcuts, use_is_mobile,
    use_scored_item,
};
#[cfg(feature = "router")]
pub use hook::{RouterSyncHandle, use_router_sync};
pub use keyboard_types::{Key, Modifiers};
pub use shortcut::{Hotkey, HotkeyParseError};
pub use types::{
    ActionPanelState, ActionRegistration, AnimationState, AsyncCommandHandle, AsyncItem,
    ChordShortcut, ChordState, CustomFilter, FormFieldType, FormValue, FrecencyStrategy,
    GlobalShortcut, GroupRegistration, ItemRegistration, ItemSelectCallback, ModeRegistration,
    PageRegistration, PaletteMode, ScoredItem, ScoringStrategy, ScoringStrategyProp,
    SelectOption, Side,
};
