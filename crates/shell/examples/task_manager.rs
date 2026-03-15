//! task_manager — "Linear" split-pane pattern.
//!
//! Demonstrates:
//! - 3-column split pane on desktop (sidebar | issue list | preview)
//! - Stack navigation on mobile: push_stack / pop_stack / can_go_back
//! - Back button driven by `data-shell-can-go-back`
//! - Cmd+K command palette (dioxus-cmdk) with issue + project search
//! - Preview pane hidden on mobile via `data-shell-display-mode`
//!
//! Source inspiration: Linear, GitHub Desktop, Supabase dashboard.
//!
//! Run on desktop:  cargo run --example task_manager
//! Run on web:      dx serve --example task_manager

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandRoot,
    CommandSeparator, Hotkey, use_global_shortcuts,
};
use dioxus_nox_shell::{
    AppShell, BreakpointConfig, DesktopSidebar, MobileSidebar, MobileSidebarBackdrop, ShellLayout,
    use_shell_context,
};

fn main() {
    dioxus::launch(App);
}

// ── Mock data ──────────────────────────────────────────────────────────────────

static PROJECTS: &[(&str, &str)] = &[
    ("eng", "Engineering"),
    ("design", "Design"),
    ("marketing", "Marketing"),
];

/// (id, title, project_id, status)
static ISSUES: &[(&str, &str, &str, &str)] = &[
    ("ENG-1", "Add dark mode", "eng", "In Progress"),
    ("ENG-2", "Fix login redirect", "eng", "Todo"),
    ("ENG-3", "API rate limiting", "eng", "Done"),
    ("DES-1", "Update icon set", "design", "In Progress"),
    ("DES-2", "Redesign onboarding", "design", "Todo"),
    ("MKT-1", "Q1 campaign copy", "marketing", "Todo"),
];

fn project_issues(project: &str) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    ISSUES
        .iter()
        .filter(|(_, _, p, _)| *p == project)
        .copied()
        .collect()
}

fn find_issue(id: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    ISSUES.iter().copied().find(|(iid, _, _, _)| *iid == id)
}

fn status_css(s: &str) -> &'static str {
    match s {
        "In Progress" => "in-progress",
        "Done" => "done",
        _ => "todo",
    }
}

fn project_name(id: &str) -> &'static str {
    PROJECTS
        .iter()
        .find(|(pid, _)| *pid == id)
        .map(|(_, n)| *n)
        .unwrap_or("Unknown")
}

// ── Root ───────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let current_project: Signal<&'static str> = use_signal(|| "eng");
    let selected_issue: Signal<Option<&'static str>> = use_signal(|| None);

    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Horizontal,
            mobile_sidebar: MobileSidebar::Drawer,
            desktop_sidebar: DesktopSidebar::Full,
            breakpoints: BreakpointConfig { compact_below: 640.0, expanded_above: 1024.0 },
            sidebar: rsx! { NavSidebar { current_project, selected_issue } },
            preview: rsx! { IssueDetail { selected_issue } },
            footer: rsx! { StatusBar { current_project, selected_issue } },
            search: rsx! { CmdPalette { current_project, selected_issue } },
            IssueList { current_project, selected_issue }
            MobileSidebarBackdrop {}
        }
    }
}

// ── Nav Sidebar ────────────────────────────────────────────────────────────────

#[component]
fn NavSidebar(
    current_project: Signal<&'static str>,
    selected_issue: Signal<Option<&'static str>>,
) -> Element {
    let ctx = use_shell_context();
    rsx! {
        nav {
            if ctx.is_mobile() {
                div { class: "sidebar-header",
                    span { class: "sidebar-title", "TaskFlow" }
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
                div { class: "brand", "TaskFlow" }
                h2 { "Projects" }
            }
            ul {
                for &(id, name) in PROJECTS {
                    li {
                        key: "{id}",
                        class: if (current_project)() == id { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            let mut p = current_project;
                            p.set(id);
                            let mut s = selected_issue;
                            s.set(None);
                            ctx.reset_stack();
                            if ctx.is_mobile() {
                                let mut open = ctx.sidebar_mobile_open;
                                open.set(false);
                            }
                        },
                        span { class: "dot" }
                        span { class: "label", "{name}" }
                    }
                }
            }
        }
    }
}

// ── Issue List (children slot) ─────────────────────────────────────────────────

#[component]
fn IssueList(
    current_project: Signal<&'static str>,
    selected_issue: Signal<Option<&'static str>>,
) -> Element {
    let ctx = use_shell_context();
    let is_mobile = ctx.is_mobile();
    let can_back = ctx.can_go_back();

    let shortcuts = use_global_shortcuts();
    use_effect(move || {
        if let Ok(hotkey) = Hotkey::parse("ctrl+k") {
            shortcuts.register(
                "tm-search",
                hotkey,
                EventHandler::new(move |_: ()| ctx.toggle_search()),
            );
        }
    });

    let project = (current_project)();
    let issues = project_issues(project);

    rsx! {
        div { class: "content-pane",
            // Mobile: stack depth > 1 → show detail inline with back button
            if is_mobile && can_back {
                div { class: "mobile-nav",
                    button {
                        class: "back-btn",
                        onclick: move |_| {
                            ctx.pop_stack();
                            let mut s = selected_issue;
                            s.set(None);
                        },
                        "← Back"
                    }
                    if let Some(issue_id) = (selected_issue)() {
                        span { class: "mobile-nav-title", "{issue_id}" }
                    }
                }
                if let Some(issue_id) = (selected_issue)() {
                    if let Some((id, title, _, status)) = find_issue(issue_id) {
                        div { class: "mobile-detail",
                            h1 { "{title}" }
                            div { class: "issue-meta",
                                span { class: "issue-id", "{id}" }
                                span { class: "badge {status_css(status)}", "{status}" }
                            }
                            p { class: "detail-body",
                                "Issue detail view. On desktop this pane sits beside the list. "
                                "On mobile, stack navigation replaces the list with this screen."
                            }
                        }
                    }
                }
            } else {
                // List header
                div { class: "list-header",
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
                            if (ctx.sidebar_visible)() { "‹ Collapse" } else { "› Expand" }
                        }
                    }
                    h1 { "{project_name(project)}" }
                    button {
                        class: "search-trigger",
                        onclick: move |_| ctx.toggle_search(),
                        span { class: "search-icon", "⌕" }
                        span { class: "search-label", "Search issues…" }
                        kbd { "Ctrl+K" }
                    }
                }
                // Issue rows
                ul { class: "issue-list",
                    for (id, title, _, status) in issues {
                        li {
                            key: "{id}",
                            class: if (selected_issue)() == Some(id) { "issue-row selected" } else { "issue-row" },
                            onclick: move |_| {
                                let mut s = selected_issue;
                                s.set(Some(id));
                                if ctx.is_mobile() {
                                    ctx.push_stack();
                                }
                            },
                            span { class: "issue-id", "{id}" }
                            span { class: "issue-title", "{title}" }
                            span { class: "badge {status_css(status)}", "{status}" }
                        }
                    }
                }
            }
        }
    }
}

// ── Issue Detail (preview slot) ────────────────────────────────────────────────

#[component]
fn IssueDetail(selected_issue: Signal<Option<&'static str>>) -> Element {
    rsx! {
        div { class: "detail-pane",
            if let Some(issue_id) = (selected_issue)() {
                if let Some((id, title, project, status)) = find_issue(issue_id) {
                    div {
                        div { class: "detail-header",
                            span { class: "issue-id", "{id}" }
                            span { class: "badge {status_css(status)}", "{status}" }
                        }
                        h2 { "{title}" }
                        p { class: "detail-meta", "Project: {project_name(project)}" }
                        p { class: "detail-body",
                            "Issue description lives here in the preview pane. "
                            "On desktop it's always visible beside the list. "
                            "On mobile, stack navigation takes over instead."
                        }
                    }
                }
            } else {
                div { class: "detail-empty", "← Select an issue" }
            }
        }
    }
}

// ── Status Bar (footer slot) ───────────────────────────────────────────────────

#[component]
fn StatusBar(
    current_project: Signal<&'static str>,
    selected_issue: Signal<Option<&'static str>>,
) -> Element {
    let project = (current_project)();
    let count = project_issues(project).len();
    rsx! {
        span { class: "breadcrumb",
            "TaskFlow"
            span { class: "sep", " / " }
            "{project_name(project)}"
            if let Some(id) = (selected_issue)() {
                span { class: "sep", " / " }
                "{id}"
            }
        }
        span { class: "issue-count", "{count} issues" }
    }
}

// ── Command Palette (search slot) ─────────────────────────────────────────────

#[component]
fn CmdPalette(
    current_project: Signal<&'static str>,
    selected_issue: Signal<Option<&'static str>>,
) -> Element {
    let ctx = use_shell_context();
    rsx! {
        CommandDialog {
            open: ctx.search_active,
            CommandRoot {
                on_select: move |value: String| {
                    if let Some(&(id, _)) = PROJECTS.iter().find(|(id, _)| *id == value.as_str()) {
                        let mut p = current_project;
                        p.set(id);
                        let mut s = selected_issue;
                        s.set(None);
                        ctx.reset_stack();
                    } else if let Some(&(id, _, _, _)) =
                        ISSUES.iter().find(|(id, _, _, _)| *id == value.as_str())
                    {
                        let mut s = selected_issue;
                        s.set(Some(id));
                    }
                    ctx.close_search();
                },
                CommandInput { placeholder: "Search issues and projects…", autofocus: true }
                CommandList {
                    CommandEmpty { "No results." }
                    CommandGroup { id: "projects", heading: "Projects",
                        for &(id, name) in PROJECTS {
                            CommandItem {
                                key: "proj-{id}",
                                id: id,
                                label: name,
                                keywords: vec!["project".to_string()],
                                span { class: "cmd-icon", "◈" }
                                span { "{name}" }
                            }
                        }
                    }
                    CommandSeparator { group_before: "projects", group_after: "issues" }
                    CommandGroup { id: "issues", heading: "Issues",
                        for &(id, title, _, status) in ISSUES {
                            CommandItem {
                                key: "issue-{id}",
                                id: id,
                                label: title,
                                keywords: vec![id.to_lowercase(), status.to_lowercase()],
                                span { class: "cmd-dot" }
                                span { class: "cmd-main", "{title}" }
                                span { class: "cmd-sub", "{id}" }
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

/* Shell root: 3 columns (sidebar | list | preview) */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-rows: 1fr auto;
    height: 100vh;
    overflow: hidden;
}

/* Desktop sidebar */
[data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
    grid-column: 1;
    grid-row: 1;
    width: 220px;
    background: #0f0f1a;
    color: #d0d0e0;
    padding: 1.5rem 0.75rem;
    overflow-y: auto;
    transition: width 0.2s ease, padding 0.2s ease;
}
[data-shell-sidebar][data-shell-sidebar-visible="false"]:not([data-shell-sidebar-mobile]) {
    width: 0;
    padding: 0;
    overflow: hidden;
}
.brand {
    font-weight: 700;
    font-size: 1.1rem;
    color: #fff;
    padding: 0 0.75rem;
    margin-bottom: 1.5rem;
    letter-spacing: -0.02em;
}
[data-shell-sidebar] h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #555;
    padding: 0 0.75rem;
    margin-bottom: 0.5rem;
}
[data-shell-sidebar] ul { list-style: none; }
.nav-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.45rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    color: #999;
    font-size: 0.875rem;
}
.nav-item:hover { background: rgba(255,255,255,0.06); color: #ddd; }
.nav-item.active { background: rgba(255,255,255,0.12); color: #fff; }
.dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #444;
    flex-shrink: 0;
}
.nav-item.active .dot { background: #5b8fff; }

/* Mobile sidebar drawer */
[data-shell-sidebar][data-shell-sidebar-mobile] {
    position: fixed;
    inset: 0;
    z-index: 100;
    width: 260px;
    background: #0f0f1a;
    color: #d0d0e0;
    padding: 1.5rem 0.75rem;
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
    margin-bottom: 1rem;
}
.sidebar-title { font-weight: 700; color: #fff; font-size: 1rem; }
.sidebar-close {
    background: none;
    border: none;
    color: #888;
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0 0.25rem;
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

/* Main content */
[data-shell-content] {
    grid-column: 2;
    grid-row: 1;
    overflow-y: auto;
    background: #f9f9fb;
}
.content-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
}
.list-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1.25rem;
    border-bottom: 1px solid #eee;
    background: #fff;
    flex-shrink: 0;
}
.list-header h1 {
    font-size: 1rem;
    font-weight: 600;
    color: #111;
    flex: 1;
}
.sidebar-toggle {
    background: none;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    color: #666;
    flex-shrink: 0;
}
.sidebar-toggle:hover { background: #f0f0f0; }
.hamburger {
    background: none;
    border: none;
    font-size: 1.25rem;
    cursor: pointer;
    color: #333;
    flex-shrink: 0;
    line-height: 1;
}
.search-trigger {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.75rem;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: #f5f5f5;
    cursor: pointer;
    font-size: 0.8rem;
    color: #888;
    flex-shrink: 0;
}
.search-trigger:hover { border-color: #bbb; background: #eee; color: #555; }
.search-icon { font-size: 1rem; }
.search-trigger kbd {
    font-size: 0.7rem;
    font-family: monospace;
    background: #e0e0e0;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
}

/* Issue rows */
.issue-list { list-style: none; flex: 1; overflow-y: auto; }
.issue-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid #f0f0f0;
    cursor: pointer;
    font-size: 0.875rem;
    background: #fff;
}
.issue-row:hover { background: #f5f5f8; }
.issue-row.selected { background: #eef2ff; }
.issue-id {
    font-size: 0.72rem;
    color: #999;
    font-family: monospace;
    flex-shrink: 0;
    min-width: 52px;
}
.issue-title { flex: 1; color: #222; }
.badge {
    font-size: 0.68rem;
    padding: 0.15rem 0.5rem;
    border-radius: 99px;
    font-weight: 500;
    flex-shrink: 0;
}
.badge.todo { background: #f0f0f0; color: #666; }
.badge.in-progress { background: #eff6ff; color: #3b82f6; }
.badge.done { background: #f0fdf4; color: #16a34a; }

/* Mobile nav header */
.mobile-nav {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #eee;
    background: #fff;
}
.back-btn {
    background: none;
    border: none;
    font-size: 0.875rem;
    cursor: pointer;
    color: #3b82f6;
    padding: 0;
}
.mobile-nav-title { font-weight: 600; font-size: 0.9rem; color: #111; }
.mobile-detail { padding: 1.5rem; background: #fff; flex: 1; }
.mobile-detail h1 { font-size: 1.25rem; margin-bottom: 0.75rem; }
.issue-meta { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem; }
.detail-body { color: #555; line-height: 1.65; margin-top: 0.75rem; font-size: 0.9rem; }

/* Preview pane */
[data-shell-preview] {
    grid-column: 3;
    grid-row: 1;
    width: 280px;
    background: #fff;
    border-left: 1px solid #eee;
    overflow-y: auto;
}
.detail-pane { padding: 1.5rem; height: 100%; }
.detail-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.875rem; }
.detail-pane h2 { font-size: 1.05rem; font-weight: 600; margin-bottom: 0.4rem; }
.detail-meta { font-size: 0.78rem; color: #999; margin-bottom: 0.75rem; }
.detail-empty {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ccc;
    font-size: 0.9rem;
}

/* Footer */
[data-shell-footer] {
    grid-column: 1 / -1;
    grid-row: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 1rem;
    background: #0f0f1a;
    color: #888;
    font-size: 0.72rem;
    border-top: 1px solid #1f1f2e;
}
.breadcrumb { display: flex; align-items: center; }
.sep { color: #444; margin: 0 0.3rem; }
.issue-count { color: #555; }

/* Search overlay — display:contents so cmdk floats freely */
[data-shell-search] { display: contents; }
[data-cmdk-overlay] { background: rgba(0,0,0,0.45); }
[data-cmdk-dialog] {
    top: 18%;
    left: 50%;
    transform: translateX(-50%);
    width: 90%;
    max-width: 540px;
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.25);
    overflow: hidden;
    border: 1px solid #e5e5e5;
}
[data-cmdk-input] {
    width: 100%;
    padding: 1rem 1.125rem;
    border: none;
    border-bottom: 1px solid #eee;
    outline: none;
    font-size: 1rem;
}
[data-cmdk-input]::placeholder { color: #aaa; }
[data-cmdk-list] { max-height: 320px; overflow-y: auto; padding: 0.375rem 0; }
[data-cmdk-group-heading] {
    padding: 0.375rem 0.875rem;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #aaa;
}
[data-cmdk-item] {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.875rem;
    font-size: 0.875rem;
    color: #333;
    cursor: pointer;
    border-radius: 6px;
    margin: 0 0.375rem;
}
[data-cmdk-item][aria-selected="true"] { background: #f0f0f0; }
[data-cmdk-empty] { padding: 1.25rem; text-align: center; font-size: 0.875rem; color: #aaa; }
[data-cmdk-separator] { height: 1px; background: #eee; margin: 0.375rem 0; }
.cmd-icon { color: #888; flex-shrink: 0; }
.cmd-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #aaa;
    flex-shrink: 0;
}
.cmd-main { flex: 1; }
.cmd-sub { font-size: 0.72rem; color: #aaa; font-family: monospace; }

/* Mobile layout */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }
    [data-shell-content] { grid-column: 1; }
    [data-shell-preview] { display: none; }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) { display: none; }
}
"#;
