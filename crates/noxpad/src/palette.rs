//! Command palette (Ctrl+K) with debounced note preview.

use std::rc::Rc;

use crate::models::Note;
use crate::utils::ensure_tab_open;
use dioxus::prelude::*;
use dioxus_nox_cmdk::{
    CommandEmpty, CommandGroup, CommandHighlight, CommandInput, CommandItem, CommandList,
    CommandRoot, use_command_palette,
};
use dioxus_nox_preview::{use_debounced_active, use_preview_cache};

#[component]
pub(crate) fn CmdkPalette(
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
