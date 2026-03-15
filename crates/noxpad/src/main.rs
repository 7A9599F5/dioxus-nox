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

mod css;
mod editor;
mod models;
mod palette;
mod panels;
mod sidebar;
mod utils;

use dioxus::prelude::*;
use dioxus_nox_dnd::{FEEDBACK_STYLES, FUNCTIONAL_STYLES};
use dioxus_nox_markdown::prelude::{Mode, generate_theme_css};
use dioxus_nox_shell::AppShell;

use crate::css::CSS;
use crate::editor::NoteEditor;
use crate::models::{seed_folders, seed_notes};
use crate::palette::CmdkPalette;
use crate::panels::{PreviewPane, StatusBar};
use crate::sidebar::NoteSidebar;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let notes: Signal<Vec<models::Note>> = use_signal(seed_notes);
    let folders: Signal<Vec<models::FolderNode>> = use_signal(seed_folders);
    let active_idx: Signal<Option<usize>> = use_signal(|| Some(0));
    let tabs: Signal<Vec<usize>> = use_signal(|| vec![0]);
    let mode: Signal<Mode> = use_signal(|| Mode::LivePreview);
    let mut search_open: Signal<bool> = use_signal(|| false);
    let search_read: ReadSignal<bool> = search_open.into();
    let highlight_css: &'static str = use_hook(|| {
        &*Box::leak(
            generate_theme_css("base16-ocean.dark", "hl-")
                .unwrap_or_default()
                .into_boxed_str(),
        )
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
