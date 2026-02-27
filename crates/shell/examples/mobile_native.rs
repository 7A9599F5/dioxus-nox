//! mobile_native — "iOS/Android Native" pattern.
//!
//! Demonstrates:
//! - Bottom tab bar (tabs slot) — 4 icon tabs, always at the bottom on mobile
//! - Floating action button (fab slot) — opens the bottom sheet at Peek snap
//! - Persistent bottom sheet (sheet slot) — snaps: Hidden → Peek → Half → Full
//! - `data-shell-sheet-state` driving CSS `transform: translateY()` transitions
//! - Desktop: full sidebar replaces bottom tabs / FAB / sheet entirely
//!
//! Source inspiration: Things 3, iOS Spotlight, Material Design 3.
//!
//! Run on desktop:  cargo run --example mobile_native
//! Run on web:      dx serve --example mobile_native

use dioxus::prelude::*;
use dioxus_nox_shell::{
    AppShell, BreakpointConfig, DesktopSidebar, MobileSidebar, SheetSnap, ShellLayout,
    use_shell_context,
};

fn main() {
    dioxus::launch(App);
}

// ── Mock data ──────────────────────────────────────────────────────────────────

static NAV_TABS: &[(&str, &str, &str)] = &[
    ("inbox", "Inbox", "●"),
    ("today", "Today", "◎"),
    ("upcoming", "Upcoming", "◷"),
    ("logbook", "Logbook", "✓"),
];

/// (id, title, due, priority)
static TASKS: &[(&str, &str, &str, &str)] = &[
    ("t1", "Review pull request", "Today", "High"),
    ("t2", "Update docs", "Today", "Medium"),
    ("t3", "Write release notes", "Tomorrow", "Medium"),
    ("t4", "Fix flaky test", "Fri", "Low"),
    ("t5", "Triage issues", "Next week", "Low"),
];

// ── Root ───────────────────────────────────────────────────────────────────────

#[component]
fn App() -> Element {
    let active_tab: Signal<&'static str> = use_signal(|| "inbox");

    rsx! {
        style { {CSS} }
        AppShell {
            layout: ShellLayout::Vertical,
            mobile_sidebar: MobileSidebar::Hidden,
            desktop_sidebar: DesktopSidebar::Full,
            breakpoints: BreakpointConfig { compact_below: 640.0, expanded_above: 1024.0 },
            sidebar: rsx! { DesktopNav { active_tab } },
            tabs: rsx! { BottomTabs { active_tab } },
            fab: rsx! { FabButton {} },
            sheet: rsx! { QuickAddSheet {} },
            TaskList { active_tab }
        }
    }
}

// ── Desktop Sidebar ────────────────────────────────────────────────────────────

#[component]
fn DesktopNav(active_tab: Signal<&'static str>) -> Element {
    rsx! {
        nav {
            div { class: "brand", "Inbox" }
            h2 { "Navigation" }
            ul {
                for &(id, label, icon) in NAV_TABS {
                    li {
                        key: "{id}",
                        class: if (active_tab)() == id { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            let mut t = active_tab;
                            t.set(id);
                        },
                        span { class: "nav-icon", "{icon}" }
                        span { class: "label", "{label}" }
                    }
                }
            }
        }
    }
}

// ── Bottom Tab Bar (tabs slot) ─────────────────────────────────────────────────

#[component]
fn BottomTabs(active_tab: Signal<&'static str>) -> Element {
    rsx! {
        div { class: "bottom-tabs",
            for &(id, label, icon) in NAV_TABS {
                button {
                    key: "{id}",
                    class: if (active_tab)() == id { "tab-btn active" } else { "tab-btn" },
                    onclick: move |_| {
                        let mut t = active_tab;
                        t.set(id);
                    },
                    span { class: "tab-icon", "{icon}" }
                    span { class: "tab-label", "{label}" }
                }
            }
        }
    }
}

// ── FAB (fab slot) ────────────────────────────────────────────────────────────

#[component]
fn FabButton() -> Element {
    let ctx = use_shell_context();
    rsx! {
        button {
            class: "fab",
            onclick: move |_| ctx.set_sheet_snap(SheetSnap::Peek),
            "+"
        }
    }
}

// ── Bottom Sheet (sheet slot) ─────────────────────────────────────────────────

#[component]
fn QuickAddSheet() -> Element {
    let ctx = use_shell_context();
    let snap = (ctx.sheet_snap)();
    let task_input = use_signal(String::new);

    rsx! {
        // Backdrop — only interactive when sheet is visible
        if snap.is_visible() {
            div {
                class: "sheet-backdrop",
                onclick: move |_| ctx.set_sheet_snap(SheetSnap::Hidden),
            }
        }
        div { class: "sheet-panel",
            // Drag handle (visual affordance)
            div { class: "sheet-handle-bar" }

            div { class: "sheet-header",
                h3 { "Quick Add" }
                button {
                    class: "sheet-close",
                    onclick: move |_| ctx.set_sheet_snap(SheetSnap::Hidden),
                    "\u{00D7}"
                }
            }

            // Snap controls — substitute for drag gestures in this demo
            div { class: "snap-controls",
                span { class: "snap-label", "Snap:" }
                button {
                    class: if snap == SheetSnap::Peek { "snap-btn active" } else { "snap-btn" },
                    onclick: move |_| ctx.set_sheet_snap(SheetSnap::Peek),
                    "Peek"
                }
                button {
                    class: if snap == SheetSnap::Half { "snap-btn active" } else { "snap-btn" },
                    onclick: move |_| ctx.set_sheet_snap(SheetSnap::Half),
                    "Half"
                }
                button {
                    class: if snap == SheetSnap::Full { "snap-btn active" } else { "snap-btn" },
                    onclick: move |_| ctx.set_sheet_snap(SheetSnap::Full),
                    "Full"
                }
            }

            div { class: "sheet-body",
                input {
                    class: "task-input",
                    r#type: "text",
                    placeholder: "New task title…",
                    value: "{task_input}",
                    oninput: move |e| {
                        let mut ti = task_input;
                        ti.set(e.value());
                    },
                }
                p { class: "sheet-hint",
                    "data-shell-sheet-state: "
                    strong { "{snap.as_str()}" }
                }
                p { class: "sheet-hint", "Tap Peek / Half / Full to change snap. Tap backdrop to dismiss." }
            }
        }
    }
}

// ── Task List (children slot) ─────────────────────────────────────────────────

#[component]
fn TaskList(active_tab: Signal<&'static str>) -> Element {
    let ctx = use_shell_context();
    let tab = (active_tab)();
    let tab_label = NAV_TABS
        .iter()
        .find(|(id, _, _)| *id == tab)
        .map(|(_, l, _)| *l)
        .unwrap_or("Tasks");

    rsx! {
        div { class: "task-pane",
            div { class: "task-header",
                if ctx.is_mobile() {
                    h1 { "{tab_label}" }
                } else {
                    button {
                        class: "sidebar-toggle",
                        onclick: move |_| ctx.toggle_sidebar(),
                        if (ctx.sidebar_visible)() { "‹" } else { "›" }
                    }
                    h1 { "{tab_label}" }
                }
            }
            ul { class: "task-list",
                for &(_, title, due, priority) in TASKS {
                    li { class: "task-row",
                        div { class: "task-check" }
                        div { class: "task-info",
                            span { class: "task-title", "{title}" }
                            span { class: "task-due", "{due}" }
                        }
                        span { class: "task-priority priority-{priority.to_lowercase()}", "{priority}" }
                    }
                }
            }

            // Desktop: show sheet controls inline (sheet slot is hidden via CSS)
            if !ctx.is_mobile() {
                div { class: "desktop-add",
                    p { class: "desktop-add-hint",
                        "On mobile: tap the FAB (＋) to open the bottom sheet."
                    }
                    p { class: "desktop-add-hint",
                        "On desktop: sidebar + content replace the mobile bottom navigation."
                    }
                    button {
                        class: "desktop-add-btn",
                        onclick: move |_| ctx.set_sheet_snap(SheetSnap::Peek),
                        "+ New Task (demo sheet)"
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

/* ── Mobile-first layout ─────────────────────────────────────────────────── */

/* Mobile: single column, bottom tab bar occupies row 2 */
[data-shell] {
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr 56px;
    height: 100vh;
    overflow: hidden;
}

[data-shell-content] {
    grid-column: 1;
    grid-row: 1;
    overflow-y: auto;
    background: #f9f9fb;
}

/* Bottom tab bar */
[data-shell-tabs] {
    grid-column: 1;
    grid-row: 2;
    z-index: 10;
}
.bottom-tabs {
    display: flex;
    height: 56px;
    background: rgba(255,255,255,0.95);
    backdrop-filter: blur(8px);
    border-top: 1px solid #e5e5e5;
}
.tab-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border: none;
    background: none;
    cursor: pointer;
    color: #999;
    font-size: 0.62rem;
    padding: 0;
    transition: color 0.15s;
}
.tab-btn.active { color: #3b82f6; }
.tab-icon { font-size: 1.1rem; line-height: 1; }
.tab-label { font-size: 0.62rem; }

/* FAB — fixed above the tab bar */
[data-shell-fab] {
    position: fixed;
    bottom: 72px;
    right: 20px;
    z-index: 20;
}
.fab {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: #3b82f6;
    color: #fff;
    border: none;
    font-size: 1.75rem;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 16px rgba(59,130,246,0.4);
    transition: transform 0.15s, box-shadow 0.15s;
}
.fab:hover { transform: scale(1.05); box-shadow: 0 6px 20px rgba(59,130,246,0.5); }

/* Bottom sheet — positioned by data-shell-sheet-state */
[data-shell-sheet] {
    position: fixed;
    bottom: 56px; /* sit above tab bar */
    left: 0;
    right: 0;
    z-index: 30;
    height: 85vh;
    background: #fff;
    border-radius: 16px 16px 0 0;
    box-shadow: 0 -4px 32px rgba(0,0,0,0.15);
    transform: translateY(100%);
    transition: transform 0.3s cubic-bezier(0.32, 0.72, 0, 1);
    pointer-events: none;
}
[data-shell-sheet][data-shell-sheet-state="hidden"] {
    transform: translateY(100%);
    pointer-events: none;
}
[data-shell-sheet][data-shell-sheet-state="peek"] {
    transform: translateY(72%);
    pointer-events: auto;
}
[data-shell-sheet][data-shell-sheet-state="half"] {
    transform: translateY(50%);
    pointer-events: auto;
}
[data-shell-sheet][data-shell-sheet-state="full"] {
    transform: translateY(0);
    pointer-events: auto;
}

.sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 29;
    background: rgba(0,0,0,0.25);
}
.sheet-handle-bar {
    width: 36px;
    height: 4px;
    background: #ddd;
    border-radius: 2px;
    margin: 10px auto 0;
}
.sheet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.25rem 0.5rem;
    border-bottom: 1px solid #f0f0f0;
}
.sheet-header h3 { font-size: 0.95rem; font-weight: 600; }
.sheet-close {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #aaa;
    cursor: pointer;
    padding: 0;
    line-height: 1;
}
.sheet-close:hover { color: #333; }
.snap-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    border-bottom: 1px solid #f5f5f5;
}
.snap-label { font-size: 0.75rem; color: #aaa; }
.snap-btn {
    padding: 0.25rem 0.75rem;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    background: #fff;
    font-size: 0.78rem;
    cursor: pointer;
    color: #555;
}
.snap-btn.active {
    background: #3b82f6;
    border-color: #3b82f6;
    color: #fff;
}
.sheet-body { padding: 1rem 1.25rem; }
.task-input {
    width: 100%;
    padding: 0.6rem 0.875rem;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    font-size: 0.9rem;
    outline: none;
    margin-bottom: 0.75rem;
}
.task-input:focus { border-color: #3b82f6; }
.sheet-hint { font-size: 0.78rem; color: #aaa; margin-top: 0.375rem; line-height: 1.5; }

/* Task list */
.task-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
}
.task-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #eee;
    background: #fff;
    flex-shrink: 0;
}
.task-header h1 { font-size: 1.1rem; font-weight: 600; color: #111; flex: 1; }
.sidebar-toggle {
    background: none;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.8rem;
    cursor: pointer;
    color: #666;
}
.task-list { list-style: none; flex: 1; overflow-y: auto; padding: 0.5rem 0; }
.task-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid #f5f5f5;
    background: #fff;
}
.task-row:hover { background: #fafafa; }
.task-check {
    width: 20px;
    height: 20px;
    border: 2px solid #ddd;
    border-radius: 50%;
    flex-shrink: 0;
    cursor: pointer;
}
.task-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.task-title { font-size: 0.875rem; color: #222; }
.task-due { font-size: 0.72rem; color: #aaa; }
.task-priority {
    font-size: 0.68rem;
    padding: 0.15rem 0.45rem;
    border-radius: 99px;
    font-weight: 500;
    flex-shrink: 0;
}
.priority-high { background: #fef2f2; color: #ef4444; }
.priority-medium { background: #fffbeb; color: #f59e0b; }
.priority-low { background: #f0fdf4; color: #22c55e; }

.desktop-add {
    padding: 1.5rem;
    border-top: 1px solid #eee;
    background: #fff;
}
.desktop-add-hint { font-size: 0.8rem; color: #888; margin-bottom: 0.5rem; }
.desktop-add-btn {
    margin-top: 0.75rem;
    padding: 0.5rem 1rem;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.875rem;
    color: #555;
}
.desktop-add-btn:hover { background: #f5f5f5; }

/* ── Desktop layout (>= 1024px) ───────────────────────────────────────────── */
@media (min-width: 1024px) {
    [data-shell] {
        grid-template-columns: auto 1fr;
        grid-template-rows: 1fr;
    }
    [data-shell-sidebar]:not([data-shell-sidebar-mobile]) {
        grid-column: 1;
        grid-row: 1;
        display: block;
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
    [data-shell-content] {
        grid-column: 2;
        grid-row: 1;
    }
    /* Hide mobile-only chrome on desktop */
    [data-shell-tabs] { display: none; }
    [data-shell-fab] { display: none; }
    [data-shell-sheet] { display: none !important; }

    /* Desktop sidebar styles */
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
    .nav-icon { font-size: 1rem; }
}
"#;
