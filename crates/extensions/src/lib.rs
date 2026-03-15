//! # dioxus-nox-extensions
//!
//! Runtime plugin system for Dioxus applications.
//!
//! Extensions register at runtime, contribute searchable commands, and have
//! activate/deactivate lifecycle hooks — making it easy to build extensible
//! UIs such as command palettes, plugin managers, or modular tool panels.
//!
//! ## Layers
//!
//! | Layer | Items | Dioxus dependency? |
//! |-------|-------|--------------------|
//! | **Types** | [`Extension`], [`PluginCommand`], [`ExtensionInfo`] | Minimal (`EventHandler` only) |
//! | **Hook** | [`use_extensions`], [`ExtensionHandle`] | Yes (Signals, context) |
//! | **Context** | [`ExtensionContext`], [`use_extension_context`] | Yes (Signals, context) |
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//! use dioxus_nox_extensions::*;
//! use std::rc::Rc;
//!
//! struct WordCount;
//!
//! impl Extension for WordCount {
//!     fn id(&self) -> &str { "word-count" }
//!     fn name(&self) -> &str { "Word Count" }
//!     fn commands(&self) -> Vec<PluginCommand> {
//!         vec![PluginCommand::new("word-count.run", "Count Words")]
//!     }
//! }
//!
//! #[component]
//! fn App() -> Element {
//!     let handle = use_extensions(None, None);
//!     handle.register(Rc::new(WordCount));
//!
//!     rsx! {
//!         div { "Extensions: {handle.extensions().len()}" }
//!     }
//! }
//! ```

mod context;
mod hook;
mod types;

#[cfg(test)]
mod tests;

pub use context::{ExtensionContext, use_extension_context};
pub use hook::{ExtensionHandle, use_extensions};
pub use types::{CommandSelectCallback, Extension, ExtensionInfo, PluginCommand};
