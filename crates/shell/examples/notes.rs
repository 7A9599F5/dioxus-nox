//! notes — "Notion/Obsidian" pattern.
//!
//! Demonstrates:
//! - Nested note tree sidebar with clickable items
//! - Full-screen search modal (modal slot, no dioxus-cmdk)
//! - `data-shell-modal-state: "presented"/"dismissed"` driving CSS opacity + pointer-events
//! - Breadcrumb footer tracking the open note path
//! - `ctx.open_modal()` / `ctx.close_modal()` / `ctx.toggle_modal()` API
//!
//! Source inspiration: Notion, Obsidian, Bear.
//!
//! Run on desktop:  cargo run --example notes
//! Run on web:      dx serve --example notes

use dioxus::prelude::*;
use dioxus_core::use_drop;
use dioxus_nox_shell::{
    AppShell, BreakpointConfig, DesktopSidebar, MobileSidebar, MobileSidebarBackdrop, ShellLayout,
    use_shell_context,
};

fn main() {
    dioxus::launch(App);
}

// ── Mock data ──────────────────────────────────────────────────────────────────

/// (id, title, parent_id — empty string means root)
static NOTES: &[(&str, &str, &str)] = &[
    ("getting-started", "Getting Started", ""),
    ("installation", "Installation", "getting-started"),
    ("configuration", "Configuration", "getting-started"),
    ("daily", "Daily Notes", ""),
    ("2026-02-26", "Feb 26, 2026", "daily"),
    ("2026-02-25", "Feb 25, 2026", "daily"),
    ("projects", "Projects", ""),
    ("dioxus-shell", "dioxus-shell", "projects"),
    ("dioxus-cmdk", "dioxus-cmdk", "projects"),
    ("snippets", "Snippets", ""),
];

/// Sample body text for each note (keyed by id prefix)
fn note_body(id: &str) -> &'static str {
    match id {
        "getting-started" => {
            "Welcome to your note-taking workspace.\n\nThis note is your starting point. Explore the sidebar to navigate between notes.\n\nPress Ctrl+K (or the Search button) to open full-screen search."
        }
        "installation" => {
            "## Installation\n\nAdd dioxus-shell to your Cargo.toml:\n\n```toml\n[dependencies]\ndioxus-shell = \"0.1\"\n```\n\nThen wrap your app with `AppShell`."
        }
        "configuration" => {
            "## Configuration\n\nConfigure AppShell with props:\n\n- `layout` — Horizontal, Vertical, or Sidebar\n- `mobile_sidebar` — Drawer, Rail, or Hidden\n- `desktop_sidebar` — Full, Rail, or Expandable"
        }
        "dioxus-shell" => {
            "## dioxus-shell\n\nHeadless application shell layout primitive for Dioxus.\n\nProvides named slots: sidebar, children, preview, footer, tabs, sheet, modal, fab, search."
        }
        _ => {
            "Note content goes here.\n\nStart writing your thoughts, meeting notes, or documentation.\n\nThe editor area supports any Dioxus content."
        }
    }
}

fn note_title(id: &str) -> &'static str {
    NOTES
        .iter()
        .find(|(nid, _, _)| *nid == id)
        .map(|(_, t, _)| *t)
        .unwrap_or("Untitled")
}

fn root_notes() -> Vec<&'static (&'static str, &'static str, &'static str)> {
    NOTES
        .iter()
        .filter(|(_, _, parent)| parent.is_empty())
        .collect()
}

fn child_notes(parent: &str) -> Vec<&'static (&'static str, &'static str, &'static str)> {
    NOTES.iter().filter(|(_, _, p)| *p == parent).collect()
}

// ── Root ───────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let selected_note: Signal<Option<&'static str>> = use_signal(|| Some("getting-started"));

    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Horizontal,
            mobile_sidebar: MobileSidebar::Drawer,
            desktop_sidebar: DesktopSidebar::Full,
            breakpoints: BreakpointConfig { compact_below: 640.0, expanded_above: 1024.0 },
            sidebar: rsx! { NoteTree { selected_note } },
            modal: rsx! { SearchModal { selected_note } },
            footer: rsx! { BreadcrumbFooter { selected_note } },
            NoteContent { selected_note }
            MobileSidebarBackdrop {}
        }
    }
}

// ── Note Tree Sidebar ──────────────────────────────────────────────────────────

#[component]
fn NoteTree(selected_note: Signal<Option<&'static str>>) -> Element {
    let ctx = use_shell_context();
    rsx! {
        nav {
            if ctx.is_mobile() {
                div { class: "sidebar-header",
                    span { class: "sidebar-title", "Notes" }
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
                div { class: "brand", "Notes" }
            }

            // Search trigger button in sidebar
            button {
                class: "sidebar-search-btn",
                onclick: move |_| ctx.open_modal(),
                span { "⌕" }
                span { class: "label", " Search notes…" }
                kbd { class: "label", "Ctrl+K" }
            }

            div { class: "note-tree",
                for &&(id, title, _) in root_notes().iter() {
                    div { class: "tree-group",
                        // Root note / folder
                        button {
                            r#type: "button",
                            role: "treeitem",
                            class: if (selected_note)() == Some(id) { "tree-item root active" } else { "tree-item root" },
                            "aria-pressed": ((selected_note)() == Some(id)).to_string(),
                            onclick: move |_| {
                                let mut s = selected_note;
                                s.set(Some(id));
                                if ctx.is_mobile() {
                                    let mut open = ctx.sidebar_mobile_open;
                                    open.set(false);
                                }
                            },
                            span { class: "tree-icon",
                                if child_notes(id).is_empty() { "○" } else { "▸" }
                            }
                            span { class: "tree-label", "{title}" }
                        }
                        // Children
                        for &&(cid, ctitle, _) in child_notes(id).iter() {
                            button {
                                r#type: "button",
                                role: "treeitem",
                                class: if (selected_note)() == Some(cid) { "tree-item child active" } else { "tree-item child" },
                                "aria-pressed": ((selected_note)() == Some(cid)).to_string(),
                                onclick: move |_| {
                                    let mut s = selected_note;
                                    s.set(Some(cid));
                                    if ctx.is_mobile() {
                                        let mut open = ctx.sidebar_mobile_open;
                                        open.set(false);
                                    }
                                },
                                span { class: "tree-icon", "○" }
                                span { class: "tree-label", "{ctitle}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Note Content (children slot) ───────────────────────────────────────────────

#[component]
fn NoteContent(selected_note: Signal<Option<&'static str>>) -> Element {
    let ctx = use_shell_context();

    // Register Ctrl+K to open the modal, namespaced for cleanup.
    use_effect(move || {
        spawn(async move {
            let js = r#"
                const handler = (e) => {
                    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
                        e.preventDefault();
                        dioxus.send('open-modal');
                    } else if (e.key === 'Escape') {
                        dioxus.send('close-modal');
                    }
                };
                window.__noxNoteKeyHandler = handler;
                document.addEventListener('keydown', handler);
            "#;
            let mut eval = document::eval(js);
            while let Ok(v) = eval.recv::<String>().await {
                match v.as_str() {
                    "open-modal" => ctx.open_modal(),
                    "close-modal" => ctx.close_modal(),
                    _ => {}
                }
            }
        });
    });

    use_drop(move || {
        spawn(async move {
            let _ = document::eval(
                "if (window.__noxNoteKeyHandler) { \
                    document.removeEventListener('keydown', window.__noxNoteKeyHandler); \
                    delete window.__noxNoteKeyHandler; \
                }",
            );
        });
    });

    rsx! {
        div { class: "note-pane",
            // Mobile header
            if ctx.is_mobile() {
                div { class: "mobile-header",
                    button {
                        class: "hamburger",
                        onclick: move |_| ctx.toggle_sidebar(),
                        "☰"
                    }
                    if let Some(id) = (selected_note)() {
                        span { class: "mobile-title", "{note_title(id)}" }
                    }
                    button {
                        class: "search-btn",
                        onclick: move |_| ctx.open_modal(),
                        "⌕"
                    }
                }
            } else {
                // Desktop toolbar
                div { class: "editor-toolbar",
                    button {
                        class: "sidebar-toggle",
                        onclick: move |_| ctx.toggle_sidebar(),
                        if (ctx.sidebar_visible)() { "‹ Collapse" } else { "› Expand" }
                    }
                    div { class: "toolbar-right",
                        button {
                            class: "search-trigger",
                            onclick: move |_| ctx.open_modal(),
                            span { "⌕" }
                            span { class: "search-label", " Search…" }
                            kbd { "Ctrl+K" }
                        }
                    }
                }
            }

            // Note body
            if let Some(id) = (selected_note)() {
                div { class: "editor-content",
                    h1 { class: "note-title", "{note_title(id)}" }
                    div { class: "note-body",
                        for line in note_body(id).lines() {
                            if line.is_empty() {
                                p { class: "note-para empty-line", " " }
                            } else if let Some(heading) = line.strip_prefix("## ") {
                                h2 { class: "note-h2", "{heading}" }
                            } else if line.starts_with("```") {
                                p { class: "note-code-fence", "{line}" }
                            } else {
                                p { class: "note-para", "{line}" }
                            }
                        }
                    }
                }
            } else {
                div { class: "note-empty",
                    span { class: "empty-icon", "○" }
                    p { "Select a note from the sidebar" }
                    p { class: "empty-hint", "or press Ctrl+K to search" }
                }
            }
        }
    }
}

// ── Full-Screen Search Modal (modal slot) ──────────────────────────────────────

#[component]
fn SearchModal(selected_note: Signal<Option<&'static str>>) -> Element {
    let ctx = use_shell_context();
    let query = use_signal(String::new);
    let q = (query)().to_lowercase();

    let results: Vec<(&str, &str)> = NOTES
        .iter()
        .filter(|(_, title, _)| q.is_empty() || title.to_lowercase().contains(&q))
        .map(|(id, title, _)| (*id, *title))
        .collect();

    rsx! {
        // Backdrop
        div {
            class: "modal-backdrop",
            onclick: move |_| ctx.close_modal(),
        }
        // Modal content box
        div { class: "modal-box",
            div { class: "modal-header",
                span { class: "modal-icon", "⌕" }
                input {
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Search all notes…",
                    autofocus: true,
                    value: "{query}",
                    oninput: move |e| {
                        let mut q = query;
                        q.set(e.value());
                    },
                    onkeydown: move |e| {
                        if e.key() == Key::Escape {
                            ctx.close_modal();
                        }
                    },
                }
                button {
                    class: "modal-close",
                    onclick: move |_| ctx.close_modal(),
                    "\u{00D7}"
                }
            }
            div { class: "modal-results",
                if results.is_empty() {
                    div { class: "modal-empty", "No notes found." }
                } else {
                    for (id, title) in results {
                        button {
                            key: "{id}",
                            r#type: "button",
                            class: "modal-result-item",
                            "aria-label": "{title}",
                            onclick: move |_| {
                                let mut s = selected_note;
                                s.set(Some(id));
                                ctx.close_modal();
                                let mut q = query;
                                q.set(String::new());
                            },
                            span { class: "result-icon", "○" }
                            span { class: "result-title", "{title}" }
                        }
                    }
                }
            }
            div { class: "modal-footer",
                span { "↑↓ navigate" }
                span { class: "modal-footer-sep", " · " }
                span { "↵ open" }
                span { class: "modal-footer-sep", " · " }
                span { "Esc close" }
            }
        }
    }
}

// ── Breadcrumb Footer ──────────────────────────────────────────────────────────

#[component]
fn BreadcrumbFooter(selected_note: Signal<Option<&'static str>>) -> Element {
    if let Some(id) = (selected_note)() {
        // Find parent
        let parent_id = NOTES
            .iter()
            .find(|(nid, _, _)| *nid == id)
            .map(|(_, _, parent)| *parent)
            .unwrap_or("");

        if !parent_id.is_empty() {
            let parent_title = note_title(parent_id);
            let title = note_title(id);
            rsx! {
                span { class: "breadcrumb",
                    "Notes"
                    span { class: "sep", " / " }
                    "{parent_title}"
                    span { class: "sep", " / " }
                    span { class: "crumb-active", "{title}" }
                }
                span { class: "footer-right", "{NOTES.len()} notes" }
            }
        } else {
            let title = note_title(id);
            rsx! {
                span { class: "breadcrumb",
                    "Notes"
                    span { class: "sep", " / " }
                    span { class: "crumb-active", "{title}" }
                }
                span { class: "footer-right", "{NOTES.len()} notes" }
            }
        }
    } else {
        rsx! {
            span { class: "breadcrumb", "Notes" }
            span { class: "footer-right", "{NOTES.len()} notes" }
        }
    }
}

// ── CSS ────────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; font-family: system-ui, sans-serif; }

/* Shell grid */
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
    width: 240px;
    background: #fafafa;
    border-right: 1px solid #ebebeb;
    padding: 0;
    overflow-y: auto;
    transition: width 0.2s ease;
}
[data-shell-sidebar][data-shell-sidebar-visible="false"]:not([data-shell-sidebar-mobile]) {
    width: 0;
    overflow: hidden;
}
.brand {
    font-weight: 700;
    font-size: 0.95rem;
    color: #111;
    padding: 1rem 1rem 0.75rem;
    border-bottom: 1px solid #ebebeb;
}
.sidebar-search-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: calc(100% - 1rem);
    margin: 0.5rem;
    padding: 0.4rem 0.75rem;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.8rem;
    color: #999;
    text-align: left;
}
.sidebar-search-btn:hover { border-color: #ccc; color: #666; }
.note-tree { padding: 0.5rem 0; }
.tree-group { margin-bottom: 0.25rem; }
.tree-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.75rem;
    cursor: pointer;
    font-size: 0.85rem;
    color: #555;
    border-radius: 4px;
    margin: 0 0.25rem;
    border: none;
    background: none;
    width: calc(100% - 0.5rem);
    text-align: left;
}
.tree-item:hover { background: #f0f0f0; color: #111; }
.tree-item.active { background: #e8e8ff; color: #3b3bf6; }
.tree-item.root { font-weight: 500; }
.tree-item.child { padding-left: 1.5rem; color: #777; }
.tree-item.child.active { background: #e8e8ff; color: #3b3bf6; }
.tree-icon { font-size: 0.7rem; color: #ccc; flex-shrink: 0; }
.tree-item.active .tree-icon { color: #3b3bf6; }

/* Mobile drawer */
[data-shell-sidebar][data-shell-sidebar-mobile] {
    position: fixed;
    inset: 0;
    z-index: 100;
    width: 260px;
    background: #fafafa;
    padding: 0;
    overflow-y: auto;
    transform: translateX(-100%);
    transition: transform 0.25s ease;
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
    padding: 1rem;
    border-bottom: 1px solid #ebebeb;
}
.sidebar-title { font-weight: 700; font-size: 0.95rem; color: #111; }
.sidebar-close {
    background: none;
    border: none;
    font-size: 1.4rem;
    color: #aaa;
    cursor: pointer;
    padding: 0;
    line-height: 1;
}
.sidebar-close:hover { color: #333; }

/* Mobile backdrop */
[data-shell-backdrop] {
    position: fixed;
    inset: 0;
    z-index: 99;
    background: rgba(0,0,0,0.35);
}

/* Content area */
[data-shell-content] {
    grid-column: 2;
    grid-row: 1;
    overflow-y: auto;
    background: #fff;
}
.note-pane { height: 100%; display: flex; flex-direction: column; }
.editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1.5rem;
    border-bottom: 1px solid #f0f0f0;
    flex-shrink: 0;
}
.sidebar-toggle {
    background: none;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.78rem;
    cursor: pointer;
    color: #666;
}
.sidebar-toggle:hover { background: #f5f5f5; }
.toolbar-right { display: flex; align-items: center; gap: 0.5rem; }
.search-trigger {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.625rem;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: #f8f8f8;
    cursor: pointer;
    font-size: 0.78rem;
    color: #888;
}
.search-trigger:hover { border-color: #bbb; background: #f0f0f0; }
.search-label { color: #aaa; }
.search-trigger kbd {
    font-size: 0.68rem;
    font-family: monospace;
    background: #e0e0e0;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
}
.mobile-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f0f0f0;
    flex-shrink: 0;
}
.hamburger {
    background: none;
    border: none;
    font-size: 1.2rem;
    cursor: pointer;
    color: #555;
    padding: 0;
}
.mobile-title { flex: 1; font-weight: 600; font-size: 0.9rem; color: #111; }
.search-btn {
    background: none;
    border: none;
    font-size: 1.1rem;
    cursor: pointer;
    color: #888;
    padding: 0;
}

/* Note editor */
.editor-content { flex: 1; padding: 2rem; max-width: 720px; }
.note-title {
    font-size: 1.75rem;
    font-weight: 700;
    color: #111;
    margin-bottom: 1.5rem;
    letter-spacing: -0.02em;
}
.note-body { line-height: 1.75; }
.note-h2 { font-size: 1.1rem; font-weight: 600; color: #222; margin: 1.25rem 0 0.5rem; }
.note-para { color: #444; font-size: 0.9rem; margin-bottom: 0.5rem; }
.note-para.empty-line { margin-bottom: 0.5rem; }
.note-code-fence {
    font-family: monospace;
    font-size: 0.8rem;
    color: #888;
    background: #f5f5f5;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin: 0.5rem 0;
    white-space: pre-wrap;
}
.note-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: #ccc;
}
.empty-icon { font-size: 2rem; }
.note-empty p { font-size: 0.9rem; }
.empty-hint { font-size: 0.78rem; color: #bbb; }

/* Modal slot
   The modal div is always in the DOM; CSS controls visibility via data-shell-modal-state. */
[data-shell-modal] {
    position: fixed;
    inset: 0;
    z-index: 50;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s ease;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
}
[data-shell-modal][data-shell-modal-state="presented"] {
    opacity: 1;
    pointer-events: auto;
}
.modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.3);
    backdrop-filter: blur(4px);
    z-index: 0;
}
.modal-box {
    position: relative;
    z-index: 1;
    width: 90%;
    max-width: 560px;
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.2);
    overflow: hidden;
    border: 1px solid #e5e5e5;
}
.modal-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f0f0f0;
}
.modal-icon { color: #aaa; font-size: 1rem; flex-shrink: 0; }
.modal-input {
    flex: 1;
    border: none;
    outline: none;
    font-size: 1rem;
    color: #111;
    background: transparent;
}
.modal-input::placeholder { color: #aaa; }
.modal-close {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #aaa;
    cursor: pointer;
    padding: 0;
    line-height: 1;
}
.modal-close:hover { color: #333; }
.modal-results { max-height: 360px; overflow-y: auto; padding: 0.375rem 0; }
.modal-result-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-size: 0.875rem;
    color: #333;
    border-radius: 6px;
    margin: 0 0.375rem;
    border: none;
    background: none;
    width: calc(100% - 0.75rem);
    text-align: left;
}
.modal-result-item:hover { background: #f5f5f5; }
.result-icon { color: #ccc; font-size: 0.8rem; flex-shrink: 0; }
.result-title { flex: 1; }
.modal-empty { padding: 1.5rem; text-align: center; font-size: 0.875rem; color: #aaa; }
.modal-footer {
    display: flex;
    align-items: center;
    padding: 0.5rem 1rem;
    border-top: 1px solid #f0f0f0;
    font-size: 0.72rem;
    color: #bbb;
}
.modal-footer-sep { margin: 0 0.25rem; }

/* Footer */
[data-shell-footer] {
    grid-column: 1 / -1;
    grid-row: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 1rem;
    background: #fafafa;
    border-top: 1px solid #ebebeb;
    font-size: 0.72rem;
    color: #aaa;
}
.breadcrumb { display: flex; align-items: center; }
.sep { margin: 0 0.3rem; color: #ddd; }
.crumb-active { color: #666; font-weight: 500; }
.footer-right { color: #ccc; }

/* Mobile layout */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }
    [data-shell-content] { grid-column: 1; }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) { display: none; }
}
"#;
