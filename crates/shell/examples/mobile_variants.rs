//! Sidebar variants showcase — desktop and mobile.
//!
//! Desktop variants (top row): Full, Rail, Expandable.
//! Mobile variants (second row): Drawer, Rail, Hidden.
//! Resize the viewport below 640 px to see mobile behaviour.
//!
//! Run on desktop:
//!   cargo run --example mobile_variants
//!
//! Run on web:
//!   dx serve --example mobile_variants

use dioxus::prelude::*;
use dioxus_nox_shell::{
    use_shell_context, AppShell, DesktopSidebar, MobileSidebar, MobileSidebarBackdrop, ShellLayout,
};

fn main() {
    dioxus::launch(App);
}

// ── Root ──────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let mut desktop = use_signal(|| DesktopSidebar::Full);
    let mut mobile = use_signal(|| MobileSidebar::Drawer);
    rsx! {
        style { {CSS} }
        div { class: "mode-picker",
            div { class: "mode-picker-row",
                span { class: "mode-picker-label", "Desktop:" }
                button {
                    class: if desktop() == DesktopSidebar::Full { "active" } else { "" },
                    onclick: move |_| desktop.set(DesktopSidebar::Full),
                    "Full"
                }
                button {
                    class: if desktop() == DesktopSidebar::Rail { "active" } else { "" },
                    onclick: move |_| desktop.set(DesktopSidebar::Rail),
                    "Rail"
                }
                button {
                    class: if desktop() == DesktopSidebar::Expandable { "active" } else { "" },
                    onclick: move |_| desktop.set(DesktopSidebar::Expandable),
                    "Expandable"
                }
            }
            div { class: "mode-picker-row",
                span { class: "mode-picker-label", "Mobile:" }
                button {
                    class: if mobile() == MobileSidebar::Drawer { "active" } else { "" },
                    onclick: move |_| mobile.set(MobileSidebar::Drawer),
                    "Drawer"
                }
                button {
                    class: if mobile() == MobileSidebar::Rail { "active" } else { "" },
                    onclick: move |_| mobile.set(MobileSidebar::Rail),
                    "Rail"
                }
                button {
                    class: if mobile() == MobileSidebar::Hidden { "active" } else { "" },
                    onclick: move |_| mobile.set(MobileSidebar::Hidden),
                    "Hidden"
                }
            }
        }
        // Shell is a wrapper so AppShell is the first (and only) node in its
        // RSX block — required by Dioxus for `key` to be valid.
        Shell { mobile: mobile(), desktop: desktop() }
    }
}

/// Thin wrapper whose sole purpose is to make `AppShell` the first node in its
/// RSX block, which is required for the `key` attribute to be valid in Dioxus.
/// The key forces a full remount when other init-only props (layout, breakpoints)
/// change. Both `mobile_sidebar` and `desktop_sidebar` are reactive on their own —
/// no remount required for variant switches.
#[component]
fn Shell(mobile: MobileSidebar, desktop: DesktopSidebar) -> Element {
    let mode_key = match mobile {
        MobileSidebar::Drawer => "drawer",
        MobileSidebar::Rail => "rail",
        MobileSidebar::Hidden => "hidden",
    };
    rsx! {
        AppShell {
            key: "{mode_key}",
            layout: ShellLayout::Horizontal,
            mobile_sidebar: mobile,
            desktop_sidebar: desktop,
            sidebar: rsx! { Sidebar {} },
            footer: rsx! { Footer { mobile, desktop } },
            Main {}
            MobileSidebarBackdrop {}
        }
    }
}

// ── Slots ─────────────────────────────────────────────────────────────────────

#[component]
fn Sidebar() -> Element {
    let ctx = use_shell_context();
    let variant = (ctx.mobile_sidebar)();
    rsx! {
        nav {
            // Header varies by context: close button for mobile drawer, h2 for desktop, nothing for rail.
            if variant == MobileSidebar::Drawer && ctx.is_mobile() {
                div { class: "sidebar-header",
                    span { class: "sidebar-title", "Navigation" }
                    button {
                        class: "sidebar-close",
                        onclick: move |_| {
                            let mut open = ctx.sidebar_mobile_open;
                            open.set(false);
                        },
                        "\u{00D7}"
                    }
                }
            } else if !ctx.is_mobile() {
                h2 { "Navigation" }
            }
            ul {
                li {
                    span { class: "icon", "⬡" }
                    span { class: "label", " Dashboard" }
                }
                li {
                    span { class: "icon", "◈" }
                    span { class: "label", " Projects" }
                }
                li {
                    span { class: "icon", "⚙" }
                    span { class: "label", " Settings" }
                }
            }
        }
    }
}

/// Main content area — explains the currently active mode.
#[component]
fn Main() -> Element {
    let ctx = use_shell_context();
    let variant = (ctx.mobile_sidebar)();
    let is_mobile = ctx.is_mobile();
    let bp = (ctx.breakpoint)();

    rsx! {
        div {
            h1 { "Mobile Sidebar Variants" }
            p { "Breakpoint: " strong { "{bp:?}" } }

            if is_mobile {
            {match variant {
                MobileSidebar::Drawer => rsx! {
                    div { class: "mode-desc",
                        h2 { "Drawer" }
                        p {
                            "A full-height overlay that slides in from the left. "
                            "Good for: apps with many nav items, content-first UX where "
                            "navigation is secondary. Tap the backdrop to close."
                        }
                        button { onclick: move |_| ctx.toggle_sidebar(), "☰  Open Drawer" }
                    }
                },
                MobileSidebar::Rail => rsx! {
                    div { class: "mode-desc",
                        h2 { "Rail" }
                        p {
                            "A narrow permanent strip always visible on the left. "
                            "Good for: apps where navigation is frequently used, "
                            "icon-based navigation, when you can't spare space "
                            "for a full sidebar but still need persistent nav."
                        }
                        p { em { "No toggle needed — the rail is always visible." } }
                    }
                },
                MobileSidebar::Hidden => rsx! {
                    div { class: "mode-desc",
                        h2 { "Hidden" }
                        p {
                            "Sidebar removed entirely. "
                            "Good for: immersive content apps, bottom tab bar navigation "
                            "(provide your own), or when sidebar nav doesn't apply on mobile."
                        }
                        div { class: "bottom-nav",
                            span { "Home" }
                            span { "Projects" }
                            span { "Settings" }
                        }
                    }
                },
            }}
        } else {
            {
                let dv = (ctx.desktop_sidebar)();
                rsx! {
                    div { class: "mode-desc",
                        {match dv {
                            DesktopSidebar::Full => rsx! {
                                h2 { "Desktop — Full Sidebar" }
                                p {
                                    "The sidebar collapses to zero width when toggled. "
                                    "Switch to Rail or Expandable to see other desktop modes."
                                }
                            },
                            DesktopSidebar::Rail => rsx! {
                                h2 { "Desktop — Rail" }
                                p {
                                    "A permanent 56 px icon-only strip. "
                                    "Labels and headings are hidden; only icons show. "
                                    "Cannot be toggled — the rail is always visible."
                                }
                            },
                            DesktopSidebar::Expandable => rsx! {
                                h2 { "Desktop — Expandable Rail" }
                                p {
                                    "Toggles between a full sidebar and a narrow icon rail. "
                                    "Collapsed = 56 px rail; expanded = full width."
                                }
                            },
                        }}
                        p { "Sidebar state: " strong { "{ctx.sidebar_state()}" } }
                        if dv != DesktopSidebar::Rail {
                            button { onclick: move |_| ctx.toggle_sidebar(), "Toggle Sidebar" }
                        }
                    }
                }
            }
        }
        }
    }
}

#[component]
fn Footer(mobile: MobileSidebar, desktop: DesktopSidebar) -> Element {
    let mobile_label = match mobile {
        MobileSidebar::Drawer => "drawer",
        MobileSidebar::Rail => "rail",
        MobileSidebar::Hidden => "hidden",
    };
    let desktop_label = match desktop {
        DesktopSidebar::Full => "full",
        DesktopSidebar::Rail => "rail",
        DesktopSidebar::Expandable => "expandable",
    };
    rsx! {
        span { "dioxus-shell v0.2 — desktop: {desktop_label} · mobile: {mobile_label}" }
    }
}

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body { height: 100%; font-family: system-ui, sans-serif; }

/* Mode picker: two rows (desktop + mobile) above the shell */
.mode-picker {
    background: #f0f0f0;
    border-bottom: 1px solid #ddd;
    padding: 0 1rem;
}
.mode-picker-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    height: 40px;
    border-bottom: 1px solid #e4e4e4;
}
.mode-picker-row:last-child { border-bottom: none; }
.mode-picker-label {
    font-size: 0.75rem;
    color: #666;
    width: 5rem;
    flex-shrink: 0;
}
.mode-picker button {
    padding: 0.3rem 0.85rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: white;
    cursor: pointer;
    font-size: 0.8rem;
}
.mode-picker button.active {
    background: #1a1a2e;
    color: white;
    border-color: #1a1a2e;
}

/* Shell root: CSS grid; height subtracts the 2-row picker (2 × 40px) */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-rows: 1fr auto;
    height: calc(100vh - 80px);
    overflow: hidden;
}

/* Desktop sidebar: always in DOM; width transitions on collapse */
[data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
    grid-column: 1;
    grid-row: 1;
    width: 220px;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 1.5rem 1rem;
    overflow-y: auto;
    transition: width 0.2s ease, padding 0.2s ease;
}

/* Full: collapse to zero */
[data-shell-desktop-variant="full"][data-shell-sidebar-visible="false"] {
    width: 0;
    padding-left: 0;
    padding-right: 0;
    overflow: hidden;
}

/* Rail: permanent 56px icon strip */
[data-shell-desktop-variant="rail"] {
    width: 56px;
    padding-left: 0;
    padding-right: 0;
    overflow: hidden;
}

/* Expandable: collapsed → rail width (overrides the generic zero-collapse rule) */
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] {
    width: 56px;
    padding-left: 0;
    padding-right: 0;
    overflow: hidden;
}

/* Rail and expandable-collapsed: icons only */
[data-shell-desktop-variant="rail"] .label,
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] .label { display: none; }
[data-shell-desktop-variant="rail"] h2,
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] h2 { display: none; }
[data-shell-desktop-variant="rail"] li,
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] li {
    justify-content: center;
    padding: 0.75rem 0;
}

/* Mobile scrim — covers content behind the drawer */
[data-shell-backdrop] {
    position: fixed;
    inset: 0;
    z-index: 99;
    background: rgba(0, 0, 0, 0.45);
}

/* Mobile overlay drawer */
[data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-variant="drawer"] {
    position: fixed;
    inset: 0;
    z-index: 100;
    width: 260px;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 1.5rem 1rem;
    overflow-y: auto;
    transform: translateX(-100%);
    transition: transform 0.25s ease;
}

/* Prevent flash-open on element insertion: drawer starts already translated off-screen */
@starting-style {
    [data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-variant="drawer"] {
        transform: translateX(-100%);
    }
}

[data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-variant="drawer"][data-shell-sidebar-state="open"] {
    transform: translateX(0);
}

/* Mobile rail: narrow static strip, always visible */
[data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-variant="rail"] {
    position: static;
    width: 56px;
    background: #1a1a2e;
    color: #e0e0e0;
    padding: 1rem 0;
    overflow: hidden;
    grid-column: 1;
    grid-row: 1;
}

/* Rail: hide text labels and section heading, center icons */
[data-shell-sidebar-variant="rail"] .label { display: none; }
[data-shell-sidebar-variant="rail"] .sidebar-header { display: none; }
[data-shell-sidebar-variant="rail"] h2 { display: none; }
[data-shell-sidebar-variant="rail"] li { justify-content: center; padding: 0.75rem 0; }

[data-shell-sidebar] h2 {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #888;
    margin-bottom: 0.75rem;
}

.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
}
.sidebar-title {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #888;
}
.sidebar-close {
    background: none;
    border: none;
    color: #aaa;
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
    padding: 0 0.25rem;
}
.sidebar-close:hover { color: #fff; }

[data-shell-sidebar] ul { list-style: none; margin-top: 0.5rem; }
[data-shell-sidebar] li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
}
[data-shell-sidebar] li:hover { background: rgba(255,255,255,0.08); }
[data-shell-sidebar] .icon { font-size: 1rem; }

[data-shell-content] {
    grid-column: 2;
    grid-row: 1;
    padding: 2rem;
    overflow-y: auto;
}

[data-shell-content] h1 { margin-bottom: 0.5rem; }
[data-shell-content] > div > p { margin-bottom: 1rem; color: #555; }
[data-shell-content] button {
    padding: 0.5rem 1.25rem;
    border: 1px solid #ccc;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.9rem;
}
[data-shell-content] button:hover { background: #f0f0f0; }

.mode-desc { margin-top: 1.5rem; }
.mode-desc h2 { margin-bottom: 0.5rem; color: #1a1a2e; }
.mode-desc p { color: #555; margin-bottom: 1rem; }

/* Bottom nav demo for Hidden mode */
.bottom-nav {
    display: flex;
    border-radius: 8px;
    background: #f0f0f0;
    overflow: hidden;
    margin-top: 1rem;
    border: 1px solid #ddd;
}
.bottom-nav span {
    flex: 1;
    text-align: center;
    padding: 0.65rem 0.5rem;
    cursor: pointer;
    font-size: 0.875rem;
    color: #333;
    border-right: 1px solid #ddd;
}
.bottom-nav span:last-child { border-right: none; }
.bottom-nav span:hover { background: #e4e4e4; }

[data-shell-footer] {
    grid-column: 1 / -1;
    grid-row: 2;
    background: #1a1a2e;
    color: #888;
    padding: 0.4rem 1rem;
    font-size: 0.75rem;
    border-top: 1px solid #2a2a4e;
}

/* ── Compact (mobile) layout ─────────────────────────────────────────────── */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }

    [data-shell-content] {
        grid-column: 1;
        padding: 1rem;
    }

    /* Hide desktop sidebar on mobile */
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
        display: none;
    }

    /* Rail: override grid to add a narrow icon column */
    [data-shell]:has([data-shell-sidebar-variant="rail"]) {
        grid-template-columns: 56px 1fr;
    }

    /* Content shifts right when rail is present */
    [data-shell]:has([data-shell-sidebar-variant="rail"]) [data-shell-content] {
        grid-column: 2;
    }
}
"#;
