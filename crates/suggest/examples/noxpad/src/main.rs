//! NoxPad — Full 6-crate demo for dioxus-nox.
//!
//! A Notion/Obsidian-style markdown notes editor demonstrating:
//! - `dioxus-nox-shell`     — 3-pane AppShell layout
//! - `dioxus-nox-markdown`  — InlineEditor with slash commands
//! - `dioxus-nox-suggest`   — /, @, # inline triggers
//! - `dioxus-nox-cmdk`      — Ctrl+K command palette
//! - `dioxus-nox-tag-input` — tag pills
//! - `dioxus-nox-dnd`       — horizontal pill drag-to-reorder
//! - `dioxus-nox-preview`   — debounced preview in palette
//!
//! Run with: dx serve -p noxpad

// NOTE: DOM cursor restoration after text replacement uses `document::eval` (wasm32 only).
// NOTE: Note data is not persisted — seed data only.
// NOTE: Tag autocomplete is plain substring match against all note tags.

#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandEmpty, CommandGroup, CommandHighlight, CommandInput, CommandItem,
    CommandList, CommandRoot, use_command_palette,
};
use dioxus_nox_dnd::{
    DragContextProvider, DragId, DragOverlay, ReorderEvent, SortableContext, SortableItem,
};
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_markdown::markdown;
use dioxus_nox_markdown::prelude::{
    ActiveBlockInputEvent, LivePreviewVariant, Mode, use_markdown_context,
};
use dioxus_nox_preview::{use_debounced_active, use_preview_cache};
use dioxus_nox_shell::AppShell;
use dioxus_nox_suggest::{TriggerConfig, TriggerSelectEvent, suggest, use_suggestion};

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
       background: #0f0f0f; color: #e0e0e0; height: 100vh; overflow: hidden; }

/* Shell layout */
[data-shell] { display: grid; height: 100vh; overflow: hidden;
    grid-template-rows: 1fr auto;
    grid-template-columns: 240px 1fr 280px;
    grid-template-areas: "sidebar main preview" "footer footer footer"; }
[data-shell-columns="1"] { grid-template-columns: 1fr; grid-template-areas: "main" "footer"; }
[data-shell-sidebar] { grid-area: sidebar; border-right: 1px solid #2a2a2a;
    overflow-y: auto; background: #111; }
[data-shell-content] { grid-area: main; display: flex; flex-direction: column;
    overflow: hidden; }
[data-shell-preview] { grid-area: preview; border-left: 1px solid #2a2a2a;
    overflow-y: auto; background: #111; padding: 16px; }
[data-shell-footer] { grid-area: footer; border-top: 1px solid #2a2a2a;
    background: #111; padding: 4px 16px; font-size: 12px; color: #666;
    display: flex; gap: 16px; align-items: center; }
[data-shell-search] { position: fixed; inset: 0; z-index: 100;
    display: none; align-items: flex-start; justify-content: center;
    padding-top: 80px; background: rgba(0,0,0,0.6); backdrop-filter: blur(4px); }
[data-shell-search][data-shell-search-active="true"] { display: flex; }

/* Sidebar */
.sidebar-header { padding: 12px 16px; font-weight: 600; font-size: 13px;
    color: #888; text-transform: uppercase; letter-spacing: 0.05em; }
.note-item { padding: 8px 16px; cursor: pointer; border-left: 2px solid transparent;
    transition: background 0.1s; font-size: 14px; }
.note-item:hover { background: #1a1a1a; }
.note-item[data-active="true"] { border-left-color: #7c6af7; background: #1a1a1a; color: #fff; }
.note-item-title { font-weight: 500; }
.note-item-tags { font-size: 11px; color: #555; margin-top: 2px; }

/* Editor */
.note-editor { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.note-editor-header { padding: 12px 24px 0; }
.note-title { font-size: 22px; font-weight: 700; background: transparent; border: none;
    color: #e0e0e0; width: 100%; outline: none; padding: 0; }
[data-md-root] { flex: 1; display: flex; flex-direction: column; overflow: hidden; padding: 0 24px; }
[data-md-editor][data-state="active"] { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
[data-md-inline-editor] { flex: 1; padding: 12px 0; outline: none;
    overflow-y: auto; line-height: 1.7; font-size: 15px; caret-color: #7c6af7; }
[data-md-inline-editor] h1 { font-size: 1.8em; font-weight: 700; margin: 0.8em 0 0.4em; }
[data-md-inline-editor] h2 { font-size: 1.4em; font-weight: 600; margin: 0.7em 0 0.3em; }
[data-md-inline-editor] h3 { font-size: 1.2em; font-weight: 600; margin: 0.6em 0 0.2em; }
[data-md-inline-editor] p { margin: 0.4em 0; }
[data-md-inline-editor] code { background: #222; padding: 1px 4px; border-radius: 3px;
    font-family: monospace; font-size: 0.9em; }
[data-md-inline-editor] pre { background: #1a1a1a; padding: 12px; border-radius: 6px; margin: 8px 0; }
[data-md-inline-editor] blockquote { border-left: 3px solid #444; padding-left: 12px;
    color: #888; margin: 8px 0; }
[data-md-inline-editor] ul, [data-md-inline-editor] ol { padding-left: 20px; margin: 4px 0; }
[data-md-inline-editor] hr { border: none; border-top: 1px solid #2a2a2a; margin: 16px 0; }
[data-block-index] { min-height: 1.2em; }
[data-md-root] textarea { flex: 1; background: transparent; border: none; outline: none;
    color: #e0e0e0; font-family: 'Fira Code', monospace; font-size: 14px;
    line-height: 1.7; resize: none; padding: 12px 0; }

/* Suggest floating list */
[data-slot="trigger-list"] { display: none; }
[data-slot="trigger-list"][data-state="open"] {
    display: block; position: fixed; z-index: 200; background: #1e1e1e;
    border: 1px solid #333; border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5);
    min-width: 240px; max-height: 320px; overflow-y: auto; padding: 4px; }
[data-suggest-group-label] { padding: 4px 8px; font-size: 11px; font-weight: 600;
    color: #666; text-transform: uppercase; letter-spacing: 0.05em; }
[data-slot="trigger-list"] [role="option"] { padding: 8px 12px; border-radius: 6px;
    cursor: pointer; font-size: 14px; display: flex; align-items: center; gap: 8px; }
[data-slot="trigger-list"] [role="option"]:hover,
[data-slot="trigger-list"] [role="option"][data-highlighted="true"] { background: #2d2d4a; color: #7c6af7; }
.cmd-icon { font-size: 16px; width: 24px; text-align: center; }

/* Tag bar */
.tag-bar { padding: 8px 24px; border-top: 1px solid #1e1e1e;
    display: flex; align-items: center; flex-wrap: wrap; gap: 6px; min-height: 40px; }
.tag-pill { display: inline-flex; align-items: center; gap: 4px;
    background: #1e1e2e; border: 1px solid #333; border-radius: 20px;
    padding: 2px 8px 2px 10px; font-size: 12px; color: #a0a0b0;
    cursor: grab; user-select: none; }
.tag-remove { border: none; background: none; color: #666; cursor: pointer;
    font-size: 14px; padding: 0 2px; line-height: 1; }
.tag-remove:hover { color: #e06c75; }
.tag-input-wrap input { background: transparent; border: none; outline: none;
    color: #e0e0e0; font-size: 12px; width: 80px; }
.tag-drag-overlay .tag-pill { opacity: 1; transform: scale(1.05);
    box-shadow: 0 4px 16px rgba(124,106,247,0.3); }

/* Mode bar */
.mode-bar { display: flex; gap: 2px; padding: 6px 24px 0; }
.mode-tab { padding: 4px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;
    border: none; background: transparent; color: #666; }
.mode-tab[data-active="true"] { background: #1e1e2e; color: #7c6af7; }

/* Cmdk dialog */
.cmdk-dialog { background: #1a1a1a; border: 1px solid #333; border-radius: 12px;
    width: 560px; max-height: 480px; overflow: hidden; display: flex;
    flex-direction: column; box-shadow: 0 16px 64px rgba(0,0,0,0.8); }
.cmdk-input-wrap { padding: 12px 16px; border-bottom: 1px solid #2a2a2a; }
.cmdk-input-wrap input { width: 100%; background: transparent; border: none; outline: none;
    color: #e0e0e0; font-size: 15px; }
.cmdk-list { overflow-y: auto; padding: 8px; max-height: 360px; }
.cmdk-group-heading { padding: 4px 8px; font-size: 11px; color: #666;
    font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
.cmdk-item { padding: 8px 12px; border-radius: 6px; cursor: pointer; font-size: 14px;
    display: flex; justify-content: space-between; align-items: center; }
.cmdk-item[data-active="true"] { background: #2d2d4a; }
.cmdk-item-title { font-weight: 500; }
.cmdk-item-tags { font-size: 11px; color: #555; }
mark { background: none; color: #7c6af7; font-weight: 600; }
.cmdk-empty { padding: 24px; text-align: center; color: #555; font-size: 14px; }

/* Preview pane */
.preview-title { font-size: 13px; font-weight: 600; color: #888;
    margin-bottom: 12px; text-transform: uppercase; letter-spacing: 0.05em; }
.preview-content { font-size: 13px; line-height: 1.6; color: #aaa; }
[data-preview-loading="true"] .preview-content { opacity: 0.4; }
.preview-empty { color: #444; font-size: 13px; }

@media (max-width: 639px) {
    [data-shell] { grid-template-columns: 1fr; grid-template-areas: "main" "footer"; }
    [data-shell-sidebar] { display: none; }
    [data-shell-preview] { display: none; }
}
"#;

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Note {
    id:      String,
    title:   String,
    content: String,
    tags:    Vec<String>,
}

fn seed_notes() -> Vec<Note> {
    vec![
        Note {
            id: "rust-ownership".into(),
            title: "Rust Ownership".into(),
            content: "# Rust Ownership\n\nRust's ownership system enables memory safety without a garbage collector.\n\n## The Rules\n\n- Each value has a single *owner*\n- When the owner goes out of scope, the value is dropped\n\n## Borrowing\n\nBorrowing lets you reference data without taking ownership.\n\n```rust\nfn print_len(s: &String) {\n    println!(\"Length: {}\", s.len());\n}\n```\n\n- [ ] Review lifetimes chapter\n- [ ] Practice with custom types".into(),
            tags: vec!["rust".into(), "memory".into(), "ownership".into()],
        },
        Note {
            id: "wasm-perf".into(),
            title: "WASM Performance".into(),
            content: "# WASM Performance\n\nWebAssembly runs at near-native speed in the browser.\n\n## Key Techniques\n\n### Minimize JS-WASM Boundary Crossings\n\nEach call across the boundary has overhead. Batch operations where possible.\n\n### Use Linear Memory\n\nPrefer `Vec<u8>` over complex allocations when passing data to JS.\n\n---\n\n> The fastest code is code that doesn't run.".into(),
            tags: vec!["wasm".into(), "performance".into(), "rust".into()],
        },
        Note {
            id: "meeting-notes".into(),
            title: "Meeting Notes".into(),
            content: "# Meeting Notes — Component API Review\n\n## Attendees\n\n@Alice @Bob @Carol\n\n## Action Items\n\n- [ ] Alice: update cmdk signal pattern\n- [ ] Bob: write migration guide\n- [ ] Carol: add E2E tests for dnd\n\n## Notes\n\nDecided to use `data-state` attributes consistently across all crates.".into(),
            tags: vec!["meeting".into(), "planning".into()],
        },
        Note {
            id: "reading-list".into(),
            title: "Reading List".into(),
            content: "# Reading List\n\n## In Progress\n\n- *Programming Rust* — Blandy & Orendorff\n- *The Rust Programming Language* — Klabnik & Nichols\n\n## Backlog\n\n- *Crafting Interpreters* — Robert Nystrom\n- *Database Internals* — Alex Petrov\n\n## Completed\n\n- *The Art of Problem Solving* ✓\n\n#rust #books #learning".into(),
            tags: vec!["books".into(), "learning".into(), "rust".into()],
        },
    ]
}

/// Compute the replacement text for a slash/mention/hashtag command.
fn cmd_to_text(trigger_char: char, value: &str) -> String {
    match trigger_char {
        '/' => match value {
            "h1"      => "# ".into(),
            "h2"      => "## ".into(),
            "h3"      => "### ".into(),
            "bold"    => "**text**".into(),
            "italic"  => "_text_".into(),
            "code"    => "`code`".into(),
            "quote"   => "> ".into(),
            "task"    => "- [ ] ".into(),
            "table"   => "| Col 1 | Col 2 |\n|-------|-------|\n|       |       |".into(),
            "divider" => "---".into(),
            _         => String::new(),
        },
        '@' => format!("@{value} "),
        '#' => format!("#{value} "),
        _   => String::new(),
    }
}

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let notes: Signal<Vec<Note>>          = use_signal(seed_notes);
    let active_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let mode: Signal<Mode>                = use_signal(|| Mode::LivePreview);
    let search_open: Signal<bool>         = use_signal(|| false);
    let search_read: ReadSignal<bool>     = search_open.into();

    rsx! {
        style { {CSS} }
        AppShell {
            search_active: Some(search_read),
            on_search_change: move |v| { let mut so = search_open; so.set(v); },
            sidebar: rsx! { NoteSidebar { notes, active_idx } },
            search:  rsx! { CmdkPalette { notes, active_idx, search_open } },
            preview: rsx! { PreviewPane { notes, active_idx } },
            footer:  rsx! { StatusBar { notes, active_idx } },

            if let Some(idx) = (active_idx)() {
                NoteEditor { notes, active_idx: idx, mode, search_open }
            } else {
                div { style: "padding:32px; color:#444", "Select a note to begin editing" }
            }
        }
    }
}

// ── NoteSidebar ───────────────────────────────────────────────────────────────

#[component]
fn NoteSidebar(notes: Signal<Vec<Note>>, active_idx: Signal<Option<usize>>) -> Element {
    rsx! {
        div {
            div { class: "sidebar-header", "Notes" }
            for (i, note) in notes.read().iter().enumerate() {
                {
                    let is_active = (active_idx)() == Some(i);
                    let tags_preview = note.tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                    let title = note.title.clone();
                    rsx! {
                        div {
                            class: "note-item",
                            "data-active": if is_active { "true" } else { "false" },
                            onclick: move |_| active_idx.set(Some(i)),
                            div { class: "note-item-title", "{title}" }
                            if !tags_preview.is_empty() {
                                div { class: "note-item-tags", "{tags_preview}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── NoteEditor ────────────────────────────────────────────────────────────────

#[component]
fn NoteEditor(
    notes: Signal<Vec<Note>>,
    active_idx: usize,
    mode: Signal<Mode>,
    search_open: Signal<bool>,
) -> Element {
    // Track which note we're editing as a signal so effects can subscribe to switches.
    let current_idx: Signal<usize> = use_signal(|| active_idx);
    {
        let mut ci = current_idx;
        ci.set(active_idx);
    }

    // Controlled value signal for markdown::Root.
    let content: Signal<String> = use_signal(|| {
        notes.peek().get(active_idx).map(|n| n.content.clone()).unwrap_or_default()
    });

    // When the active note switches, reset the content signal.
    // Use notes.peek() to avoid subscribing to content changes (only idx matters here).
    use_effect(move || {
        let idx = *current_idx.read();
        let new_content = notes.peek().get(idx).map(|n| n.content.clone()).unwrap_or_default();
        let mut c = content;
        c.set(new_content);
    });

    // Inline-mode cursor state for suggest::Trigger external_input.
    let inline_input: Signal<(String, usize)> = use_signal(|| (String::new(), 0));
    let inline_block_idx: Signal<usize>        = use_signal(|| 0);
    // markdown::Root instance_n exposed via EditorBody for DOM ID construction.
    let editor_n: Signal<Option<u64>>          = use_signal(|| None);

    let note_title = notes.read().get(active_idx).map(|n| n.title.clone()).unwrap_or_default();

    // All tags across all notes for # suggestions.
    let all_tags: Vec<String> = {
        let ns = notes.read();
        let mut tags: Vec<String> = ns.iter().flat_map(|n| n.tags.iter().cloned()).collect();
        tags.sort();
        tags.dedup();
        tags
    };
    let people = vec!["Alice", "Bob", "Carol", "Dan", "Eve"];

    rsx! {
        div { class: "note-editor",
            ModeBar { mode }

            div { class: "note-editor-header",
                input {
                    class: "note-title",
                    value: "{note_title}",
                    oninput: move |evt| {
                        let v = evt.value();
                        let mut ns = notes.write();
                        if let Some(note) = ns.get_mut(active_idx) {
                            note.title = v;
                        }
                    }
                }
            }

            suggest::Root {
                triggers: vec![
                    TriggerConfig::slash(),
                    TriggerConfig::mention(),
                    TriggerConfig::hashtag(),
                ],
                on_select: move |evt: TriggerSelectEvent| {
                    let replacement = cmd_to_text(evt.trigger_char, &evt.value);
                    if replacement.is_empty() { return; }
                    let trigger_end = evt.trigger_offset
                        + evt.trigger_char.len_utf8()
                        + evt.filter.len();

                    if (mode)() == Mode::LivePreview {
                        // Inline mode: replace text in contenteditable block.
                        let (block_text, _) = (*inline_input.read()).clone();
                        let idx         = *inline_block_idx.read();
                        let safe_start  = evt.trigger_offset.min(block_text.len());
                        let safe_end    = trigger_end.min(block_text.len());
                        let new_block   = format!("{}{}{}", &block_text[..safe_start],
                                                  replacement, &block_text[safe_end..]);
                        let cursor_after = safe_start + replacement.len();
                        if let Some(n) = *editor_n.read() {
                            let eid = format!("nox-md-{n}-inline");
                            spawn(async move {
                                let js = format!(r#"(function(){{
    var ed=document.getElementById('{eid}');
    var bl=ed?ed.querySelector('[data-block-index="{idx}"]'):null;
    if(!bl)return;
    bl.textContent={new_block:?};
    var sel=window.getSelection(),range=document.createRange();
    var tn=bl.firstChild||bl;
    var off=Math.min({cursor_after},bl.textContent.length);
    try{{range.setStart(tn,off);range.collapse(true);sel.removeAllRanges();sel.addRange(range);}}catch(e){{}}
    bl.dispatchEvent(new Event('input',{{bubbles:true}}));
}})();"#);
                                let _ = document::eval(&js).await;
                            });
                        }
                    } else {
                        // Source / Read mode: replace in controlled content signal.
                        let old         = (*content.read()).clone();
                        let safe_start  = evt.trigger_offset.min(old.len());
                        let safe_end    = trigger_end.min(old.len());
                        let new_text    = format!("{}{}{}", &old[..safe_start], replacement, &old[safe_end..]);
                        let cursor_after = safe_start + replacement.len();
                        let mut c = content;
                        c.set(new_text.clone());
                        // Sync textarea DOM cursor position.
                        if let Some(n) = *editor_n.read() {
                            let eid = format!("nox-md-{n}-editor");
                            let nt  = new_text.clone();
                            spawn(async move {
                                let js = format!(
                                    "var el=document.getElementById('{eid}');\
                                     if(el){{el.value={nt:?};\
                                     el.setSelectionRange({cursor_after},{cursor_after});\
                                     el.dispatchEvent(new Event('input',{{bubbles:true}}));}}"
                                );
                                let _ = document::eval(&js).await;
                            });
                        }
                    }
                },

                suggest::Trigger {
                    // Feed inline-editor cursor position reactively when in Inline mode.
                    external_input: if (mode)() == Mode::LivePreview {
                        Some(inline_input.into())
                    } else {
                        None
                    },

                    markdown::Root {
                        value: Some(content),
                        on_value_change: move |v: String| {
                            let mut c = content;
                            c.set(v.clone());
                            let mut ns = notes.write();
                            if let Some(note) = ns.get_mut(active_idx) {
                                note.content = v;
                            }
                        },
                        mode: Some(mode),
                        live_preview_variant: LivePreviewVariant::Inline,

                        EditorBody { inline_input, inline_block_idx, editor_n }
                    }
                }

                SlashList { all_tags, people }
            }

            TagBar { notes, active_idx }
        }
    }
}

// ── EditorBody (inside markdown::Root context) ────────────────────────────────

#[component]
fn EditorBody(
    inline_input: Signal<(String, usize)>,
    inline_block_idx: Signal<usize>,
    editor_n: Signal<Option<u64>>,
) -> Element {
    let ctx = use_markdown_context();
    // Expose the instance_n so the parent can construct DOM IDs.
    {
        let mut en = editor_n;
        en.set(Some(ctx.instance_n));
    }

    rsx! {
        markdown::Editor {
            on_active_block_input: move |e: ActiveBlockInputEvent| {
                let mut ii = inline_input;
                ii.set((e.text, e.cursor_utf16));
                let mut ib = inline_block_idx;
                ib.set(e.block_idx);
            },
        }
        markdown::Preview {}
    }
}

// ── ModeBar ───────────────────────────────────────────────────────────────────

#[component]
fn ModeBar(mode: Signal<Mode>) -> Element {
    let cur = (mode)();
    rsx! {
        div { class: "mode-bar",
            button {
                class: "mode-tab",
                "data-active": if cur == Mode::Source { "true" } else { "false" },
                onclick: move |_| { let mut m = mode; m.set(Mode::Source); },
                "Source"
            }
            button {
                class: "mode-tab",
                "data-active": if cur == Mode::LivePreview { "true" } else { "false" },
                onclick: move |_| { let mut m = mode; m.set(Mode::LivePreview); },
                "Inline"
            }
            button {
                class: "mode-tab",
                "data-active": if cur == Mode::Read { "true" } else { "false" },
                onclick: move |_| { let mut m = mode; m.set(Mode::Read); },
                "Read"
            }
        }
    }
}

// ── SlashList ─────────────────────────────────────────────────────────────────

#[component]
fn SlashList(all_tags: Vec<String>, people: Vec<&'static str>) -> Element {
    let sg     = use_suggestion();
    let filter = sg.filter();
    let trigger = sg.active_char();

    rsx! {
        suggest::List {
            if trigger == Some('/') {
                {
                    let slash_cmds: &[(&str, &str, &str)] = &[
                        ("h1",      "Heading 1",   "H₁"),
                        ("h2",      "Heading 2",   "H₂"),
                        ("h3",      "Heading 3",   "H₃"),
                        ("bold",    "Bold",         "𝐁"),
                        ("italic",  "Italic",       "𝐼"),
                        ("code",    "Inline Code",  "</>"),
                        ("quote",   "Blockquote",   "❝"),
                        ("task",    "Task Item",    "☑"),
                        ("table",   "Table",        "▦"),
                        ("divider", "Divider",      "─"),
                    ];
                    let f = filter.to_lowercase();
                    rsx! {
                        for &(val, label, icon) in slash_cmds.iter()
                            .filter(|&&(v, l, _)| f.is_empty() || v.contains(&*f) || l.to_lowercase().contains(&*f))
                        {
                            suggest::Item { value: val.to_string(),
                                span { class: "cmd-icon", "{icon}" }
                                "{label}"
                            }
                        }
                    }
                }
            }

            if trigger == Some('@') {
                {
                    let f = filter.to_lowercase();
                    rsx! {
                        for &person in people.iter()
                            .filter(|&&p| f.is_empty() || p.to_lowercase().contains(&*f))
                        {
                            suggest::Item { value: person.to_string(),
                                span { class: "cmd-icon", "👤" }
                                "{person}"
                            }
                        }
                    }
                }
            }

            if trigger == Some('#') {
                {
                    let f = filter.to_lowercase();
                    rsx! {
                        for tag in all_tags.iter()
                            .filter(|t| f.is_empty() || t.to_lowercase().contains(&*f))
                        {
                            suggest::Item { value: tag.clone(),
                                span { class: "cmd-icon", "#" }
                                "{tag}"
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── TagBar ────────────────────────────────────────────────────────────────────

#[component]
fn TagBar(notes: Signal<Vec<Note>>, active_idx: usize) -> Element {
    let tags: Vec<String> = notes.read().get(active_idx).map(|n| n.tags.clone()).unwrap_or_default();
    let tag_ids: Vec<DragId> = tags.iter().map(|t| DragId::new(t.clone())).collect();
    let new_tag: Signal<String> = use_signal(String::new);

    rsx! {
        div { class: "tag-bar",
            DragContextProvider {
                SortableContext {
                    id: DragId::new("tags"),
                    items: tag_ids.clone(),
                    orientation: Orientation::Horizontal,
                    on_reorder: move |evt: ReorderEvent| {
                        let mut ns = notes.write();
                        if let Some(note) = ns.get_mut(active_idx) {
                            if evt.from_index < note.tags.len() && evt.to_index < note.tags.len() {
                                note.tags.swap(evt.from_index, evt.to_index);
                            }
                        }
                    },
                    for (i, tag) in tags.iter().enumerate() {
                        {
                            let tag_val  = tag.clone();
                            let drag_id  = tag_ids.get(i).cloned()
                                .unwrap_or_else(|| DragId::new(tag.clone()));
                            rsx! {
                                SortableItem { id: drag_id,
                                    span { class: "tag-pill",
                                        "#{tag_val}"
                                        button {
                                            class: "tag-remove",
                                            onclick: move |_| {
                                                let tv = tag_val.clone();
                                                let mut ns = notes.write();
                                                if let Some(note) = ns.get_mut(active_idx) {
                                                    note.tags.retain(|t| t != &tv);
                                                }
                                            },
                                            "×"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                DragOverlay {
                    div { class: "tag-drag-overlay",
                        span { class: "tag-pill", "tag" }
                    }
                }
            }
            // Inline add-tag input
            div { class: "tag-input-wrap",
                input {
                    r#type: "text",
                    placeholder: "add tag…",
                    value: "{new_tag}",
                    oninput: move |evt| {
                        let mut nt = new_tag;
                        nt.set(evt.value());
                    },
                    onkeydown: move |evt| {
                        if evt.key().to_string() == "Enter" {
                            let val = new_tag.read().trim().to_string();
                            if !val.is_empty() {
                                let mut ns = notes.write();
                                if let Some(note) = ns.get_mut(active_idx) {
                                    if !note.tags.contains(&val) {
                                        note.tags.push(val);
                                    }
                                }
                                let mut nt = new_tag;
                                nt.set(String::new());
                            }
                        }
                    },
                }
            }
        }
    }
}

// ── CmdkPalette ───────────────────────────────────────────────────────────────

#[component]
fn CmdkPalette(
    notes: Signal<Vec<Note>>,
    active_idx: Signal<Option<usize>>,
    search_open: Signal<bool>,
) -> Element {
    let palette = use_command_palette(true);  // true = enable Ctrl+K shortcut

    // Sync shell search_open with palette open signal.
    use_effect(move || {
        let is_open = (palette.open)();
        let mut so = search_open;
        so.set(is_open);
    });

    // Track active item for debounced preview.
    let active_val: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        div {
            onclick: move |_| palette.hide(),
            div {
                class: "cmdk-dialog",
                onclick: move |evt| evt.stop_propagation(),

                CommandRoot {
                    on_active_change: move |v: Option<String>| {
                        let mut av = active_val;
                        av.set(v);
                    },
                    on_select: move |val: String| {
                        if let Some(id) = val.strip_prefix("note:") {
                            let id_str = id.to_string();
                            let ns = notes.read();
                            if let Some(i) = ns.iter().position(|n| n.id == id_str) {
                                active_idx.set(Some(i));
                            }
                        }
                        palette.hide();
                        let mut so = search_open;
                        so.set(false);
                    },

                    div { class: "cmdk-input-wrap",
                        CommandInput { placeholder: "Search notes…" }
                    }
                    div { class: "cmdk-list",
                        CommandList {
                            CommandEmpty {
                                div { class: "cmdk-empty", "No results found." }
                            }
                            CommandGroup { id: "notes", heading: "Notes",
                                for note in notes.read().iter() {
                                    {
                                        let val   = format!("note:{}", note.id);
                                        let title = note.title.clone();
                                        let tags_str = note.tags.join(", ");
                                        rsx! {
                                            CommandItem {
                                                id: val.clone(),
                                                value: val,
                                                label: title.clone(),
                                                div { class: "cmdk-item",
                                                    div { class: "cmdk-item-title",
                                                        CommandHighlight { label: title }
                                                    }
                                                    if !tags_str.is_empty() {
                                                        div { class: "cmdk-item-tags", "{tags_str}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                ActiveNotePreview { notes, active_val }
            }
        }
    }
}

// ── ActiveNotePreview (debounced preview in palette) ──────────────────────────

#[component]
fn ActiveNotePreview(notes: Signal<Vec<Note>>, active_val: Signal<Option<String>>) -> Element {
    let debounced = use_debounced_active(active_val.into(), 150);
    let cache     = use_preview_cache(10);
    let cache_eff = cache.clone();  // clone for use_effect capture

    use_effect(move || {
        let val = (debounced)();
        let Some(ref v) = val else { return };
        if cache_eff.get(v).is_some() { return; }

        let notes_snap = notes.read();
        if let Some(note_id) = v.strip_prefix("note:") {
            if let Some(note) = notes_snap.iter().find(|n| n.id == note_id) {
                let preview_text = note.content.lines().take(6).collect::<Vec<_>>().join("\n");
                let key = v.clone();
                cache_eff.insert(key, Rc::new(move || {
                    let pt = preview_text.clone();
                    rsx! { div { class: "preview-content", "{pt}" } }
                }));
            }
        }
    });

    let val       = (debounced)();
    let is_loading = (active_val)() != val;

    rsx! {
        div {
            "data-preview-loading": if is_loading { "true" } else { "false" },
            if let Some(ref v) = val {
                if let Some(render_fn) = cache.get(v) {
                    div { {(render_fn)()} }
                } else {
                    div { class: "preview-empty", "Loading…" }
                }
            } else {
                div { class: "preview-empty", "Arrow through results to preview" }
            }
        }
    }
}

// ── PreviewPane (right sidebar, always visible) ───────────────────────────────

#[component]
fn PreviewPane(notes: Signal<Vec<Note>>, active_idx: Signal<Option<usize>>) -> Element {
    let info = use_memo(move || {
        let idx = (active_idx)()?;
        notes.read().get(idx).map(|n| (n.title.clone(), n.content.clone()))
    });

    rsx! {
        if let Some((title, content)) = (info)() {
            div { class: "preview-title", "{title}" }
            div { class: "preview-content",
                {
                    let preview = content.chars().take(400).collect::<String>();
                    rsx! { "{preview}" }
                }
            }
        } else {
            div { class: "preview-empty", "No note selected" }
        }
    }
}

// ── StatusBar ─────────────────────────────────────────────────────────────────

#[component]
fn StatusBar(notes: Signal<Vec<Note>>, active_idx: Signal<Option<usize>>) -> Element {
    let stats = use_memo(move || {
        let idx  = (active_idx)()?;
        let ns   = notes.read();
        let note = ns.get(idx)?;
        let words = note.content.split_whitespace().count();
        let tags  = note.tags.len();
        Some((words, tags))
    });

    rsx! {
        if let Some((words, tags)) = (stats)() {
            span { "{words} words" }
            span { "{tags} tags" }
        } else {
            span { "No note" }
        }
    }
}
