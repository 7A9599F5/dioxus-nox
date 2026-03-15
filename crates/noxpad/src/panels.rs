//! Leaf display components: PreviewPane and StatusBar.

use crate::models::Note;
use dioxus::prelude::*;

#[component]
pub(crate) fn PreviewPane(notes: Signal<Vec<Note>>, active_idx: Signal<Option<usize>>) -> Element {
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

#[component]
pub(crate) fn StatusBar(notes: Signal<Vec<Note>>, active_idx: Signal<Option<usize>>) -> Element {
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
