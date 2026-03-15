use crate::breakpoint::{
    BreakpointConfig, DesktopSidebar, MobileSidebar, SheetSnap, ShellBreakpoint,
    use_shell_breakpoint,
};
use crate::{ShellContext, use_shell_context};
use dioxus::prelude::*;
use std::fmt;

/// Selects the CSS layout mode applied via `data-shell-layout`.
///
/// The value is surfaced as a data attribute so consumers can write
/// CSS selectors like `[data-shell-layout="horizontal"] { … }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShellLayout {
    /// Side-by-side panes (default). Sidebar | Main | Preview.
    #[default]
    Horizontal,
    /// Stacked panes. Main above, preview below.
    Vertical,
    /// Dedicated sidebar-first layout.
    Sidebar,
}

impl ShellLayout {
    /// Returns the lowercase string used as the `data-shell-layout` value.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Sidebar => "sidebar",
        }
    }
}

impl fmt::Display for ShellLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_data_attr())
    }
}

// ── Private signal bundle ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct ShellSignals {
    layout: Signal<ShellLayout>,
    sidebar_visible: Signal<bool>,
    sidebar_mobile_open: Signal<bool>,
    mobile_sidebar: Signal<MobileSidebar>,
    desktop_sidebar: Signal<DesktopSidebar>,
    stack_depth: Signal<u32>,
    modal_open: Signal<bool>,
    search_active: Signal<bool>,
    sheet_snap: Signal<SheetSnap>,
    on_modal_change: Signal<Option<EventHandler<bool>>>,
    on_search_change: Signal<Option<EventHandler<bool>>>,
}

/// Composite hook — initialises all internal signals for `AppShell`.
/// Must be called unconditionally from the component body.
fn use_shell_signals(
    layout: ShellLayout,
    mobile_sidebar: MobileSidebar,
    desktop_sidebar: DesktopSidebar,
) -> ShellSignals {
    ShellSignals {
        layout: use_signal(|| layout),
        sidebar_visible: use_signal(|| true),
        sidebar_mobile_open: use_signal(|| false),
        mobile_sidebar: use_signal(|| mobile_sidebar),
        desktop_sidebar: use_signal(|| desktop_sidebar),
        stack_depth: use_signal(|| 1u32),
        modal_open: use_signal(|| false),
        search_active: use_signal(|| false),
        sheet_snap: use_signal(|| SheetSnap::Hidden),
        on_modal_change: use_signal(|| None),
        on_search_change: use_signal(|| None),
    }
}

// ── AppShell component ────────────────────────────────────────────────────────

/// Persistent application shell with named slots and responsive breakpoint awareness.
///
/// Provides [`ShellContext`] to all descendants via `use_context_provider`.
/// All layout is CSS-driven through `data-shell*` attributes — zero inline styles.
///
/// On compact (mobile) viewports, the sidebar switches to a separate mobile tree
/// controlled by the `mobile_sidebar` variant. On desktop, the sidebar stays in
/// the DOM and CSS controls visibility via `data-shell-sidebar-visible`.
///
/// # Focus traps
///
/// `AppShell` does **not** manage focus traps. Consumers are responsible for
/// implementing keyboard focus management (e.g., a focus-lock equivalent) within
/// modal and search slot content.
///
/// # Example
///
/// ```rust,ignore
/// AppShell {
///     sidebar: rsx! { MySidebar {} },
///     AppContent {}
/// }
/// ```
#[component]
pub fn AppShell(
    /// The main content slot (always rendered).
    children: Element,
    /// Optional sidebar slot.
    #[props(default)]
    sidebar: Option<Element>,
    /// Optional preview/detail pane slot.
    #[props(default)]
    preview: Option<Element>,
    /// Optional footer slot.
    #[props(default)]
    footer: Option<Element>,
    /// Initial layout mode. Defaults to [`ShellLayout::Horizontal`].
    #[props(default)]
    layout: ShellLayout,
    /// Mobile sidebar variant. Defaults to [`MobileSidebar::Drawer`].
    #[props(default)]
    mobile_sidebar: MobileSidebar,
    /// Desktop sidebar variant. Defaults to [`DesktopSidebar::Full`].
    #[props(default)]
    desktop_sidebar: DesktopSidebar,
    /// Breakpoint thresholds for compact/expanded detection. Defaults to
    /// `{ compact_below: 640.0, expanded_above: 1024.0 }`.
    #[props(default)]
    breakpoints: BreakpointConfig,
    /// CSP-safe override: supply an external breakpoint signal (e.g., from SSR
    /// or a `matchMedia` wrapper) instead of running the built-in eval.
    /// When provided, the built-in JS eval still runs but its output is ignored.
    #[props(default)]
    external_breakpoint: Option<ReadSignal<ShellBreakpoint>>,
    /// Extra CSS classes applied to the root element.
    #[props(default)]
    class: Option<String>,
    /// ARIA role for the sidebar region. Defaults to `"complementary"`.
    #[props(into, default = "complementary".to_string())]
    sidebar_role: String,
    /// `aria-label` for the preview region. Defaults to `"Preview"`.
    #[props(into, default = "Preview".to_string())]
    preview_label: String,
    /// Optional bottom tab bar slot. Renders as `[data-shell-tabs]`.
    #[props(default)]
    tabs: Option<Element>,
    /// Optional persistent bottom sheet slot. Renders as `[data-shell-sheet]`.
    #[props(default)]
    sheet: Option<Element>,
    /// Optional full-screen modal slot. Renders as `[data-shell-modal][role="dialog"]`.
    #[props(default)]
    modal: Option<Element>,
    /// Optional floating action button slot. Renders as `[data-shell-fab]`.
    #[props(default)]
    fab: Option<Element>,
    /// Optional search overlay slot. Renders as `[data-shell-search]`.
    #[props(default)]
    search: Option<Element>,
    /// Controlled modal state. When supplied, `AppShell` keeps its internal
    /// `modal_open` signal in sync with this signal on every render.
    #[props(default)]
    modal_open: Option<ReadSignal<bool>>,
    /// Callback fired when the shell changes modal open state.
    #[props(default)]
    on_modal_change: Option<EventHandler<bool>>,
    /// Controlled search-active state. When supplied, `AppShell` keeps its
    /// internal `search_active` signal in sync with this signal on every render.
    #[props(default)]
    search_active: Option<ReadSignal<bool>>,
    /// Callback fired when the shell changes search active state.
    #[props(default)]
    on_search_change: Option<EventHandler<bool>>,
    /// Additional HTML attributes spread onto the root `<div>`.
    /// Useful for `data-testid`, custom ARIA annotations, etc.
    #[props(default)]
    additional_attributes: Vec<Attribute>,
) -> Element {
    let signals = use_shell_signals(layout, mobile_sidebar, desktop_sidebar);

    let runtime_bp = use_shell_breakpoint(breakpoints.compact_below, breakpoints.expanded_above);
    let breakpoint = external_breakpoint.unwrap_or(runtime_bp);

    // Sync mutable enum props → signals on every render.
    // `peek()` reads without subscribing (no reactive dep → no re-render loop).
    if *signals.mobile_sidebar.peek() != mobile_sidebar {
        let mut s = signals.mobile_sidebar;
        s.set(mobile_sidebar);
    }
    if *signals.desktop_sidebar.peek() != desktop_sidebar {
        let mut s = signals.desktop_sidebar;
        s.set(desktop_sidebar);
    }

    // Sync controlled props → internal signals.
    use_effect(move || {
        if let Some(controlled) = modal_open {
            let mut s = signals.modal_open;
            s.set(controlled());
        }
    });
    use_effect(move || {
        if let Some(controlled) = search_active {
            let mut s = signals.search_active;
            s.set(controlled());
        }
    });

    // Store callbacks in signals so ShellContext methods can fire them.
    // Set unconditionally — EventHandler is not PartialEq.
    // Safe: AppShell never reads these signals reactively, so no re-render loop.
    {
        let mut s = signals.on_modal_change;
        s.set(on_modal_change);
    }
    {
        let mut s = signals.on_search_change;
        s.set(on_search_change);
    }

    let ctx = use_context_provider(|| ShellContext {
        layout: signals.layout,
        breakpoint,
        sidebar_visible: signals.sidebar_visible,
        sidebar_mobile_open: signals.sidebar_mobile_open,
        mobile_sidebar: signals.mobile_sidebar.into(),
        desktop_sidebar: signals.desktop_sidebar.into(),
        stack_depth: signals.stack_depth,
        modal_open: signals.modal_open,
        search_active: signals.search_active,
        sheet_snap: signals.sheet_snap,
        on_modal_change: signals.on_modal_change,
        on_search_change: signals.on_search_change,
    });

    let is_mobile = (breakpoint)().is_compact();
    let mobile_sidebar_val = mobile_sidebar;
    let desktop_sidebar_val = desktop_sidebar;

    let has_sidebar = sidebar.is_some();
    let sidebar_for_desktop = sidebar.clone();

    let derived_columns: &'static str = if is_mobile || !has_sidebar || !(signals.sidebar_visible)()
    {
        "1"
    } else {
        "2"
    };

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-shell": "",
            "data-shell-layout": (ctx.layout)().as_data_attr(),
            "data-shell-breakpoint": (breakpoint)().as_str(),
            "data-shell-sidebar-state": ctx.sidebar_state(),
            "data-shell-columns": derived_columns,
            "data-shell-display-mode": if is_mobile { "stack" } else { "side-by-side" },
            "data-shell-stack-depth": (signals.stack_depth)().to_string(),
            "data-shell-can-go-back": ((signals.stack_depth)() > 1).to_string(),
            "data-shell-search-active": (signals.search_active)().to_string(),
            "data-shell-modal-state": if (signals.modal_open)() { "presented" } else { "dismissed" },
            ..additional_attributes,

            // Desktop sidebar: always in DOM when present; CSS controls width
            // transitions via data-shell-sidebar-visible and data-shell-desktop-variant.
            if sidebar_for_desktop.is_some() && !is_mobile {
                div {
                    role: sidebar_role.as_str(),
                    "data-shell-sidebar": "",
                    "data-shell-sidebar-visible": (signals.sidebar_visible)().to_string(),
                    "data-shell-desktop-variant": match desktop_sidebar_val {
                        DesktopSidebar::Full       => "full",
                        DesktopSidebar::Rail       => "rail",
                        DesktopSidebar::Expandable => "expandable",
                    },
                    {sidebar_for_desktop}
                }
            }

            // Mobile sidebar: tree switches based on MobileSidebar variant.
            if sidebar.is_some() && is_mobile && mobile_sidebar_val != MobileSidebar::Hidden {
                div {
                    "data-shell-sidebar": "",
                    "data-shell-sidebar-mobile": "true",
                    "data-shell-sidebar-variant": match mobile_sidebar_val {
                        MobileSidebar::Drawer => "drawer",
                        MobileSidebar::Rail   => "rail",
                        MobileSidebar::Hidden => "hidden",
                    },
                    "data-shell-sidebar-state": if (signals.sidebar_mobile_open)() { "open" } else { "closed" },
                    {sidebar}
                }
            }

            div {
                role: "main",
                "data-shell-content": "",
                {children}
            }

            if let Some(preview_el) = preview {
                div {
                    role: "region",
                    "aria-label": preview_label.as_str(),
                    "data-shell-preview": "",
                    {preview_el}
                }
            }

            if let Some(footer_el) = footer {
                div {
                    role: "contentinfo",
                    "data-shell-footer": "",
                    {footer_el}
                }
            }

            // Bottom tab bar — persistent bottom navigation.
            if let Some(tabs_el) = tabs {
                div {
                    role: "navigation",
                    "data-shell-tabs": "",
                    {tabs_el}
                }
            }

            // Persistent bottom sheet — snap-point driven overlay.
            if let Some(sheet_el) = sheet {
                div {
                    role: "complementary",
                    "data-shell-sheet": "",
                    "data-shell-sheet-state": (signals.sheet_snap)().as_str(),
                    {sheet_el}
                }
            }

            // Floating action button.
            if let Some(fab_el) = fab {
                div {
                    "data-shell-fab": "",
                    {fab_el}
                }
            }

            // Search overlay.
            if let Some(search_el) = search {
                div {
                    role: "search",
                    "data-shell-search": "",
                    "data-shell-search-active": (signals.search_active)().to_string(),
                    {search_el}
                }
            }

            // Full-screen modal — top layer, rendered last so it sits above all other regions.
            if let Some(modal_el) = modal {
                div {
                    role: "dialog",
                    "aria-modal": "true",
                    "data-shell-modal": "",
                    "data-shell-modal-state": if (signals.modal_open)() { "presented" } else { "dismissed" },
                    {modal_el}
                }
            }
        }
    }
}

// ── MobileSidebarBackdrop ─────────────────────────────────────────────────────

/// Scrim rendered behind the mobile sidebar drawer.
///
/// Place inside `AppShell` (requires [`ShellContext`]). Renders only when
/// [`ShellContext::is_mobile`] is `true` and `sidebar_mobile_open` is open.
/// Clicking closes the drawer.
///
/// Style via `[data-shell-backdrop]` in your CSS.
///
/// This is the preferred pattern (following Radix/Headless UI/Vaul precedent)
/// over a hard-coded backdrop inside `AppShell`.
#[component]
pub fn MobileSidebarBackdrop(
    /// Extra CSS classes applied to the backdrop element.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx = use_shell_context();
    if ctx.is_mobile() && (ctx.sidebar_mobile_open)() {
        rsx! {
            div {
                "data-shell-backdrop": "",
                class: class.unwrap_or_default(),
                onclick: move |_| {
                    let mut open = ctx.sidebar_mobile_open;
                    open.set(false);
                },
            }
        }
    } else {
        rsx! {}
    }
}
