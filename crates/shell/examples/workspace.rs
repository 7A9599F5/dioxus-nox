//! workspace — "VS Code" pattern.
//!
//! Demonstrates:
//! - DesktopSidebar::Expandable: rail (48px icons) ↔ full sidebar (220px) via toggle
//! - `data-shell-sidebar-state: "rail"/"expanded"` driving CSS width transitions
//! - File tabs at top of the content column (tabs slot, explicit CSS grid placement)
//! - Mock editor content area with active-file tracking
//! - VS Code-style status bar (footer): branch, errors, line/col info
//! - Cmd+K quick-open palette (dioxus-cmdk) with keyboard shortcut hints
//! - Mobile: Drawer sidebar; tabs scroll horizontally
//!
//! Source inspiration: VS Code, Figma, Linear.
//!
//! Run on desktop:  cargo run --example workspace
//! Run on web:      dx serve --example workspace

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList,
    CommandRoot, Hotkey, use_global_shortcuts,
};
use dioxus_nox_shell::{AppShell, BreakpointConfig, DesktopSidebar, MobileSidebar, MobileSidebarBackdrop, ShellLayout, use_shell_context};

fn main() {
    dioxus::launch(App);
}

// ── Mock data ──────────────────────────────────────────────────────────────────

/// (id, display name, language, shortcut hint)
static FILES: &[(&str, &str, &str, &str)] = &[
    ("main", "main.rs", "Rust", "Ctrl+P"),
    ("shell", "shell.rs", "Rust", ""),
    ("context", "context.rs", "Rust", ""),
    ("breakpoint", "breakpoint.rs", "Rust", ""),
    ("cargo", "Cargo.toml", "TOML", ""),
    ("readme", "README.md", "Markdown", ""),
];

static NAV_SECTIONS: &[(&str, &str)] = &[
    ("explorer", "Explorer"),
    ("search", "Search"),
    ("git", "Source Control"),
    ("run", "Run & Debug"),
    ("extensions", "Extensions"),
];

static NAV_ICONS: &[&str] = &["⊞", "⌕", "⎇", "▶", "⊡"];

// ── Root ───────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    // Indices into FILES for open tabs
    let open_files: Signal<Vec<usize>> = use_signal(|| vec![0, 1, 2]);
    let active_file: Signal<Option<usize>> = use_signal(|| Some(0));
    let active_nav: Signal<&'static str> = use_signal(|| "explorer");

    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Sidebar,
            mobile_sidebar: MobileSidebar::Drawer,
            desktop_sidebar: DesktopSidebar::Expandable,
            breakpoints: BreakpointConfig { compact_below: 640.0, expanded_above: 1024.0 },
            sidebar: rsx! { ActivityBar { active_nav, open_files, active_file } },
            tabs: rsx! { FileTabs { open_files, active_file } },
            footer: rsx! { StatusBar { active_file } },
            search: rsx! { QuickOpen { open_files, active_file } },
            EditorArea { active_file }
            MobileSidebarBackdrop {}
        }
    }
}

// ── Activity Bar / Sidebar ─────────────────────────────────────────────────────

#[component]
fn ActivityBar(
    active_nav: Signal<&'static str>,
    open_files: Signal<Vec<usize>>,
    active_file: Signal<Option<usize>>,
) -> Element {
    let ctx = use_shell_context();
    let state = ctx.sidebar_state();

    rsx! {
        div { class: "activity-bar",
            // Mobile: drawer header
            if ctx.is_mobile() {
                div { class: "sidebar-header",
                    span { class: "sidebar-title", "Explorer" }
                    button {
                        class: "sidebar-close",
                        onclick: move |_| {
                            let mut open = ctx.sidebar_mobile_open;
                            open.set(false);
                        },
                        "\u{00D7}"
                    }
                }
            }

            // Icon strip (always visible)
            div { class: "activity-icons",
                for (&(id, _), &icon) in NAV_SECTIONS.iter().zip(NAV_ICONS.iter()) {
                    button {
                        key: "{id}",
                        class: if (active_nav)() == id { "activity-icon active" } else { "activity-icon" },
                        title: id,
                        onclick: move |_| {
                            let mut nav = active_nav;
                            nav.set(id);
                            // If collapsed on desktop, expand
                            if !ctx.is_mobile() && state == "rail" {
                                ctx.toggle_sidebar();
                            }
                        },
                        "{icon}"
                    }
                }
            }

            // Panel (visible when expanded)
            div { class: "sidebar-panel",
                h2 { "Explorer" }
                ul { class: "file-tree",
                    for (i, &(id, name, lang, _)) in FILES.iter().enumerate() {
                        li {
                            key: "{id}",
                            class: if (active_file)() == Some(i) { "file-item active" } else { "file-item" },
                            onclick: move |_| {
                                // Open file tab if not already open
                                let mut of = open_files;
                                if !of().contains(&i) {
                                    of.write().push(i);
                                }
                                let mut af = active_file;
                                af.set(Some(i));
                                if ctx.is_mobile() {
                                    let mut open = ctx.sidebar_mobile_open;
                                    open.set(false);
                                }
                            },
                            span { class: "file-lang-dot lang-{lang.to_lowercase()}" }
                            span { class: "label", "{name}" }
                        }
                    }
                }
            }
        }
    }
}

// ── File Tabs (tabs slot) ──────────────────────────────────────────────────────

#[component]
fn FileTabs(open_files: Signal<Vec<usize>>, active_file: Signal<Option<usize>>) -> Element {
    let ctx = use_shell_context();
    rsx! {
        div { class: "file-tabs-bar",
            if ctx.is_mobile() {
                button {
                    class: "hamburger",
                    onclick: move |_| ctx.toggle_sidebar(),
                    "☰"
                }
            }
            div { class: "tabs-scroll",
                for i in (open_files)() {
                    if let Some(&(_, name, _, _)) = FILES.get(i) {
                        div {
                            key: "{i}",
                            class: if (active_file)() == Some(i) { "file-tab active" } else { "file-tab" },
                            onclick: move |_| {
                                let mut af = active_file;
                                af.set(Some(i));
                            },
                            span { class: "tab-name", "{name}" }
                            button {
                                class: "tab-close",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    let mut of = open_files;
                                    of.write().retain(|&x| x != i);
                                    // If we closed the active tab, pick adjacent
                                    let mut af = active_file;
                                    if af() == Some(i) {
                                        let remaining = of();
                                        af.set(remaining.last().copied());
                                    }
                                },
                                "\u{00D7}"
                            }
                        }
                    }
                }
            }
            // Quick-open button on the right
            button {
                class: "quick-open-btn",
                onclick: move |_| ctx.toggle_search(),
                "⌕  Ctrl+P"
            }
        }
    }
}

// ── Editor Area (children slot) ───────────────────────────────────────────────

#[component]
fn EditorArea(active_file: Signal<Option<usize>>) -> Element {
    let ctx = use_shell_context();

    let shortcuts = use_global_shortcuts();
    use_effect(move || {
        if let Ok(hotkey) = Hotkey::parse("ctrl+p") {
            shortcuts.register(
                "ws-quick-open",
                hotkey,
                EventHandler::new(move |_: ()| ctx.toggle_search()),
            );
        }
        if let Ok(hotkey) = Hotkey::parse("ctrl+b") {
            shortcuts.register(
                "ws-toggle-sidebar",
                hotkey,
                EventHandler::new(move |_: ()| ctx.toggle_sidebar()),
            );
        }
    });

    rsx! {
        div { class: "editor-area",
            if let Some(idx) = (active_file)() {
                if let Some(&(_, name, lang, _)) = FILES.get(idx) {
                    div { class: "editor-content",
                        div { class: "editor-toolbar",
                            span { class: "editor-breadcrumb", "src / {name}" }
                            span { class: "editor-lang", "{lang}" }
                        }
                        div { class: "editor-body",
                            pre { class: "code-block",
                                "// {name}\n"
                                "// Language: {lang}\n\n"
                                "// This is the editor content area.\n"
                                "// In a real app, a text editor component lives here.\n\n"
                                "// Keyboard shortcuts:\n"
                                "//   Ctrl+P    Quick-open file\n"
                                "//   Ctrl+B    Toggle sidebar\n"
                            }
                        }
                    }
                }
            } else {
                div { class: "editor-empty",
                    span { class: "editor-empty-icon", "⊞" }
                    p { "Open a file from the Explorer" }
                    p { class: "editor-empty-hint", "or press Ctrl+P to quick-open" }
                }
            }
        }
    }
}

// ── Status Bar (footer slot) ───────────────────────────────────────────────────

#[component]
fn StatusBar(active_file: Signal<Option<usize>>) -> Element {
    let name = (active_file)()
        .and_then(|i| FILES.get(i))
        .map(|(_, n, _, _)| *n)
        .unwrap_or("No file open");
    rsx! {
        div { class: "status-left",
            span { class: "status-branch", "⎇  main" }
            span { class: "status-sep", " " }
            span { class: "status-errors", "⊗ 0  ⚠ 0" }
        }
        div { class: "status-right",
            span { "{name}" }
            span { class: "status-sep", "  |  " }
            span { "Ln 1, Col 1" }
            span { class: "status-sep", "  |  " }
            span { "UTF-8" }
        }
    }
}

// ── Quick-Open Palette (search slot) ──────────────────────────────────────────

#[component]
fn QuickOpen(open_files: Signal<Vec<usize>>, active_file: Signal<Option<usize>>) -> Element {
    let ctx = use_shell_context();
    rsx! {
        CommandDialog {
            open: ctx.search_active,
            CommandRoot {
                on_select: move |value: String| {
                    if let Some(i) = FILES.iter().position(|(id, _, _, _)| *id == value.as_str()) {
                        let mut of = open_files;
                        if !of().contains(&i) {
                            of.write().push(i);
                        }
                        let mut af = active_file;
                        af.set(Some(i));
                    }
                    ctx.close_search();
                },
                CommandInput { placeholder: "Go to file…", autofocus: true }
                CommandList {
                    CommandEmpty { "No files found." }
                    CommandGroup { id: "files", heading: "Open File",
                        for (i, &(id, name, lang, hint)) in FILES.iter().enumerate() {
                            CommandItem {
                                key: "{id}",
                                id: id,
                                label: name,
                                keywords: vec![name.to_lowercase(), lang.to_lowercase()],
                                span { class: "cmd-file-dot lang-{lang.to_lowercase()}" }
                                span { class: "cmd-main", "{name}" }
                                span { class: "cmd-lang", "{lang}" }
                                if !hint.is_empty() {
                                    kbd { class: "cmd-hint", "{hint}" }
                                }
                                if (open_files)().contains(&i) {
                                    span { class: "cmd-open-dot", "●" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── CSS ────────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; font-family: system-ui, sans-serif; }

/* Shell grid: sidebar spans rows, tabs at top, editor middle, status bar bottom */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: 35px 1fr 22px;
    height: 100vh;
    overflow: hidden;
    background: #1e1e1e;
    color: #ccc;
}

/* Desktop sidebar — spans all rows */
[data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
    grid-column: 1;
    grid-row: 1 / -1;
    display: flex;
    flex-direction: row;
    background: #252526;
    overflow: hidden;
    transition: width 0.2s ease;
}

/* Expandable: rail width when collapsed */
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] {
    width: 48px;
}
/* Expandable: full width when expanded */
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="true"] {
    width: 268px;
}
/* Full: collapse to 0 */
[data-shell-desktop-variant="full"][data-shell-sidebar-visible="false"] {
    width: 0;
    overflow: hidden;
}
[data-shell-desktop-variant="full"][data-shell-sidebar-visible="true"] {
    width: 220px;
}

/* Activity bar */
.activity-bar {
    display: flex;
    flex-direction: row;
    height: 100%;
    width: 100%;
}
.activity-icons {
    display: flex;
    flex-direction: column;
    width: 48px;
    flex-shrink: 0;
    background: #333;
    padding: 4px 0;
}
.activity-icon {
    width: 48px;
    height: 48px;
    border: none;
    background: none;
    color: #888;
    font-size: 1.1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s;
}
.activity-icon:hover { color: #ccc; }
.activity-icon.active {
    color: #fff;
    border-left: 2px solid #0078d4;
}

/* Sidebar panel — hidden when rail-only (collapsed Expandable) */
.sidebar-panel {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem 0;
}
[data-shell-desktop-variant="expandable"][data-shell-sidebar-visible="false"] .sidebar-panel {
    display: none;
}

[data-shell-sidebar] h2 {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #888;
    padding: 0 0.75rem;
    margin-bottom: 0.4rem;
}
.file-tree { list-style: none; }
.file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.75rem;
    cursor: pointer;
    font-size: 0.825rem;
    color: #ccc;
    border-left: 2px solid transparent;
}
.file-item:hover { background: rgba(255,255,255,0.05); }
.file-item.active { background: rgba(255,255,255,0.08); border-left-color: #0078d4; color: #fff; }
.file-lang-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}
.lang-rust { background: #ce422b; }
.lang-toml { background: #9c4221; }
.lang-markdown { background: #4a5568; }

/* Mobile sidebar drawer */
[data-shell-sidebar][data-shell-sidebar-mobile] {
    position: fixed;
    inset: 0;
    z-index: 100;
    width: 280px;
    background: #252526;
    color: #ccc;
    padding: 1rem 0.75rem;
    overflow-y: auto;
    transform: translateX(-100%);
    transition: transform 0.25s ease;
    flex-direction: column;
}
[data-shell-sidebar][data-shell-sidebar-mobile][data-shell-sidebar-state="open"] {
    transform: translateX(0);
}
@starting-style {
    [data-shell-sidebar][data-shell-sidebar-mobile] { transform: translateX(-100%); }
}
.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
}
.sidebar-title { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: #888; }
.sidebar-close {
    background: none;
    border: none;
    color: #888;
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
}
.sidebar-close:hover { color: #fff; }

/* Mobile backdrop */
[data-shell-backdrop] {
    position: fixed;
    inset: 0;
    z-index: 99;
    background: rgba(0,0,0,0.5);
}

/* File tabs bar (tabs slot — placed at grid row 1) */
[data-shell-tabs] {
    grid-column: 2;
    grid-row: 1;
    z-index: 1;
}
.file-tabs-bar {
    display: flex;
    align-items: center;
    height: 35px;
    background: #252526;
    border-bottom: 1px solid #1e1e1e;
    overflow: hidden;
}
.tabs-scroll {
    display: flex;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
}
.tabs-scroll::-webkit-scrollbar { display: none; }
.file-tab {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0 0.875rem;
    height: 35px;
    border-right: 1px solid #1e1e1e;
    cursor: pointer;
    font-size: 0.8rem;
    color: #888;
    white-space: nowrap;
    flex-shrink: 0;
}
.file-tab:hover { background: rgba(255,255,255,0.05); color: #ccc; }
.file-tab.active { background: #1e1e1e; color: #fff; border-top: 1px solid #0078d4; }
.tab-name { font-size: 0.8rem; }
.tab-close {
    background: none;
    border: none;
    color: #666;
    font-size: 0.9rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
}
.tab-close:hover { background: rgba(255,255,255,0.12); color: #ccc; }
.file-tab.active .tab-close { color: #aaa; }
.hamburger {
    background: none;
    border: none;
    color: #888;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0 0.75rem;
    height: 35px;
    border-right: 1px solid #1e1e1e;
}
.hamburger:hover { color: #ccc; }
.quick-open-btn {
    background: none;
    border: none;
    border-left: 1px solid #1e1e1e;
    color: #666;
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0 0.75rem;
    height: 35px;
    white-space: nowrap;
}
.quick-open-btn:hover { color: #ccc; }

/* Editor (content slot — grid row 2) */
[data-shell-content] {
    grid-column: 2;
    grid-row: 2;
    overflow: hidden;
    background: #1e1e1e;
}
.editor-area {
    height: 100%;
    display: flex;
    flex-direction: column;
}
.editor-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}
.editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 1rem;
    background: #2d2d2d;
    border-bottom: 1px solid #1a1a1a;
    font-size: 0.75rem;
}
.editor-breadcrumb { color: #ccc; }
.editor-lang { color: #888; }
.editor-body { flex: 1; overflow: auto; }
.code-block {
    padding: 1.5rem;
    font-family: "Fira Code", "Consolas", monospace;
    font-size: 0.875rem;
    line-height: 1.7;
    color: #9cdcfe;
    white-space: pre-wrap;
    min-height: 100%;
}
.editor-empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: #555;
}
.editor-empty-icon { font-size: 2rem; }
.editor-empty p { font-size: 0.9rem; }
.editor-empty-hint { font-size: 0.78rem; color: #444; }

/* Status bar (footer slot — grid row 3) */
[data-shell-footer] {
    grid-column: 2;
    grid-row: 3;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.75rem;
    background: #0078d4;
    color: #fff;
    font-size: 0.72rem;
}
.status-left, .status-right { display: flex; align-items: center; gap: 0.25rem; }
.status-branch { font-weight: 500; }
.status-errors { color: rgba(255,255,255,0.8); }
.status-sep { opacity: 0.5; }

/* Search slot */
[data-shell-search] { display: contents; }
[data-cmdk-overlay] { background: rgba(0,0,0,0.6); }
[data-cmdk-dialog] {
    top: 12%;
    left: 50%;
    transform: translateX(-50%);
    width: 90%;
    max-width: 520px;
    background: #252526;
    border-radius: 8px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.5);
    overflow: hidden;
    border: 1px solid #444;
    color: #ccc;
}
[data-cmdk-input] {
    width: 100%;
    padding: 0.875rem 1rem;
    border: none;
    border-bottom: 1px solid #333;
    outline: none;
    font-size: 0.95rem;
    background: transparent;
    color: #ccc;
}
[data-cmdk-input]::placeholder { color: #666; }
[data-cmdk-list] { max-height: 340px; overflow-y: auto; padding: 0.25rem 0; }
[data-cmdk-group-heading] {
    padding: 0.375rem 0.875rem;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #666;
}
[data-cmdk-item] {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.875rem;
    font-size: 0.85rem;
    color: #ccc;
    cursor: pointer;
    border-radius: 4px;
    margin: 0 0.25rem;
}
[data-cmdk-item][aria-selected="true"] { background: #094771; color: #fff; }
[data-cmdk-empty] { padding: 1.25rem; text-align: center; font-size: 0.875rem; color: #666; }
.cmd-file-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}
.cmd-main { flex: 1; }
.cmd-lang { font-size: 0.72rem; color: #666; }
.cmd-hint {
    font-size: 0.68rem;
    font-family: monospace;
    background: #333;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    color: #aaa;
}
.cmd-open-dot { color: #0078d4; font-size: 0.6rem; }

/* Mobile layout */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 35px 1fr 22px;
    }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) { display: none; }
    [data-shell-tabs] { grid-column: 1; }
    [data-shell-content] { grid-column: 1; }
    [data-shell-footer] { grid-column: 1; }
}
"#;
