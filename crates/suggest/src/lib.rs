//! # dioxus-nox-suggest
//!
//! Headless inline-trigger suggestion primitive for Dioxus 0.7.
//!
//! Covers the "type a special char, pick from a floating list" pattern:
//! slash commands (`/`), @mentions, `#`hashtags, and any custom trigger char.
//!
//! ## Quick start
//!
//! ```ignore
//! use dioxus_nox_suggest::{TriggerConfig, TriggerSelectEvent, suggest, use_suggestion};
//!
//! #[component]
//! fn MyEditor() -> Element {
//!     rsx! {
//!         suggest::Root {
//!             triggers: vec![TriggerConfig::slash(), TriggerConfig::mention()],
//!             on_select: move |evt: TriggerSelectEvent| {
//!                 // evt.trigger_char, evt.value, evt.filter, evt.trigger_offset
//!             },
//!             suggest::Trigger {
//!                 textarea { … }
//!             }
//!             suggest::List {
//!                 suggest::Item { value: "heading1", "Heading 1" }
//!                 suggest::Item { value: "heading2", "Heading 2" }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Key types
//!
//! - [`TriggerConfig`] — configure each trigger char (`char`, `line_start_only`, etc.)
//! - [`TriggerSelectEvent`] — event fired on selection; includes `trigger_offset`
//!   for computing the text-replacement range
//! - [`SuggestionHandle`] — returned by [`use_suggestion`]; read-only context access
//!
//! ## Composing with dioxus-nox-cmdk
//!
//! For consumers who also use `dioxus-nox-cmdk` and want nucleo-powered fuzzy
//! filtering, feed the active filter into cmdk's search:
//!
//! ```ignore
//! // In consumer code — no dependency on cmdk inside this crate
//! let sg = use_suggestion();
//! use_effect(move || { cmd_ctx.search.set(sg.filter()); });
//! ```

mod components;
mod hook;
mod placement;
mod trigger;
mod types;

#[cfg(test)]
mod tests;

pub use hook::{SuggestionHandle, use_suggestion};
pub use types::{TriggerConfig, TriggerContext, TriggerSelectEvent};

// Re-export pure utility functions for consumers who want char-agnostic helpers.
pub use placement::compute_float_style;
pub use trigger::{detect_trigger, extract_filter};

/// Compound components for building suggestion UIs.
///
/// Assemble parts explicitly to match your layout:
///
/// ```text
/// suggest::Root { suggest::Trigger { … } suggest::List { suggest::Item { … } } }
/// ```
pub mod suggest {
    pub use super::components::{Empty, Group, Item, List, Root, Trigger};
}
