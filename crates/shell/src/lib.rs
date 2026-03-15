//! # dioxus-shell
//!
//! Application shell layout primitive for Dioxus.
//!
//! Provides [`AppShell`] — a persistent, always-visible split-pane layout
//! with named slots (sidebar, children, preview, footer). Headless: all
//! layout is CSS-driven via `data-shell*` attributes.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use dioxus_nox_shell::prelude::*;
//!
//! AppShell {
//!     sidebar: rsx! { MySidebar {} },
//!     MyMainContent {}
//! }
//! ```

pub mod breakpoint;
mod context;
mod shell;
#[cfg(test)]
mod tests;

pub use breakpoint::{
    BreakpointConfig, DesktopSidebar, MobileSidebar, SheetSnap, ShellBreakpoint,
    use_shell_breakpoint,
};
pub use context::{ShellContext, use_shell_context};
pub use shell::{AppShell, MobileSidebarBackdrop, ShellLayout};
