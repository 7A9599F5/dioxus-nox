//! Basic dioxus-shell example.
//!
//! Run on desktop:
//!   cargo run --example basic
//!
//! Run on web:
//!   dx serve --example basic
//!
//! Run on Android / iOS:
//!   dx serve --example basic --platform android
//!   dx serve --example basic --platform ios

use dioxus::prelude::*;
use dioxus_nox_shell::{use_shell_context, AppShell, MobileSidebarBackdrop, ShellLayout};

fn main() {
    dioxus::launch(App);
}

// ── Root ──────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Horizontal,
            sidebar: rsx! { Sidebar {} },
            preview: rsx! { Preview {} },
            footer: rsx! { Footer {} },
            Main {}
            MobileSidebarBackdrop {}
        }
    }
}

// ── Slots ─────────────────────────────────────────────────────────────────────

#[component]
fn Sidebar() -> Element {
    let ctx = use_shell_context();

    rsx! {
        nav {
            if ctx.is_mobile() {
                div { class: "sidebar-header",
                    h2 { "Navigation" }
                    button {
                        class: "sidebar-close",
                        onclick: move |_| {
                            let mut open = ctx.sidebar_mobile_open;
                            open.set(false);
                        },
                        "\u{00D7}"
                    }
                }
            } else {
                h2 { "Navigation" }
            }
            ul {
                li { "Dashboard" }
                li { "Projects" }
                li { "Settings" }
            }
        }
    }
}

/// Main slot — lives inside AppShell so use_shell_context() works here.
#[component]
fn Main() -> Element {
    let ctx = use_shell_context();
    let is_mobile = ctx.is_mobile();
    let bp = (ctx.breakpoint)();

    rsx! {
        div {
            h1 { "dioxus-shell" }
            p { "A headless split-pane shell for Dioxus." }
            p {
                "Breakpoint: "
                strong { "{bp:?}" }
            }

            if is_mobile {
                button {
                    onclick: move |_| ctx.toggle_sidebar(),
                    "☰  Open Sidebar"
                }
            } else {
                p {
                    "Sidebar visible: "
                    strong { "{ctx.sidebar_visible}" }
                }
                button {
                    onclick: move |_| ctx.toggle_sidebar(),
                    "Toggle Sidebar"
                }
            }
        }
    }
}

#[component]
fn Preview() -> Element {
    rsx! {
        div {
            h3 { "Preview" }
            p { "Detail panel content goes here." }
        }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        span { "dioxus-shell v0.2.0 — data-shell-layout: horizontal" }
    }
}

// ── CSS (targets data-shell* attributes — exactly how consumers use the lib) ──
//
// Tailwind v4 theme integration example (add to your own stylesheet):
//   @import "tailwindcss";
//   @theme {
//     --color-shell-bg: oklch(0.98 0.01 250);
//     --color-shell-fg: oklch(0.24 0.01 250);
//     --color-shell-sidebar: oklch(0.15 0.02 260);
//   }
//   [data-shell] { background: var(--color-shell-bg); color: var(--color-shell-fg); }
//   [data-shell-sidebar] { background: var(--color-shell-sidebar); }

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body { height: 100%; font-family: system-ui, sans-serif; }

/* Shell root: CSS grid so footer always sticks to the bottom */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-rows: 1fr auto;
    height: 100vh;
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

[data-shell-sidebar][data-shell-sidebar-visible="false"]:not([data-shell-sidebar-mobile]) {
    width: 0;
    padding-left: 0;
    padding-right: 0;
    overflow: hidden;
}

/* Mobile scrim — covers content behind the drawer; click to close */
[data-shell-backdrop] {
    position: fixed;
    inset: 0;
    z-index: 99;
    background: rgba(0, 0, 0, 0.45);
}

/* Mobile overlay drawer */
[data-shell-sidebar][data-shell-sidebar-mobile] {
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

[data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-state="open"] {
    transform: translateX(0);
}

[data-shell-sidebar] h2 {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #888;
    margin-bottom: 0.75rem;
}

/* Mobile drawer header: label + close button */
.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
}
.sidebar-header h2 { margin-bottom: 0; }
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

[data-shell-sidebar] ul { list-style: none; }
[data-shell-sidebar] li {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
}
[data-shell-sidebar] li:hover { background: rgba(255,255,255,0.08); }

[data-shell-content] {
    grid-column: 2;
    grid-row: 1;
    padding: 2rem;
    overflow-y: auto;
}

[data-shell-content] h1 { margin-bottom: 0.5rem; }
[data-shell-content] p  { margin-bottom: 1rem; color: #555; }
[data-shell-content] button {
    padding: 0.5rem 1.25rem;
    border: 1px solid #ccc;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.9rem;
}
[data-shell-content] button:hover { background: #f0f0f0; }

[data-shell-preview] {
    grid-column: 3;
    grid-row: 1;
    width: 260px;
    background: #f7f7f7;
    border-left: 1px solid #e0e0e0;
    padding: 1.5rem 1rem;
    overflow-y: auto;
}

[data-shell-preview] h3 { margin-bottom: 0.5rem; }
[data-shell-preview] p  { color: #666; font-size: 0.9rem; }

[data-shell-footer] {
    grid-column: 1 / -1;
    grid-row: 2;
    background: #1a1a2e;
    color: #888;
    padding: 0.4rem 1rem;
    font-size: 0.75rem;
    border-top: 1px solid #2a2a4e;
}

/* Compact (mobile) layout: stack everything */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }

    [data-shell-content] {
        grid-column: 1;
        padding: 1rem;
    }

    [data-shell-preview] {
        display: none;
    }

    /* Hide desktop sidebar on mobile — Dioxus swaps trees but CSS is instant */
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
        display: none;
    }
}
"#;
