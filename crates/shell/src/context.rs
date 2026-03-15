use crate::ShellLayout;
use crate::breakpoint::{DesktopSidebar, MobileSidebar, SheetSnap, ShellBreakpoint};
use dioxus::prelude::*;

/// Reactive context shared across the shell tree.
///
/// Provided by `AppShell` via `use_context_provider`. Access it from any
/// descendant component with [`use_shell_context`].
#[derive(Clone, Copy)]
pub struct ShellContext {
    /// Current layout mode, reactive via `Signal`.
    pub layout: Signal<ShellLayout>,
    /// Current viewport breakpoint, read-only reactive signal.
    pub breakpoint: ReadSignal<ShellBreakpoint>,
    /// Whether the desktop sidebar is in its expanded state (`true`) or not (`false`).
    ///
    /// For `Full`: `true` = visible at full width, `false` = collapsed to zero.
    /// For `Expandable`: `true` = full width, `false` = rail width.
    /// For `Rail`: ignored (rail is always visible at its fixed width).
    pub sidebar_visible: Signal<bool>,
    /// Whether the mobile overlay sidebar is open.
    pub sidebar_mobile_open: Signal<bool>,
    /// Mobile sidebar variant (Drawer, Rail, or Hidden).
    pub mobile_sidebar: ReadSignal<MobileSidebar>,
    /// Desktop sidebar variant (Full, Rail, or Expandable).
    pub desktop_sidebar: ReadSignal<DesktopSidebar>,
    /// Stack navigation depth. Starts at 1 (root screen).
    pub stack_depth: Signal<u32>,
    /// Whether the full-screen modal is currently presented.
    pub modal_open: Signal<bool>,
    /// Whether the search overlay is currently active.
    pub search_active: Signal<bool>,
    /// Current snap position of the persistent bottom sheet.
    pub sheet_snap: Signal<SheetSnap>,
    /// Callback fired when modal state changes (controlled-mode support).
    pub(crate) on_modal_change: Signal<Option<EventHandler<bool>>>,
    /// Callback fired when search active state changes (controlled-mode support).
    pub(crate) on_search_change: Signal<Option<EventHandler<bool>>>,
}

impl ShellContext {
    /// `true` when the current breakpoint is compact (phone-sized viewport).
    pub fn is_mobile(&self) -> bool {
        (self.breakpoint)().is_compact()
    }

    /// Toggles the appropriate sidebar state based on the current breakpoint.
    ///
    /// - On mobile: toggles `sidebar_mobile_open` (overlay open/closed)
    /// - On desktop `Full` / `Expandable`: toggles `sidebar_visible`
    /// - On desktop `Rail`: no-op (rail is always visible)
    ///
    /// Takes `&self` because [`Signal`] has interior mutability.
    pub fn toggle_sidebar(&self) {
        if self.is_mobile() {
            let mut mob = self.sidebar_mobile_open;
            mob.set(!(self.sidebar_mobile_open)());
        } else if (self.desktop_sidebar)() != DesktopSidebar::Rail {
            let mut vis = self.sidebar_visible;
            vis.set(!(self.sidebar_visible)());
        }
    }

    /// Returns the `data-shell-sidebar-state` attribute value for the root element.
    ///
    /// | Context | Value |
    /// |---------|-------|
    /// | Mobile open | `"open"` |
    /// | Mobile closed | `"closed"` |
    /// | Desktop expanded | `"expanded"` |
    /// | Desktop `Full` collapsed | `"collapsed"` |
    /// | Desktop `Rail` (always) | `"rail"` |
    /// | Desktop `Expandable` collapsed | `"rail"` |
    pub fn sidebar_state(&self) -> &'static str {
        if self.is_mobile() {
            if (self.sidebar_mobile_open)() {
                "open"
            } else {
                "closed"
            }
        } else {
            match (self.desktop_sidebar)() {
                DesktopSidebar::Rail => "rail",
                DesktopSidebar::Full => {
                    if (self.sidebar_visible)() {
                        "expanded"
                    } else {
                        "collapsed"
                    }
                }
                DesktopSidebar::Expandable => {
                    if (self.sidebar_visible)() {
                        "expanded"
                    } else {
                        "rail"
                    }
                }
            }
        }
    }

    // ── Stack navigation ──────────────────────────────────────────────────────

    /// Pushes a new screen onto the stack (increments depth by 1).
    pub fn push_stack(&self) {
        let mut s = self.stack_depth;
        s.set((self.stack_depth)() + 1);
    }

    /// Pops the top screen from the stack (decrements depth by 1, minimum 1).
    pub fn pop_stack(&self) {
        let d = (self.stack_depth)();
        if d > 1 {
            let mut s = self.stack_depth;
            s.set(d - 1);
        }
    }

    /// Resets the stack to the root screen (depth 1).
    pub fn reset_stack(&self) {
        let mut s = self.stack_depth;
        s.set(1);
    }

    /// `true` when there is at least one screen above the root to pop back to.
    pub fn can_go_back(&self) -> bool {
        (self.stack_depth)() > 1
    }

    // ── Full-screen modal ─────────────────────────────────────────────────────

    /// Presents the full-screen modal.
    pub fn open_modal(&self) {
        let mut m = self.modal_open;
        m.set(true);
        if let Some(cb) = (self.on_modal_change)() {
            cb.call(true);
        }
    }

    /// Dismisses the full-screen modal.
    pub fn close_modal(&self) {
        let mut m = self.modal_open;
        m.set(false);
        if let Some(cb) = (self.on_modal_change)() {
            cb.call(false);
        }
    }

    /// Toggles the full-screen modal between presented and dismissed.
    pub fn toggle_modal(&self) {
        let next = !(self.modal_open)();
        let mut m = self.modal_open;
        m.set(next);
        if let Some(cb) = (self.on_modal_change)() {
            cb.call(next);
        }
    }

    // ── Search overlay ────────────────────────────────────────────────────────

    /// Activates the search overlay.
    pub fn open_search(&self) {
        let mut s = self.search_active;
        s.set(true);
        if let Some(cb) = (self.on_search_change)() {
            cb.call(true);
        }
    }

    /// Deactivates the search overlay.
    pub fn close_search(&self) {
        let mut s = self.search_active;
        s.set(false);
        if let Some(cb) = (self.on_search_change)() {
            cb.call(false);
        }
    }

    /// Toggles the search overlay between active and inactive.
    pub fn toggle_search(&self) {
        let next = !(self.search_active)();
        let mut s = self.search_active;
        s.set(next);
        if let Some(cb) = (self.on_search_change)() {
            cb.call(next);
        }
    }

    // ── Bottom sheet ──────────────────────────────────────────────────────────

    /// Sets the bottom sheet to the given snap position.
    pub fn set_sheet_snap(&self, snap: SheetSnap) {
        let mut s = self.sheet_snap;
        s.set(snap);
    }
}

/// Access [`ShellContext`] from any descendant of [`AppShell`].
///
/// # Panics
///
/// Panics if called outside an `AppShell` tree (no context provided).
pub fn use_shell_context() -> ShellContext {
    use_context::<ShellContext>()
}
