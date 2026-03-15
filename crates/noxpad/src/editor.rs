//! Main editor pane: NoteEditor, TabStrip, ModeBar, SuggestMarkdownEditor, SlashList, TagBar.

use crate::models::Note;
use crate::utils::*;
use dioxus::prelude::*;
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    DragContextProvider, DragId, DragOverlay, ReorderEvent, SortableContext, SortableItem,
};
use dioxus_nox_markdown::markdown;
use dioxus_nox_markdown::prelude::Mode;
use dioxus_nox_markdown::types::{ActiveBlockInputEvent, LivePreviewVariant};
use dioxus_nox_suggest::{TriggerConfig, TriggerSelectEvent, suggest, use_suggestion};

#[component]
pub(crate) fn NoteEditor(
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
    let inline_input_read: ReadSignal<(String, usize)> = inline_input.into();
    let mut pending_cursor: Signal<Option<usize>> = use_signal(|| None);

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
                        TriggerConfig { line_start_only: false, ..TriggerConfig::slash() },
                        TriggerConfig::mention(),
                        TriggerConfig::hashtag(),
                    ],
                    on_select: move |evt: TriggerSelectEvent| {
                        let replacement = cmd_to_text(evt.trigger_char, &evt.value);
                        if replacement.is_empty() {
                            return;
                        }

                        // trigger_offset is always a full-doc byte offset (both
                        // Source mode and Inline mode now pass full doc text), so
                        // replace_trigger_range works directly on the full content.
                        let old_text = content.read().clone();
                        let new_text = replace_trigger_range(
                            &old_text,
                            evt.trigger_offset,
                            evt.trigger_char,
                            &evt.filter,
                            &replacement,
                        );

                        // Place cursor right after the inserted replacement text
                        let cursor_pos = evt.trigger_offset + replacement.len();
                        pending_cursor.set(Some(cursor_pos));

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

                        SuggestMarkdownEditor {
                            content,
                            mode,
                            notes,
                            current_idx,
                            inline_input,
                            pending_cursor,
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
fn TabStrip(
    notes: Signal<Vec<Note>>,
    tabs: Signal<Vec<usize>>,
    active_idx: Signal<Option<usize>>,
) -> Element {
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

/// Renders the markdown editor with suggestion popover key interception.
///
/// Must be a child of `suggest::Root` so `use_suggestion()` can access the context.
#[component]
fn SuggestMarkdownEditor(
    content: Signal<String>,
    mode: Signal<Mode>,
    notes: Signal<Vec<Note>>,
    current_idx: Signal<usize>,
    inline_input: Signal<(String, usize)>,
    pending_cursor: Signal<Option<usize>>,
) -> Element {
    let sg = use_suggestion();
    let key_intercept = Callback::new(move |key: String| sg.handle_keydown(&key));

    rsx! {
        markdown::Root {
            value: Some(content),
            mode: Some(mode),
            live_preview_variant: LivePreviewVariant::Inline,
            pending_cursor: Some(pending_cursor),
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
                    // Pass full document text + absolute cursor to
                    // suggest::Trigger so line_start_only detection
                    // works correctly in inline mode.
                    let full_raw = content.read().clone();
                    let prefix_len = evt.block_start.min(full_raw.len());
                    let prefix_utf16 = full_raw[..prefix_len].encode_utf16().count();
                    let abs_cursor_utf16 = prefix_utf16 + evt.cursor_raw_utf16;
                    inline_input.set((full_raw, abs_cursor_utf16));
                },
                on_key_intercept: key_intercept,
            }
        }
    }
}

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
