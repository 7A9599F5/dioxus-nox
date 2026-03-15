//! cmdk_search — AppShell + CommandDialog integration example.
//!
//! Demonstrates the search slot: dioxus-cmdk's `CommandDialog` driven by
//! the shell's own `search_active` signal. The signal is shared directly —
//! no bridging needed. Shell close methods, cmdk Escape-key handling, and
//! backdrop clicks all write to the same signal.
//!
//! Run on desktop:
//!   cargo run --example cmdk_search
//!
//! Run on web:
//!   dx serve --example cmdk_search

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandRoot,
    CommandSeparator, Hotkey, use_global_shortcuts,
};
use dioxus_nox_shell::{AppShell, MobileSidebarBackdrop, ShellLayout, use_shell_context};

fn main() {
    dioxus::launch(App);
}

// ── Page registry ─────────────────────────────────────────────────────────────

/// (id, display title, search keywords)
static PAGES: &[(&str, &str, &str)] = &[
    ("dashboard", "Dashboard", "dash overview home"),
    ("projects", "Projects", "work tasks"),
    ("team", "Team", "people members"),
    ("analytics", "Analytics", "stats reports"),
    ("settings", "Settings", "preferences config"),
];

fn find_page(id: &str) -> (&'static str, &'static str, &'static str) {
    PAGES
        .iter()
        .find(|(pid, _, _)| *pid == id)
        .copied()
        .unwrap_or(PAGES[0])
}

// ── Root ──────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let current_page: Signal<&'static str> = use_signal(|| "dashboard");
    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Horizontal,
            sidebar: rsx! { Sidebar { current_page } },
            footer:  rsx! { Footer  { current_page } },
            search:  rsx! { SearchPalette { current_page } },
            Main { current_page }
            MobileSidebarBackdrop {}
        }
    }
}

// ── Main content ──────────────────────────────────────────────────────────────

#[component]
fn Main(current_page: Signal<&'static str>) -> Element {
    let ctx = use_shell_context();
    let is_mobile = ctx.is_mobile();

    // Register Ctrl+K global shortcut.
    // Wasm only: use_global_shortcuts installs a document keydown listener.
    // On desktop (non-wasm) the shortcuts handle is a no-op; use the button.
    let shortcuts = use_global_shortcuts();
    use_effect(move || {
        if let Ok(hotkey) = Hotkey::parse("ctrl+k") {
            shortcuts.register(
                "open-search",
                hotkey,
                EventHandler::new(move |_: ()| {
                    ctx.toggle_search();
                }),
            );
        }
    });

    let page = (current_page)();
    let (_, title, _) = find_page(page);

    rsx! {
        div {
            // Mobile: hamburger opens the drawer
            if is_mobile {
                button {
                    class: "hamburger",
                    onclick: move |_| ctx.toggle_sidebar(),
                    "☰"
                }
            } else {
                button {
                    class: "sidebar-toggle",
                    onclick: move |_| ctx.toggle_sidebar(),
                    if (ctx.sidebar_visible)() { "← Collapse" } else { "→ Expand" }
                }
            }

            h1 { "{title}" }
            p { "Select a page from the sidebar, or open the search palette." }

            // Search-bar trigger — primary way to open the palette on desktop.
            button {
                class: "search-trigger",
                onclick: move |_| ctx.toggle_search(),
                span { class: "search-icon", "⌕" }
                span { class: "search-label", "Search pages and actions…" }
                kbd { class: "search-hint", "Ctrl+K" }
            }
        }
    }
}

// ── Search palette ────────────────────────────────────────────────────────────

/// Renders inside the shell `search` slot.
///
/// `ctx.search_active` is a `Signal<bool>` passed directly to `CommandDialog { open }`.
/// This means the shell's `toggle_search / close_search` methods and cmdk's built-in
/// Escape-key / backdrop handlers all share the same signal — no bridging required.
#[component]
fn SearchPalette(current_page: Signal<&'static str>) -> Element {
    let ctx = use_shell_context();

    rsx! {
        CommandDialog {
            open: ctx.search_active,

            CommandRoot {
                // Root on_select handles page navigation.
                // Shell-action items use item-level on_select to avoid value collision.
                on_select: move |value: String| {
                    if let Some(&(id, _, _)) = PAGES.iter().find(|(id, _, _)| *id == value.as_str()) {
                        let mut p = current_page;
                        p.set(id);
                    }
                    ctx.close_search();
                },

                CommandInput { placeholder: "Search pages and actions…", autofocus: true }

                CommandList {
                    CommandEmpty { "No results." }

                    CommandGroup { id: "pages", heading: "Pages",
                        for &(id, label, kw) in PAGES {
                            CommandItem {
                                key: "{id}",
                                id: id,
                                label: label,
                                keywords: kw
                                    .split_whitespace()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>(),
                                span { "{label}" }
                            }
                        }
                    }

                    CommandSeparator { group_before: "pages", group_after: "shell" }

                    CommandGroup { id: "shell", heading: "Shell",
                        CommandItem {
                            id: "toggle-sidebar",
                            label: "Toggle Sidebar",
                            keywords: vec![
                                "sidebar".to_string(),
                                "nav".to_string(),
                                "drawer".to_string(),
                            ],
                            on_select: move |_: String| {
                                ctx.toggle_sidebar();
                                ctx.close_search();
                            },
                            span { "Toggle Sidebar" }
                        }
                        CommandItem {
                            id: "go-dashboard",
                            label: "Go to Dashboard",
                            keywords: vec![
                                "home".to_string(),
                                "start".to_string(),
                            ],
                            on_select: move |_: String| {
                                let mut p = current_page;
                                p.set("dashboard");
                                ctx.close_search();
                            },
                            span { "Go to Dashboard" }
                        }
                    }
                }
            }
        }
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

#[component]
fn Sidebar(current_page: Signal<&'static str>) -> Element {
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
                for &(id, label, _) in PAGES {
                    li {
                        key: "{id}",
                        class: if (current_page)() == id { "nav-item active" } else { "nav-item" },
                        "aria-current": if (current_page)() == id { "page" } else { "" },
                        onclick: move |_| {
                            let mut p = current_page;
                            p.set(id);
                        },
                        "{label}"
                    }
                }
            }
        }
    }
}

// ── Footer ────────────────────────────────────────────────────────────────────

#[component]
fn Footer(current_page: Signal<&'static str>) -> Element {
    let page = (current_page)();
    let (_, title, _) = find_page(page);
    rsx! {
        span { "Page: {title}" }
        span { class: "footer-sep", " · " }
        span { class: "footer-hint", "Press Ctrl+K to search" }
    }
}

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; font-family: system-ui, sans-serif; }

/* Shell root: CSS grid — footer always sticks to bottom */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: 1fr auto;
    height: 100vh;
    overflow: hidden;
}

/* Desktop sidebar */
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

/* Mobile scrim */
[data-shell-backdrop] {
    position: fixed;
    inset: 0;
    z-index: 99;
    background: rgba(0, 0, 0, 0.45);
}

/* Sidebar typography */
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

/* Nav list */
[data-shell-sidebar] ul { list-style: none; }
.nav-item {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    color: #ccc;
    font-size: 0.9rem;
}
.nav-item:hover { background: rgba(255,255,255,0.08); color: #fff; }
.nav-item.active {
    background: rgba(255,255,255,0.15);
    color: #fff;
    font-weight: 500;
}

/* Main content */
[data-shell-content] {
    grid-column: 2;
    grid-row: 1;
    padding: 2rem;
    overflow-y: auto;
}
[data-shell-content] h1 { margin-bottom: 0.5rem; font-size: 1.5rem; }
[data-shell-content] p  { margin-bottom: 1.5rem; color: #555; }

/* Sidebar toggle (desktop) */
.sidebar-toggle {
    background: none;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-bottom: 1rem;
    color: #555;
}
.sidebar-toggle:hover { background: #f0f0f0; }

/* Hamburger (mobile) */
.hamburger {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0;
    margin-bottom: 1rem;
    color: #333;
    line-height: 1;
}

/* Search trigger button — looks like a search bar */
.search-trigger {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    max-width: 420px;
    padding: 0.5rem 0.875rem;
    border: 1px solid #ddd;
    border-radius: 8px;
    background: #f8f8f8;
    cursor: pointer;
    text-align: left;
    font-size: 0.875rem;
    color: #888;
    transition: border-color 0.15s, background 0.15s;
}
.search-trigger:hover { border-color: #bbb; background: #f0f0f0; color: #555; }
.search-icon { font-size: 1.1rem; flex-shrink: 0; }
.search-label { flex: 1; }
.search-hint {
    font-size: 0.7rem;
    font-family: monospace;
    background: #eee;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    flex-shrink: 0;
    color: #666;
}

/* Footer */
[data-shell-footer] {
    grid-column: 1 / -1;
    grid-row: 2;
    background: #1a1a2e;
    color: #888;
    padding: 0.4rem 1rem;
    font-size: 0.75rem;
    border-top: 1px solid #2a2a4e;
}
.footer-sep { color: #555; }
.footer-hint { color: #666; }

/* Search slot: display:contents makes the wrapper div invisible to layout.
   CommandDialog renders fixed-position overlays that float above everything. */
[data-shell-search] { display: contents; }

/* CommandDialog overlay (backdrop) */
[data-cmdk-overlay] {
    background: rgba(0, 0, 0, 0.45);
}

/* CommandDialog container — positioned, sized, and styled */
[data-cmdk-dialog] {
    top: 18%;
    left: 50%;
    transform: translateX(-50%);
    width: 90%;
    max-width: 520px;
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.3);
    overflow: hidden;
    border: 1px solid #e5e5e5;
}

/* Search input */
[data-cmdk-input] {
    width: 100%;
    padding: 1rem 1.125rem;
    border: none;
    border-bottom: 1px solid #eee;
    outline: none;
    font-size: 1rem;
    background: transparent;
    color: #111;
}
[data-cmdk-input]::placeholder { color: #aaa; }

/* Scrollable list */
[data-cmdk-list] {
    max-height: 320px;
    overflow-y: auto;
    padding: 0.375rem 0;
}

/* Group heading */
[data-cmdk-group-heading] {
    padding: 0.375rem 0.875rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #aaa;
}

/* Items */
[data-cmdk-item] {
    display: flex;
    align-items: center;
    padding: 0.5rem 0.875rem;
    font-size: 0.9rem;
    color: #333;
    cursor: pointer;
    border-radius: 6px;
    margin: 0 0.375rem;
}
[data-cmdk-item][aria-selected="true"] {
    background: #f0f0f0;
    color: #000;
}
[data-cmdk-item][aria-disabled="true"] {
    opacity: 0.4;
    cursor: default;
}

/* Empty state */
[data-cmdk-empty] {
    padding: 1.25rem;
    text-align: center;
    font-size: 0.875rem;
    color: #aaa;
}

/* Separator */
[data-cmdk-separator] {
    height: 1px;
    background: #eee;
    margin: 0.375rem 0;
}

/* Compact (mobile) layout */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }
    [data-shell-content] {
        grid-column: 1;
        padding: 1rem;
    }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
        display: none;
    }
}
"#;
