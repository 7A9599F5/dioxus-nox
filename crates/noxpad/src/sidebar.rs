//! Folder/note sidebar tree view with drag-to-reorder.

use crate::models::{FolderNode, Note};
use crate::utils::*;
use dioxus::prelude::*;
use dioxus_nox_dnd::types::Orientation;
use dioxus_nox_dnd::{
    DragId, DragOverlay, DragType, MoveEvent, ReorderEvent, SortableContext, SortableGroup,
    SortableItem,
};

#[component]
pub(crate) fn NoteSidebar(
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
