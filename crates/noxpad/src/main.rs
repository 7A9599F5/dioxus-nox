//! NoxPad — Full 6-crate demo for dioxus-nox.
//!
//! A Notion/Obsidian-style markdown notes editor demonstrating:
//! - `dioxus-nox-shell`     — 3-pane AppShell layout
//! - `dioxus-nox-markdown`  — InlineEditor with slash commands
//! - `dioxus-nox-suggest`   — /, @, # inline triggers
//! - `dioxus-nox-cmdk`      — Ctrl+K command palette
//! - `dioxus-nox-tag-input` — tag pills
//! - `dioxus-nox-dnd`       — tree/tab drag-to-reorder
//! - `dioxus-nox-preview`   — debounced preview in palette
//!
//! Run with: dx serve -p noxpad

#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandEmpty, CommandGroup, CommandHighlight, CommandInput, CommandItem, CommandList,
    CommandRoot, use_command_palette,
};
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    DragContextProvider, DragId, DragOverlay, DragType, MoveEvent, ReorderEvent, SortableContext,
    SortableGroup, SortableItem, FEEDBACK_STYLES, FUNCTIONAL_STYLES,
};
use dioxus_nox_markdown::markdown;
use dioxus_nox_markdown::prelude::{Mode, generate_theme_css};
use dioxus_nox_markdown::types::{ActiveBlockInputEvent, LivePreviewVariant};
use dioxus_nox_preview::{use_debounced_active, use_preview_cache};
use dioxus_nox_shell::AppShell;
use dioxus_nox_suggest::{TriggerConfig, TriggerSelectEvent, suggest, use_suggestion};

const FOLDER_TREE_ID: &str = "folder-tree";
const FOLDER_DRAG_PREFIX: &str = "folder:";
const FOLDER_NOTES_PREFIX: &str = "folder-notes:";
const NOTE_DRAG_PREFIX: &str = "note:";
const TAB_STRIP_ID: &str = "tab-strip";
const TAB_DRAG_PREFIX: &str = "tab:";
const NOTE_DRAG_TYPE: &str = "note";
const FOLDER_DRAG_TYPE: &str = "folder";

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
       background: #0f0f0f; color: #e0e0e0; height: 100vh; overflow: hidden; }

/* Shell layout */
[data-shell] { display: grid; height: 100vh; overflow: hidden;
    grid-template-rows: 1fr auto;
    grid-template-columns: 280px 1fr 280px;
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

/* Sidebar tree */
.sidebar-header { padding: 12px 16px; font-weight: 600; font-size: 12px;
    color: #888; text-transform: uppercase; letter-spacing: 0.05em; }
.sidebar-tree { padding: 8px; display: flex; flex-direction: column; gap: 6px; }
.folder-node { border: 1px solid #242424; border-radius: 8px; background: #151515; }
.folder-header { display: flex; align-items: center; gap: 8px; padding: 6px 8px;
    font-size: 13px; color: #bbb; border-bottom: 1px solid #1f1f1f; }
.folder-header button { border: none; background: transparent; color: inherit; cursor: pointer; }
.folder-handle { color: #666; cursor: grab; user-select: none; }
.folder-name { font-weight: 600; }
.folder-count { margin-left: auto; font-size: 11px; color: #6f6f6f; }
.folder-notes { padding: 4px; display: flex; flex-direction: column; gap: 2px; }
.note-item { padding: 7px 8px; cursor: pointer; border-left: 2px solid transparent;
    transition: background 0.12s; font-size: 13px; border-radius: 6px;
    display: flex; align-items: center; gap: 4px; }
.drag-handle { cursor: grab; opacity: 0.3; font-size: 10px; flex-shrink: 0; }
.drag-handle:hover { opacity: 0.7; }
.note-item:hover { background: #202020; }
.note-item[data-active="true"] { border-left-color: #7c6af7; background: #23203a; color: #fff; }
.note-item-title { font-weight: 500; }
.note-item-tags { font-size: 11px; color: #666; margin-top: 1px; }
.empty-folder { padding: 8px; color: #555; font-size: 12px; }

/* Editor */
.note-editor { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.note-editor-header { padding: 12px 24px 0; }
.note-title { font-size: 22px; font-weight: 700; background: transparent; border: none;
    color: #e0e0e0; width: 100%; outline: none; padding: 0; }

.tab-strip { display: flex; gap: 4px; padding: 8px 16px 0; border-bottom: 1px solid #1f1f1f;
    min-height: 38px; align-items: center; overflow-x: auto; }
.tab-item { display: inline-flex; align-items: center; gap: 8px; padding: 6px 10px;
    border-radius: 6px 6px 0 0; border: 1px solid #2b2b2b; border-bottom: none;
    color: #999; background: #141414; cursor: pointer; white-space: nowrap; }
.tab-item[data-active="true"] { color: #fff; background: #1d1d1d; border-color: #3a335f; }
.tab-drag-handle { cursor: grab; opacity: 0.3; font-size: 10px; }
.tab-drag-handle:hover { opacity: 0.7; }
.tab-close { border: none; background: transparent; color: #666; cursor: pointer; font-size: 13px; }
.tab-close:hover { color: #d97b7b; }

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
[role="article"][data-md-mode="read"] { flex: 1; padding: 12px 0; overflow-y: auto;
    line-height: 1.7; font-size: 15px; }
[data-md-mode="read"] h1 { font-size: 1.8em; font-weight: 700; margin: 0.8em 0 0.4em; }
[data-md-mode="read"] h2 { font-size: 1.4em; font-weight: 600; margin: 0.7em 0 0.3em; }
[data-md-mode="read"] h3 { font-size: 1.2em; font-weight: 600; margin: 0.6em 0 0.2em; }
[data-md-mode="read"] p { margin: 0.4em 0; }
[data-md-mode="read"] code { background: #222; padding: 1px 4px; border-radius: 3px;
    font-family: monospace; font-size: 0.9em; }
[data-md-mode="read"] pre { background: #1a1a1a; padding: 12px; border-radius: 6px; margin: 8px 0; }
[data-md-mode="read"] blockquote { border-left: 3px solid #444; padding-left: 12px;
    color: #888; margin: 8px 0; }
[data-md-mode="read"] ul, [data-md-mode="read"] ol { padding-left: 20px; margin: 4px 0; }
[data-md-mode="read"] hr { border: none; border-top: 1px solid #2a2a2a; margin: 16px 0; }
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
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Clone)]
struct FolderNode {
    id: String,
    name: String,
    note_indices: Vec<usize>,
    collapsed: bool,
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
            content: "# Meeting Notes - Component API Review\n\n## Attendees\n\n@Alice @Bob @Carol\n\n## Action Items\n\n- [ ] Alice: update cmdk signal pattern\n- [ ] Bob: write migration guide\n- [ ] Carol: add E2E tests for dnd\n\n## Notes\n\nDecided to use `data-state` attributes consistently across all crates.".into(),
            tags: vec!["meeting".into(), "planning".into()],
        },
        Note {
            id: "reading-list".into(),
            title: "Reading List".into(),
            content: "# Reading List\n\n## In Progress\n\n- *Programming Rust* - Blandy & Orendorff\n- *The Rust Programming Language* - Klabnik & Nichols\n\n## Backlog\n\n- *Crafting Interpreters* - Robert Nystrom\n- *Database Internals* - Alex Petrov\n\n## Completed\n\n- *The Art of Problem Solving*\n\n#rust #books #learning".into(),
            tags: vec!["books".into(), "learning".into(), "rust".into()],
        },
    ]
}

fn seed_folders() -> Vec<FolderNode> {
    vec![
        FolderNode {
            id: "inbox".into(),
            name: "Inbox".into(),
            note_indices: vec![0, 2],
            collapsed: false,
        },
        FolderNode {
            id: "engineering".into(),
            name: "Engineering".into(),
            note_indices: vec![1],
            collapsed: false,
        },
        FolderNode {
            id: "reference".into(),
            name: "Reference".into(),
            note_indices: vec![3],
            collapsed: false,
        },
    ]
}

/// Compute the replacement text for a slash/mention/hashtag command.
fn cmd_to_text(trigger_char: char, value: &str) -> String {
    match trigger_char {
        '/' => match value {
            "h1" => "# ".into(),
            "h2" => "## ".into(),
            "h3" => "### ".into(),
            "bold" => "**text**".into(),
            "italic" => "_text_".into(),
            "code" => "`code`".into(),
            "quote" => "> ".into(),
            "task" => "- [ ] ".into(),
            "table" => "| Col 1 | Col 2 |\n|-------|-------|\n|       |       |".into(),
            "divider" => "---".into(),
            _ => String::new(),
        },
        '@' => format!("@{value} "),
        '#' => format!("#{value} "),
        _ => String::new(),
    }
}

fn folder_drag_id(folder_id: &str) -> DragId {
    DragId::new(format!("{FOLDER_DRAG_PREFIX}{folder_id}"))
}

fn folder_notes_container_id(folder_id: &str) -> String {
    format!("{FOLDER_NOTES_PREFIX}{folder_id}")
}

fn note_drag_id(note_idx: usize) -> DragId {
    DragId::new(format!("{NOTE_DRAG_PREFIX}{note_idx}"))
}

fn tab_drag_id(note_idx: usize) -> DragId {
    DragId::new(format!("{TAB_DRAG_PREFIX}{note_idx}"))
}

fn parse_note_drag_id(id: &DragId) -> Option<usize> {
    id.0.strip_prefix(NOTE_DRAG_PREFIX)?.parse::<usize>().ok()
}

fn parse_folder_notes_container_id(container_id: &str) -> Option<&str> {
    container_id.strip_prefix(FOLDER_NOTES_PREFIX)
}

fn normalize_container_id(id: &DragId) -> &str {
    id.0.strip_suffix("-container").unwrap_or(&id.0)
}

fn reorder_in_vec<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= items.len() || to >= items.len() || from == to {
        return false;
    }
    let item = items.remove(from);
    items.insert(to, item);
    true
}

fn reorder_folder_notes(
    folders: &mut [FolderNode],
    folder_id: &str,
    from: usize,
    to: usize,
) -> bool {
    let Some(folder) = folders.iter_mut().find(|folder| folder.id == folder_id) else {
        return false;
    };
    reorder_in_vec(&mut folder.note_indices, from, to)
}

fn move_note_between_folders(
    folders: &mut [FolderNode],
    from_folder: &str,
    to_folder: &str,
    note_idx: usize,
    to_index: usize,
) -> bool {
    if from_folder == to_folder {
        let Some(folder) = folders.iter_mut().find(|folder| folder.id == from_folder) else {
            return false;
        };
        let Some(from_index) = folder.note_indices.iter().position(|idx| *idx == note_idx) else {
            return false;
        };
        if from_index == to_index {
            return false;
        }
        let item = folder.note_indices.remove(from_index);
        let insert_at = to_index.min(folder.note_indices.len());
        folder.note_indices.insert(insert_at, item);
        return true;
    }

    let source_idx = folders.iter().position(|folder| folder.id == from_folder);
    let target_idx = folders.iter().position(|folder| folder.id == to_folder);
    let (Some(source_idx), Some(target_idx)) = (source_idx, target_idx) else {
        return false;
    };

    let Some(source_pos) = folders[source_idx]
        .note_indices
        .iter()
        .position(|idx| *idx == note_idx)
    else {
        return false;
    };

    folders[source_idx].note_indices.remove(source_pos);

    if let Some(existing) = folders[target_idx]
        .note_indices
        .iter()
        .position(|idx| *idx == note_idx)
    {
        folders[target_idx].note_indices.remove(existing);
    }

    let insert_at = to_index.min(folders[target_idx].note_indices.len());
    folders[target_idx].note_indices.insert(insert_at, note_idx);
    true
}

fn ensure_tab_open(tabs: &mut Vec<usize>, note_idx: usize) {
    if !tabs.contains(&note_idx) {
        tabs.push(note_idx);
    }
}

fn close_tab(tabs: &mut Vec<usize>, active: Option<usize>, closing: usize) -> Option<usize> {
    let Some(closing_pos) = tabs.iter().position(|idx| *idx == closing) else {
        return active;
    };

    tabs.remove(closing_pos);
    if tabs.is_empty() {
        return None;
    }

    match active {
        Some(current) if current == closing => {
            if closing_pos > 0 {
                Some(tabs[closing_pos - 1])
            } else {
                Some(tabs[0])
            }
        }
        Some(current) => tabs
            .contains(&current)
            .then_some(current)
            .or_else(|| tabs.first().copied()),
        None => tabs.first().copied(),
    }
}

fn replace_trigger_range(
    text: &str,
    trigger_offset: usize,
    trigger_char: char,
    filter: &str,
    replacement: &str,
) -> String {
    let start = trigger_offset.min(text.len());
    let end = start
        .saturating_add(trigger_char.len_utf8())
        .saturating_add(filter.len())
        .min(text.len());
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

fn replace_in_active_block(
    full_text: &str,
    block: &ActiveBlockInputEvent,
    event: &TriggerSelectEvent,
    replacement: &str,
) -> Option<String> {
    let block_start = block.block_start.min(full_text.len());
    let block_end = block.block_end.min(full_text.len());
    if block_end < block_start {
        return None;
    }

    let replaced_block = replace_trigger_range(
        &block.raw_text,
        event.trigger_offset,
        event.trigger_char,
        &event.filter,
        replacement,
    );

    Some(format!(
        "{}{}{}",
        &full_text[..block_start],
        replaced_block,
        &full_text[block_end..]
    ))
}

fn note_by_index(notes: &[Note], idx: usize) -> Option<&Note> {
    notes.get(idx)
}

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let notes: Signal<Vec<Note>> = use_signal(seed_notes);
    let folders: Signal<Vec<FolderNode>> = use_signal(seed_folders);
    let active_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let tabs: Signal<Vec<usize>> = use_signal(|| vec![0]);
    let mode: Signal<Mode> = use_signal(|| Mode::LivePreview);
    let mut search_open: Signal<bool> = use_signal(|| false);
    let search_read: ReadSignal<bool> = search_open.into();
    let highlight_css: &'static str = use_hook(|| {
        &*Box::leak(generate_theme_css("base16-ocean.dark", "hl-").unwrap_or_default().into_boxed_str())
    });

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {FEEDBACK_STYLES} }
        style { {CSS} }
        style { {highlight_css} }
        AppShell {
            search_active: Some(search_read),
            on_search_change: move |v| {
                search_open.set(v);
            },
            sidebar: rsx! { NoteSidebar { notes, folders, active_idx, tabs } },
            search: rsx! { CmdkPalette { notes, active_idx, tabs, search_open } },
            preview: rsx! { PreviewPane { notes, active_idx } },
            footer: rsx! { StatusBar { notes, active_idx } },

            if (active_idx)().is_some() {
                NoteEditor { notes, active_idx, tabs, mode }
            } else {
                div { style: "padding:32px; color:#444", "Select a note to begin editing" }
            }
        }
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

#[component]
fn NoteSidebar(
    notes: Signal<Vec<Note>>,
    folders: Signal<Vec<FolderNode>>,
    active_idx: Signal<Option<usize>>,
    tabs: Signal<Vec<usize>>,
) -> Element {
    let folders_snapshot = folders.read().clone();
    let notes_snapshot = notes.read().clone();
    let folder_drag_ids: Vec<DragId> = folders_snapshot
        .iter()
        .map(|folder| folder_drag_id(&folder.id))
        .collect();

    rsx! {
        div {
            div { class: "sidebar-header", "Folders" }
            div { class: "sidebar-tree",
                SortableGroup {
                    on_reorder: move |evt: ReorderEvent| {
                        let container_id = normalize_container_id(&evt.container_id);
                        if container_id == FOLDER_TREE_ID {
                            let mut folder_state = folders.write();
                            reorder_in_vec(&mut folder_state, evt.from_index, evt.to_index);
                            return;
                        }

                        if let Some(folder_id) = parse_folder_notes_container_id(container_id) {
                            let mut folder_state = folders.write();
                            reorder_folder_notes(&mut folder_state, folder_id, evt.from_index, evt.to_index);
                        }
                    },
                    on_move: move |evt: MoveEvent| {
                        let from_container = normalize_container_id(&evt.from_container);
                        let to_container = normalize_container_id(&evt.to_container);

                        let Some(from_folder) = parse_folder_notes_container_id(from_container) else {
                            return;
                        };
                        let Some(to_folder) = parse_folder_notes_container_id(to_container) else {
                            return;
                        };
                        let Some(note_idx) = parse_note_drag_id(&evt.item_id) else {
                            return;
                        };

                        let mut folder_state = folders.write();
                        move_note_between_folders(
                            &mut folder_state,
                            from_folder,
                            to_folder,
                            note_idx,
                            evt.to_index,
                        );
                    },

                    SortableContext {
                        id: DragId::new(FOLDER_TREE_ID),
                        items: folder_drag_ids,
                        orientation: Orientation::Vertical,
                        accepts: vec![DragType::new(FOLDER_DRAG_TYPE)],

                        for folder in folders_snapshot.iter() {
                            {
                                let folder_id = folder.id.clone();
                                let folder_name = folder.name.clone();
                                let note_indices = folder.note_indices.clone();
                                let collapsed = folder.collapsed;
                                let note_drag_ids: Vec<DragId> = note_indices
                                    .iter()
                                    .map(|note_idx| note_drag_id(*note_idx))
                                    .collect();

                                rsx! {
                                    SortableItem {
                                        key: "{folder_id}",
                                        id: folder_drag_id(&folder_id),
                                        drag_type: Some(DragType::new(FOLDER_DRAG_TYPE)),
                                        handle: Some(".folder-handle".to_string()),

                                        div { class: "folder-node",
                                            div { class: "folder-header",
                                                span { class: "folder-handle", "::" }
                                                button {
                                                    onclick: move |_| {
                                                        let mut folder_state = folders.write();
                                                        if let Some(folder) = folder_state.iter_mut().find(|folder| folder.id == folder_id) {
                                                            folder.collapsed = !folder.collapsed;
                                                        }
                                                    },
                                                    if collapsed { ">" } else { "v" }
                                                }
                                                span { class: "folder-name", "{folder_name}" }
                                                span { class: "folder-count", "{note_indices.len()}" }
                                            }

                                            if !collapsed {
                                                SortableContext {
                                                    id: DragId::new(folder_notes_container_id(&folder_id)),
                                                    items: note_drag_ids,
                                                    orientation: Orientation::Vertical,
                                                    accepts: vec![DragType::new(NOTE_DRAG_TYPE)],

                                                    div { class: "folder-notes",
                                                        for note_idx in note_indices.iter().copied() {
                                                            {
                                                                let is_active = (active_idx)() == Some(note_idx);
                                                                let note = note_by_index(&notes_snapshot, note_idx);
                                                                let title = note
                                                                    .map(|note| note.title.clone())
                                                                    .unwrap_or_else(|| "Missing note".to_string());
                                                                let tags_preview = note
                                                                    .map(|note| note.tags.iter().take(3).cloned().collect::<Vec<_>>().join(", "))
                                                                    .unwrap_or_default();

                                                                rsx! {
                                                                    SortableItem {
                                                                        key: "note-{note_idx}",
                                                                        id: note_drag_id(note_idx),
                                                                        drag_type: Some(DragType::new(NOTE_DRAG_TYPE)),
                                                                        handle: Some("[data-drag-handle]".to_string()),

                                                                        div {
                                                                            class: "note-item",
                                                                            "data-active": if is_active { "true" } else { "false" },
                                                                            onclick: move |_| {
                                                                                active_idx.set(Some(note_idx));
                                                                                let mut tab_state = tabs.write();
                                                                                ensure_tab_open(&mut tab_state, note_idx);
                                                                            },
                                                                            span { "data-drag-handle": "", class: "drag-handle", "⠿" }
                                                                            div { class: "note-item-title", "{title}" }
                                                                            if !tags_preview.is_empty() {
                                                                                div { class: "note-item-tags", "{tags_preview}" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        if note_indices.is_empty() {
                                                            div { class: "empty-folder", "Drop notes here" }
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

                    DragOverlay {
                        div { class: "tag-drag-overlay",
                            span { class: "tag-pill", "Moving" }
                        }
                    }
                }
            }
        }
    }
}

// ── Editor ────────────────────────────────────────────────────────────────────

#[component]
fn NoteEditor(
    notes: Signal<Vec<Note>>,
    active_idx: Signal<Option<usize>>,
    tabs: Signal<Vec<usize>>,
    mode: Signal<Mode>,
) -> Element {
    let Some(initial_idx) = (active_idx)() else {
        return rsx! {
            div { style: "padding:32px; color:#444", "Select a note to begin editing" }
        };
    };

    let mut current_idx: Signal<usize> = use_signal(|| initial_idx);
    let mut content: Signal<String> = use_signal(|| {
        notes
            .peek()
            .get(initial_idx)
            .map(|note| note.content.clone())
            .unwrap_or_default()
    });

    let mut inline_input: Signal<(String, usize)> = use_signal(|| (String::new(), 0));
    let mut active_block_input: Signal<Option<ActiveBlockInputEvent>> = use_signal(|| None);
    let inline_input_read: ReadSignal<(String, usize)> = inline_input.into();

    use_effect(move || {
        if let Some(idx) = (active_idx)() {
            current_idx.set(idx);
            let mut tab_state = tabs.write();
            ensure_tab_open(&mut tab_state, idx);
        }
    });

    use_effect(move || {
        let idx = *current_idx.read();
        let next_content = notes
            .peek()
            .get(idx)
            .map(|note| note.content.clone())
            .unwrap_or_default();
        content.set(next_content);
        inline_input.set((String::new(), 0));
        active_block_input.set(None);
    });

    let note_title = notes
        .read()
        .get(*current_idx.read())
        .map(|note| note.title.clone())
        .unwrap_or_default();

    let all_tags: Vec<String> = {
        let notes_read = notes.read();
        let mut tags: Vec<String> = notes_read
            .iter()
            .flat_map(|note| note.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    };
    let people = vec!["Alice", "Bob", "Carol", "Dan", "Eve"];

    rsx! {
        div { class: "note-editor",
            TabStrip { notes, tabs, active_idx }
            ModeBar { mode }

            div { class: "note-editor-header",
                input {
                    class: "note-title",
                    value: "{note_title}",
                    oninput: move |evt| {
                        let value = evt.value();
                        let idx = *current_idx.read();
                        let mut note_state = notes.write();
                        if let Some(note) = note_state.get_mut(idx) {
                            note.title = value;
                        }
                    }
                }
            }

            if (mode)() == Mode::Read {
                markdown::Root {
                    value: Some(content),
                    mode: Some(mode),
                    live_preview_variant: LivePreviewVariant::Inline,
                    markdown::Content {}
                }
            } else {
                suggest::Root {
                    triggers: vec![
                        TriggerConfig::slash(),
                        TriggerConfig::mention(),
                        TriggerConfig::hashtag(),
                    ],
                    on_select: move |evt: TriggerSelectEvent| {
                        let replacement = cmd_to_text(evt.trigger_char, &evt.value);
                        if replacement.is_empty() {
                            return;
                        }

                        let old_text = content.read().clone();
                        let new_text = if let Some(block) = active_block_input.read().as_ref() {
                            replace_in_active_block(&old_text, block, &evt, &replacement)
                                .unwrap_or_else(|| {
                                    replace_trigger_range(
                                        &old_text,
                                        evt.trigger_offset,
                                        evt.trigger_char,
                                        &evt.filter,
                                        &replacement,
                                    )
                                })
                        } else {
                            replace_trigger_range(
                                &old_text,
                                evt.trigger_offset,
                                evt.trigger_char,
                                &evt.filter,
                                &replacement,
                            )
                        };

                        content.set(new_text.clone());
                        let idx = *current_idx.read();
                        let mut note_state = notes.write();
                        if let Some(note) = note_state.get_mut(idx) {
                            note.content = new_text;
                        }
                    },

                    suggest::Trigger {
                        external_input: if (mode)() == Mode::LivePreview {
                            Some(inline_input_read)
                        } else {
                            None
                        },
                        style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",

                        markdown::Root {
                            value: Some(content),
                            mode: Some(mode),
                            live_preview_variant: LivePreviewVariant::Inline,
                            on_value_change: move |value: String| {
                                content.set(value.clone());
                                let idx = *current_idx.read();
                                let mut note_state = notes.write();
                                if let Some(note) = note_state.get_mut(idx) {
                                    note.content = value;
                                }
                            },

                            markdown::Editor {
                                on_active_block_input: move |evt: ActiveBlockInputEvent| {
                                    inline_input
                                        .set((evt.raw_text.clone(), evt.cursor_raw_utf16));
                                    active_block_input.set(Some(evt));
                                }
                            }
                        }
                    }

                    SlashList { all_tags, people }
                }
            }

            TagBar {
                notes,
                active_idx: *current_idx.read()
            }
        }
    }
}

#[component]
fn TabStrip(notes: Signal<Vec<Note>>, tabs: Signal<Vec<usize>>, active_idx: Signal<Option<usize>>) -> Element {
    let tabs_snapshot = tabs.read().clone();
    let notes_snapshot = notes.read().clone();
    let tab_ids: Vec<DragId> = tabs_snapshot
        .iter()
        .map(|note_idx| tab_drag_id(*note_idx))
        .collect();

    rsx! {
        SortableContext {
            id: DragId::new(TAB_STRIP_ID),
            items: tab_ids,
            orientation: Orientation::Horizontal,
            class: "tab-strip",
            on_reorder: move |evt: ReorderEvent| {
                let mut tab_state = tabs.write();
                reorder_in_vec(&mut tab_state, evt.from_index, evt.to_index);
            },

            for note_idx in tabs_snapshot.iter().copied() {
                {
                    let title = note_by_index(&notes_snapshot, note_idx)
                        .map(|note| note.title.clone())
                        .unwrap_or_else(|| "Missing note".to_string());
                    let is_active = (active_idx)() == Some(note_idx);

                    rsx! {
                        SortableItem {
                            key: "tab-{note_idx}",
                            id: tab_drag_id(note_idx),
                            handle: Some("[data-drag-handle]".to_string()),

                            div {
                                class: "tab-item",
                                "data-active": if is_active { "true" } else { "false" },
                                onclick: move |_| {
                                    active_idx.set(Some(note_idx));
                                    let mut tab_state = tabs.write();
                                    ensure_tab_open(&mut tab_state, note_idx);
                                },
                                span { "data-drag-handle": "", class: "tab-drag-handle", "⠿" }
                                span { "{title}" }
                                button {
                                    class: "tab-close",
                                    onclick: move |evt: MouseEvent| {
                                        evt.stop_propagation();
                                        let mut tab_state = tabs.write();
                                        let next_active = close_tab(&mut tab_state, (active_idx)(), note_idx);
                                        active_idx.set(next_active);
                                    },
                                    "x"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── ModeBar ───────────────────────────────────────────────────────────────────

#[component]
fn ModeBar(mode: Signal<Mode>) -> Element {
    let current_mode = (mode)();
    rsx! {
        div { class: "mode-bar",
            button {
                class: "mode-tab",
                "data-active": if current_mode == Mode::Source { "true" } else { "false" },
                onclick: move |_| {
                    mode.set(Mode::Source);
                },
                "Source"
            }
            button {
                class: "mode-tab",
                "data-active": if current_mode == Mode::LivePreview { "true" } else { "false" },
                onclick: move |_| {
                    mode.set(Mode::LivePreview);
                },
                "Inline"
            }
            button {
                class: "mode-tab",
                "data-active": if current_mode == Mode::Read { "true" } else { "false" },
                onclick: move |_| {
                    mode.set(Mode::Read);
                },
                "Read"
            }
        }
    }
}

// ── SlashList ─────────────────────────────────────────────────────────────────

#[component]
fn SlashList(all_tags: Vec<String>, people: Vec<&'static str>) -> Element {
    let suggestion = use_suggestion();
    let filter = suggestion.filter();
    let trigger = suggestion.active_char();

    rsx! {
        suggest::List {
            if trigger == Some('/') {
                {
                    let slash_cmds: &[(&str, &str, &str)] = &[
                        ("h1", "Heading 1", "H1"),
                        ("h2", "Heading 2", "H2"),
                        ("h3", "Heading 3", "H3"),
                        ("bold", "Bold", "B"),
                        ("italic", "Italic", "I"),
                        ("code", "Inline Code", "</>"),
                        ("quote", "Blockquote", ">"),
                        ("task", "Task Item", "[]"),
                        ("table", "Table", "Tbl"),
                        ("divider", "Divider", "---"),
                    ];
                    let normalized = filter.to_lowercase();
                    rsx! {
                        for &(value, label, icon) in slash_cmds
                            .iter()
                            .filter(|&&(value, label, _)| {
                                normalized.is_empty()
                                    || value.contains(&normalized)
                                    || label.to_lowercase().contains(&normalized)
                            })
                        {
                            suggest::Item { value: value.to_string(),
                                span { class: "cmd-icon", "{icon}" }
                                "{label}"
                            }
                        }
                    }
                }
            }

            if trigger == Some('@') {
                {
                    let normalized = filter.to_lowercase();
                    rsx! {
                        for &person in people
                            .iter()
                            .filter(|&&person| normalized.is_empty() || person.to_lowercase().contains(&normalized))
                        {
                            suggest::Item { value: person.to_string(),
                                span { class: "cmd-icon", "@" }
                                "{person}"
                            }
                        }
                    }
                }
            }

            if trigger == Some('#') {
                {
                    let normalized = filter.to_lowercase();
                    rsx! {
                        for tag in all_tags
                            .iter()
                            .filter(|tag| normalized.is_empty() || tag.to_lowercase().contains(&normalized))
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
    let tags: Vec<String> = notes
        .read()
        .get(active_idx)
        .map(|note| note.tags.clone())
        .unwrap_or_default();
    let tag_ids: Vec<DragId> = tags.iter().map(|tag| DragId::new(tag.clone())).collect();
    let mut new_tag: Signal<String> = use_signal(String::new);

    rsx! {
        div { class: "tag-bar",
            DragContextProvider {
                SortableContext {
                    id: DragId::new("tags"),
                    items: tag_ids.clone(),
                    orientation: Orientation::Horizontal,
                    on_reorder: move |evt: ReorderEvent| {
                        let mut note_state = notes.write();
                        if let Some(note) = note_state.get_mut(active_idx) {
                            reorder_in_vec(&mut note.tags, evt.from_index, evt.to_index);
                        }
                    },

                    for (idx, tag) in tags.iter().enumerate() {
                        {
                            let tag_value = tag.clone();
                            let drag_id = tag_ids
                                .get(idx)
                                .cloned()
                                .unwrap_or_else(|| DragId::new(tag.clone()));
                            rsx! {
                                SortableItem {
                                    id: drag_id,
                                    span { class: "tag-pill",
                                        "#{tag_value}"
                                        button {
                                            class: "tag-remove",
                                            onclick: move |_| {
                                                let mut note_state = notes.write();
                                                if let Some(note) = note_state.get_mut(active_idx) {
                                                    note.tags.retain(|tag| tag != &tag_value);
                                                }
                                            },
                                            "x"
                                        },
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

            div { class: "tag-input-wrap",
                input {
                    r#type: "text",
                    placeholder: "add tag...",
                    value: "{new_tag}",
                    oninput: move |evt| {
                        new_tag.set(evt.value());
                    },
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key().to_string() == "Enter" {
                            let value = new_tag.read().trim().to_string();
                            if !value.is_empty() {
                                let mut note_state = notes.write();
                                if let Some(note) = note_state.get_mut(active_idx)
                                    && !note.tags.contains(&value)
                                {
                                    note.tags.push(value);
                                }
                                new_tag.set(String::new());
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
    tabs: Signal<Vec<usize>>,
    search_open: Signal<bool>,
) -> Element {
    let palette = use_command_palette(true);

    use_effect(move || {
        let is_open = (palette.open)();
        search_open.set(is_open);
    });

    let mut active_val: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        div {
            onclick: move |_| palette.hide(),
            div {
                class: "cmdk-dialog",
                onclick: move |evt| evt.stop_propagation(),

                CommandRoot {
                    on_active_change: move |value: Option<String>| {
                        active_val.set(value);
                    },
                    on_select: move |value: String| {
                        if let Some(note_id) = value.strip_prefix("note:") {
                            let note_id = note_id.to_string();
                            let notes_read = notes.read();
                            if let Some(index) = notes_read.iter().position(|note| note.id == note_id) {
                                active_idx.set(Some(index));
                                let mut tab_state = tabs.write();
                                ensure_tab_open(&mut tab_state, index);
                            }
                        }
                        palette.hide();
                        search_open.set(false);
                    },

                    div { class: "cmdk-input-wrap",
                        CommandInput { placeholder: "Search notes..." }
                    }
                    div { class: "cmdk-list",
                        CommandList {
                            CommandEmpty {
                                div { class: "cmdk-empty", "No results found." }
                            }
                            CommandGroup { id: "notes", heading: "Notes",
                                for note in notes.read().iter() {
                                    {
                                        let value = format!("note:{}", note.id);
                                        let title = note.title.clone();
                                        let tags = note.tags.join(", ");
                                        rsx! {
                                            CommandItem {
                                                id: value.clone(),
                                                value: value,
                                                label: title.clone(),
                                                div { class: "cmdk-item",
                                                    div { class: "cmdk-item-title",
                                                        CommandHighlight { label: title }
                                                    }
                                                    if !tags.is_empty() {
                                                        div { class: "cmdk-item-tags", "{tags}" }
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
    let cache = use_preview_cache(10);
    let cache_for_effect = cache.clone();

    use_effect(move || {
        let selected = (debounced)();
        let Some(ref value) = selected else {
            return;
        };
        if cache_for_effect.get(value).is_some() {
            return;
        }

        let notes_snapshot = notes.read();
        if let Some(note_id) = value.strip_prefix("note:")
            && let Some(note) = notes_snapshot.iter().find(|note| note.id == note_id)
        {
            let preview_text = note.content.lines().take(6).collect::<Vec<_>>().join("\n");
            let cache_key = value.clone();
            cache_for_effect.insert(
                cache_key,
                Rc::new(move || {
                    let text = preview_text.clone();
                    rsx! { div { class: "preview-content", "{text}" } }
                }),
            );
        }
    });

    let selected = (debounced)();
    let is_loading = (active_val)() != selected;

    rsx! {
        div {
            "data-preview-loading": if is_loading { "true" } else { "false" },
            if let Some(ref value) = selected {
                if let Some(render_fn) = cache.get(value) {
                    div { {(render_fn)()} }
                } else {
                    div { class: "preview-empty", "Loading..." }
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
        notes
            .read()
            .get(idx)
            .map(|note| (note.title.clone(), note.content.clone()))
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
        let idx = (active_idx)()?;
        let notes_read = notes.read();
        let note = notes_read.get(idx)?;
        let words = note.content.split_whitespace().count();
        let tags = note.tags.len();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_vector_moves_item() {
        let mut values = vec![1, 2, 3, 4];
        let changed = reorder_in_vec(&mut values, 1, 3);
        assert!(changed);
        assert_eq!(values, vec![1, 3, 4, 2]);
    }

    #[test]
    fn move_note_between_folders_updates_membership() {
        let mut folders = vec![
            FolderNode {
                id: "a".to_string(),
                name: "A".to_string(),
                note_indices: vec![0, 1],
                collapsed: false,
            },
            FolderNode {
                id: "b".to_string(),
                name: "B".to_string(),
                note_indices: vec![2],
                collapsed: false,
            },
        ];

        let changed = move_note_between_folders(&mut folders, "a", "b", 1, 0);
        assert!(changed);
        assert_eq!(folders[0].note_indices, vec![0]);
        assert_eq!(folders[1].note_indices, vec![1, 2]);
    }

    #[test]
    fn close_tab_selects_previous_when_active_closed() {
        let mut tabs = vec![0, 1, 2];
        let next_active = close_tab(&mut tabs, Some(2), 2);
        assert_eq!(tabs, vec![0, 1]);
        assert_eq!(next_active, Some(1));
    }
}
