//! admin — "Vercel/Stripe Dashboard" pattern.
//!
//! Demonstrates:
//! - Collapsible sidebar sections with `[aria-expanded]` CSS-driven chevron rotation
//! - Sub-page tabs at the top of the content column (tabs slot)
//! - Content table with sortable mock rows
//! - Pagination footer with record count
//! - Global search palette (dioxus-cmdk) — jumps to sections and items
//!
//! Source inspiration: Vercel dashboard, Stripe, GitHub settings.
//!
//! Run on desktop:  cargo run --example admin
//! Run on web:      dx serve --example admin

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList,
    CommandRoot, CommandSeparator, Hotkey, use_global_shortcuts,
};
use dioxus_nox_shell::{AppShell, BreakpointConfig, DesktopSidebar, MobileSidebar, MobileSidebarBackdrop, ShellLayout, use_shell_context};

fn main() {
    dioxus::launch(App);
}

// ── Mock data ──────────────────────────────────────────────────────────────────

/// (section_id, section_label, items: &[(item_id, item_label)])
#[allow(clippy::type_complexity)]
static SECTIONS: &[(&str, &str, &[(&str, &str)])] = &[
    (
        "analytics",
        "Analytics",
        &[
            ("overview", "Overview"),
            ("performance", "Performance"),
            ("conversions", "Conversions"),
        ],
    ),
    (
        "deployments",
        "Deployments",
        &[
            ("production", "Production"),
            ("staging", "Staging"),
            ("previews", "Previews"),
        ],
    ),
    (
        "settings",
        "Settings",
        &[
            ("general", "General"),
            ("team", "Team"),
            ("billing", "Billing"),
        ],
    ),
];

static SUB_TABS: &[&str] = &["Overview", "Settings", "Logs"];

/// (id, name, status, updated)
static TABLE_ROWS: &[(&str, &str, &str, &str)] = &[
    ("d1", "web-app-prod", "Ready", "2m ago"),
    ("d2", "api-server", "Building", "5m ago"),
    ("d3", "marketing-site", "Ready", "1h ago"),
    ("d4", "admin-panel", "Error", "2h ago"),
    ("d5", "docs-site", "Ready", "1d ago"),
];

fn status_css(s: &str) -> &'static str {
    match s {
        "Ready" => "ready",
        "Building" => "building",
        "Error" => "error",
        _ => "unknown",
    }
}

// ── Root ───────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let active_section: Signal<&'static str> = use_signal(|| "analytics");
    let active_item: Signal<&'static str> = use_signal(|| "overview");
    let active_tab: Signal<&'static str> = use_signal(|| "Overview");

    // One open/closed signal per section
    let sec0_open = use_signal(|| true);
    let sec1_open = use_signal(|| false);
    let sec2_open = use_signal(|| false);

    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Horizontal,
            mobile_sidebar: MobileSidebar::Drawer,
            desktop_sidebar: DesktopSidebar::Full,
            breakpoints: BreakpointConfig { compact_below: 640.0, expanded_above: 1280.0 },
            sidebar: rsx! {
                CollapsibleNav {
                    active_section,
                    active_item,
                    sec0_open,
                    sec1_open,
                    sec2_open,
                }
            },
            tabs: rsx! { SubTabs { active_tab } },
            footer: rsx! { PaginationFooter { active_section, active_item } },
            search: rsx! { GlobalSearch { active_section, active_item, active_tab } },
            ContentTable { active_tab }
            MobileSidebarBackdrop {}
        }
    }
}

// ── Collapsible Sidebar Nav ────────────────────────────────────────────────────

#[component]
fn CollapsibleNav(
    active_section: Signal<&'static str>,
    active_item: Signal<&'static str>,
    sec0_open: Signal<bool>,
    sec1_open: Signal<bool>,
    sec2_open: Signal<bool>,
) -> Element {
    let ctx = use_shell_context();
    let section_opens: [Signal<bool>; 3] = [sec0_open, sec1_open, sec2_open];

    rsx! {
        nav {
            if ctx.is_mobile() {
                div { class: "sidebar-header",
                    span { class: "sidebar-title", "Dashboard" }
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
                div { class: "brand", "Acme Corp" }
            }

            div { class: "nav-sections",
                for (si, &(sec_id, sec_label, items)) in SECTIONS.iter().enumerate() {
                    div { class: "nav-section",
                        // Section header — clicking toggles collapse
                        button {
                            key: "sec-{sec_id}",
                            class: "section-header",
                            "aria-expanded": if (section_opens[si])() { "true" } else { "false" },
                            onclick: move |_| {
                                let mut s = section_opens[si];
                                s.set(!(section_opens[si])());
                            },
                            span { class: "section-label", "{sec_label}" }
                            span { class: "chevron", "›" }
                        }
                        // Section items — shown when open
                        div { class: "section-items",
                            for &(item_id, item_label) in items {
                                button {
                                    key: "item-{item_id}",
                                    class: if (active_item)() == item_id && (active_section)() == sec_id {
                                        "nav-item active"
                                    } else {
                                        "nav-item"
                                    },
                                    onclick: move |_| {
                                        let mut sec = active_section;
                                        sec.set(sec_id);
                                        let mut item = active_item;
                                        item.set(item_id);
                                        if ctx.is_mobile() {
                                            let mut mob = ctx.sidebar_mobile_open;
                                            mob.set(false);
                                        }
                                    },
                                    "{item_label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Sub-page Tabs (tabs slot) ──────────────────────────────────────────────────

#[component]
fn SubTabs(active_tab: Signal<&'static str>) -> Element {
    let ctx = use_shell_context();
    rsx! {
        div { class: "sub-tabs-bar",
            if ctx.is_mobile() {
                button {
                    class: "hamburger",
                    onclick: move |_| ctx.toggle_sidebar(),
                    "☰"
                }
            } else {
                button {
                    class: "sidebar-toggle",
                    onclick: move |_| ctx.toggle_sidebar(),
                    if (ctx.sidebar_visible)() { "‹" } else { "›" }
                }
            }
            div { class: "tabs-list",
                for &tab in SUB_TABS {
                    button {
                        key: "{tab}",
                        class: if (active_tab)() == tab { "sub-tab active" } else { "sub-tab" },
                        onclick: move |_| {
                            let mut t = active_tab;
                            t.set(tab);
                        },
                        "{tab}"
                    }
                }
            }
            // Global search trigger
            button {
                class: "global-search-btn",
                onclick: move |_| ctx.toggle_search(),
                "⌕  Search"
                kbd { "Ctrl+K" }
            }
        }
    }
}

// ── Content Table (children slot) ─────────────────────────────────────────────

#[component]
fn ContentTable(active_tab: Signal<&'static str>) -> Element {
    let ctx = use_shell_context();
    let tab = (active_tab)();

    let shortcuts = use_global_shortcuts();
    use_effect(move || {
        if let Ok(hotkey) = Hotkey::parse("ctrl+k") {
            shortcuts.register(
                "admin-search",
                hotkey,
                EventHandler::new(move |_: ()| ctx.toggle_search()),
            );
        }
    });

    rsx! {
        div { class: "content-pane",
            div { class: "content-header",
                h1 { "Deployments" }
                span { class: "content-tab-label", "— {tab}" }
                button { class: "new-btn", "＋ New" }
            }
            div { class: "table-wrapper",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Name" }
                            th { "Status" }
                            th { "Updated" }
                            th { "" }
                        }
                    }
                    tbody {
                        for &(id, name, status, updated) in TABLE_ROWS {
                            tr { key: "{id}",
                                td { class: "td-name",
                                    span { class: "row-icon", "◈" }
                                    "{name}"
                                }
                                td {
                                    span { class: "status-pill {status_css(status)}", "{status}" }
                                }
                                td { class: "td-muted", "{updated}" }
                                td { class: "td-actions",
                                    button { class: "row-action", "…" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Pagination Footer ──────────────────────────────────────────────────────────

#[component]
fn PaginationFooter(
    active_section: Signal<&'static str>,
    active_item: Signal<&'static str>,
) -> Element {
    let sec = (active_section)();
    let item = (active_item)();
    let sec_label = SECTIONS
        .iter()
        .find(|(id, _, _)| *id == sec)
        .map(|(_, l, _)| *l)
        .unwrap_or(sec);
    let item_label = SECTIONS
        .iter()
        .find(|(id, _, _)| *id == sec)
        .and_then(|(_, _, items)| items.iter().find(|(id, _)| *id == item))
        .map(|(_, l)| *l)
        .unwrap_or(item);

    rsx! {
        span { class: "breadcrumb",
            "Acme Corp"
            span { class: "sep", " / " }
            "{sec_label}"
            span { class: "sep", " / " }
            "{item_label}"
        }
        div { class: "pagination",
            button { class: "page-btn", "← Prev" }
            span { class: "page-info", "Page 1 of 3  ·  {TABLE_ROWS.len()} records" }
            button { class: "page-btn", "Next →" }
        }
    }
}

// ── Global Search (search slot) ───────────────────────────────────────────────

#[component]
fn GlobalSearch(
    active_section: Signal<&'static str>,
    active_item: Signal<&'static str>,
    active_tab: Signal<&'static str>,
) -> Element {
    let ctx = use_shell_context();
    rsx! {
        CommandDialog {
            open: ctx.search_active,
            CommandRoot {
                on_select: move |value: String| {
                    // Jump to section
                    for &(sec_id, _, items) in SECTIONS {
                        if sec_id == value.as_str() {
                            let mut sec = active_section;
                            sec.set(sec_id);
                            if let Some(&(item_id, _)) = items.first() {
                                let mut item = active_item;
                                item.set(item_id);
                            }
                            ctx.close_search();
                            return;
                        }
                        for &(item_id, _) in items {
                            if item_id == value.as_str() {
                                let mut sec = active_section;
                                sec.set(sec_id);
                                let mut item = active_item;
                                item.set(item_id);
                                ctx.close_search();
                                return;
                            }
                        }
                    }
                    // Switch tab
                    for &tab in SUB_TABS {
                        if tab == value.as_str() {
                            let mut t = active_tab;
                            t.set(tab);
                            ctx.close_search();
                            return;
                        }
                    }
                    ctx.close_search();
                },
                CommandInput { placeholder: "Search sections, items, actions…", autofocus: true }
                CommandList {
                    CommandEmpty { "No results." }
                    CommandGroup { id: "sections", heading: "Sections",
                        for &(sec_id, sec_label, _) in SECTIONS {
                            CommandItem {
                                key: "sec-{sec_id}",
                                id: sec_id,
                                label: sec_label,
                                keywords: vec!["section".to_string()],
                                span { class: "cmd-icon", "⊡" }
                                span { "{sec_label}" }
                            }
                        }
                    }
                    CommandSeparator { group_before: "sections", group_after: "items" }
                    CommandGroup { id: "items", heading: "Pages",
                        for &(_, _, items) in SECTIONS {
                            for &(item_id, item_label) in items {
                                CommandItem {
                                    key: "item-{item_id}",
                                    id: item_id,
                                    label: item_label,
                                    keywords: vec![item_label.to_lowercase()],
                                    span { class: "cmd-icon cmd-page", "○" }
                                    span { "{item_label}" }
                                }
                            }
                        }
                    }
                    CommandSeparator { group_before: "items", group_after: "tabs" }
                    CommandGroup { id: "tabs", heading: "Tabs",
                        for &tab in SUB_TABS {
                            CommandItem {
                                key: "tab-{tab}",
                                id: tab,
                                label: tab,
                                keywords: vec!["tab".to_string()],
                                span { class: "cmd-icon", "▣" }
                                span { "{tab}" }
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

/* Shell grid: sidebar | (tabs / content / footer) */
[data-shell] {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: 42px 1fr 36px;
    height: 100vh;
    overflow: hidden;
}

/* Desktop sidebar — spans all 3 rows */
[data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
    grid-column: 1;
    grid-row: 1 / -1;
    width: 220px;
    background: #fafafa;
    border-right: 1px solid #eaeaea;
    padding: 1rem 0;
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
    font-size: 0.9rem;
    color: #111;
    padding: 0 1rem 0.75rem;
    border-bottom: 1px solid #eaeaea;
    margin-bottom: 0.5rem;
    letter-spacing: -0.01em;
}

/* Collapsible sections */
.nav-sections { padding: 0 0.5rem; }
.nav-section { margin-bottom: 0.25rem; }
.section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.4rem 0.625rem;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 0.775rem;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-radius: 4px;
    text-align: left;
}
.section-header:hover { background: #f0f0f0; color: #333; }
.chevron {
    font-size: 0.8rem;
    color: #aaa;
    transition: transform 0.2s ease;
    line-height: 1;
}
/* Rotate chevron when expanded */
[aria-expanded="true"] .chevron { transform: rotate(90deg); }

.section-items {
    overflow: hidden;
    max-height: 0;
    transition: max-height 0.2s ease;
}
/* Show items when section header is expanded */
.section-header[aria-expanded="true"] + .section-items {
    max-height: 200px;
}

.nav-item {
    display: block;
    width: 100%;
    padding: 0.375rem 0.625rem 0.375rem 1.25rem;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 0.85rem;
    color: #555;
    border-radius: 4px;
}
.nav-item:hover { background: #f0f0f0; color: #111; }
.nav-item.active { background: #e8e8ff; color: #3b3bf6; font-weight: 500; }

/* Mobile drawer */
[data-shell-sidebar][data-shell-sidebar-mobile] {
    position: fixed;
    inset: 0;
    z-index: 100;
    width: 260px;
    background: #fafafa;
    padding: 1rem;
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
    margin-bottom: 0.75rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid #eaeaea;
}
.sidebar-title { font-weight: 700; color: #111; font-size: 0.9rem; }
.sidebar-close {
    background: none;
    border: none;
    font-size: 1.4rem;
    color: #999;
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

/* Sub-tabs bar (tabs slot — placed at grid row 1, col 2) */
[data-shell-tabs] {
    grid-column: 2;
    grid-row: 1;
    border-bottom: 1px solid #eaeaea;
    background: #fff;
}
.sub-tabs-bar {
    display: flex;
    align-items: center;
    height: 42px;
    padding: 0 1rem;
    gap: 0.5rem;
}
.hamburger {
    background: none;
    border: none;
    font-size: 1.1rem;
    cursor: pointer;
    color: #666;
    padding: 0;
    flex-shrink: 0;
}
.sidebar-toggle {
    background: none;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.2rem 0.5rem;
    font-size: 0.8rem;
    cursor: pointer;
    color: #666;
    flex-shrink: 0;
}
.sidebar-toggle:hover { background: #f5f5f5; }
.tabs-list { display: flex; align-items: center; gap: 0.25rem; flex: 1; }
.sub-tab {
    padding: 0.35rem 0.75rem;
    border: none;
    border-radius: 4px;
    background: none;
    font-size: 0.85rem;
    cursor: pointer;
    color: #666;
}
.sub-tab:hover { background: #f5f5f5; color: #333; }
.sub-tab.active { background: #f0f0ff; color: #3b3bf6; font-weight: 500; }
.global-search-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.75rem;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: #f8f8f8;
    cursor: pointer;
    font-size: 0.8rem;
    color: #888;
    flex-shrink: 0;
}
.global-search-btn:hover { border-color: #bbb; background: #eee; color: #555; }
.global-search-btn kbd {
    font-size: 0.68rem;
    font-family: monospace;
    background: #e0e0e0;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
}

/* Content (grid row 2) */
[data-shell-content] {
    grid-column: 2;
    grid-row: 2;
    overflow-y: auto;
    background: #fff;
}
.content-pane { height: 100%; display: flex; flex-direction: column; }
.content-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem 1.5rem 0.75rem;
    border-bottom: 1px solid #f5f5f5;
    flex-shrink: 0;
}
.content-header h1 { font-size: 1.05rem; font-weight: 600; color: #111; }
.content-tab-label { font-size: 0.85rem; color: #aaa; flex: 1; }
.new-btn {
    padding: 0.35rem 0.875rem;
    background: #111;
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
}
.new-btn:hover { background: #333; }

/* Data table */
.table-wrapper { flex: 1; overflow-y: auto; }
.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
}
.data-table th {
    padding: 0.625rem 1.5rem;
    text-align: left;
    font-size: 0.72rem;
    font-weight: 600;
    color: #999;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid #f5f5f5;
    white-space: nowrap;
}
.data-table td {
    padding: 0.75rem 1.5rem;
    border-bottom: 1px solid #f5f5f5;
    color: #333;
}
.data-table tr:hover td { background: #fafafa; }
.td-name {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
    color: #111;
}
.row-icon { color: #ccc; }
.status-pill {
    display: inline-block;
    padding: 0.15rem 0.55rem;
    border-radius: 99px;
    font-size: 0.72rem;
    font-weight: 500;
}
.status-pill.ready { background: #f0fdf4; color: #16a34a; }
.status-pill.building { background: #fffbeb; color: #d97706; }
.status-pill.error { background: #fef2f2; color: #ef4444; }
.td-muted { color: #999; font-size: 0.8rem; }
.td-actions { width: 40px; text-align: center; }
.row-action {
    background: none;
    border: none;
    font-size: 1rem;
    cursor: pointer;
    color: #aaa;
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
}
.row-action:hover { background: #f0f0f0; color: #333; }

/* Footer (grid row 3) */
[data-shell-footer] {
    grid-column: 2;
    grid-row: 3;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    background: #fafafa;
    border-top: 1px solid #eaeaea;
    font-size: 0.75rem;
    color: #999;
}
.breadcrumb { display: flex; align-items: center; }
.sep { margin: 0 0.3rem; color: #ccc; }
.pagination { display: flex; align-items: center; gap: 0.75rem; }
.page-btn {
    background: none;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.2rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    color: #666;
}
.page-btn:hover { background: #f0f0f0; }
.page-info { color: #aaa; }

/* Search overlay */
[data-shell-search] { display: contents; }
[data-cmdk-overlay] { background: rgba(0,0,0,0.35); }
[data-cmdk-dialog] {
    top: 15%;
    left: 50%;
    transform: translateX(-50%);
    width: 90%;
    max-width: 520px;
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 16px 48px rgba(0,0,0,0.18);
    overflow: hidden;
    border: 1px solid #e5e5e5;
}
[data-cmdk-input] {
    width: 100%;
    padding: 0.875rem 1rem;
    border: none;
    border-bottom: 1px solid #eee;
    outline: none;
    font-size: 0.95rem;
}
[data-cmdk-input]::placeholder { color: #aaa; }
[data-cmdk-list] { max-height: 340px; overflow-y: auto; padding: 0.25rem 0; }
[data-cmdk-group-heading] {
    padding: 0.375rem 0.875rem;
    font-size: 0.65rem;
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
    border-radius: 5px;
    margin: 0 0.375rem;
}
[data-cmdk-item][aria-selected="true"] { background: #f0f0ff; color: #3b3bf6; }
[data-cmdk-empty] { padding: 1.25rem; text-align: center; font-size: 0.875rem; color: #aaa; }
[data-cmdk-separator] { height: 1px; background: #eee; margin: 0.25rem 0; }
.cmd-icon { color: #aaa; flex-shrink: 0; }
.cmd-page { color: #ccc; }

/* Mobile layout */
@media (max-width: 639px) {
    [data-shell] {
        grid-template-columns: 1fr;
        grid-template-rows: 42px 1fr 36px;
    }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) { display: none; }
    [data-shell-tabs] { grid-column: 1; }
    [data-shell-content] { grid-column: 1; }
    [data-shell-footer] { grid-column: 1; }
}
"#;
