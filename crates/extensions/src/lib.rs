//! # dioxus-extensions
//!
//! Runtime plugin system for Dioxus applications.
//!
//! See SPEC.md for the full design specification.
//! Implementation: run the BUILD_PROMPT.md prompt in a fresh Claude Code session.
//!
//! ## Planned API
//! - `Extension` trait: `id()`, `name()`, `commands()`, `on_activate()`, `on_deactivate()`
//! - `PluginCommand` struct: `id`, `label`, `keywords`, `group`, `on_select`
//! - `ExtensionHandle`: `register(ext)`, `unregister(ext_id)`, `extensions()`
//! - `use_extensions(register_cb, unregister_cb) -> ExtensionHandle`
//! - `ExtensionInfo` struct: `id`, `name`, `command_count`
