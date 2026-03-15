//! Utility functions: drag ID management, reordering, tab management, text editing.

use crate::models::{FolderNode, Note};
use dioxus_nox_dnd::DragId;

pub(crate) const FOLDER_TREE_ID: &str = "folder-tree";
pub(crate) const FOLDER_DRAG_PREFIX: &str = "folder:";
pub(crate) const FOLDER_NOTES_PREFIX: &str = "folder-notes:";
pub(crate) const NOTE_DRAG_PREFIX: &str = "note:";
pub(crate) const TAB_STRIP_ID: &str = "tab-strip";
pub(crate) const TAB_DRAG_PREFIX: &str = "tab:";
pub(crate) const NOTE_DRAG_TYPE: &str = "note";
pub(crate) const FOLDER_DRAG_TYPE: &str = "folder";

/// Compute the replacement text for a slash/mention/hashtag command.
pub(crate) fn cmd_to_text(trigger_char: char, value: &str) -> String {
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

pub(crate) fn folder_drag_id(folder_id: &str) -> DragId {
    DragId::new(format!("{FOLDER_DRAG_PREFIX}{folder_id}"))
}

pub(crate) fn folder_notes_container_id(folder_id: &str) -> String {
    format!("{FOLDER_NOTES_PREFIX}{folder_id}")
}

pub(crate) fn note_drag_id(note_idx: usize) -> DragId {
    DragId::new(format!("{NOTE_DRAG_PREFIX}{note_idx}"))
}

pub(crate) fn tab_drag_id(note_idx: usize) -> DragId {
    DragId::new(format!("{TAB_DRAG_PREFIX}{note_idx}"))
}

pub(crate) fn parse_note_drag_id(id: &DragId) -> Option<usize> {
    id.0.strip_prefix(NOTE_DRAG_PREFIX)?.parse::<usize>().ok()
}

pub(crate) fn parse_folder_notes_container_id(container_id: &str) -> Option<&str> {
    container_id.strip_prefix(FOLDER_NOTES_PREFIX)
}

pub(crate) fn normalize_container_id(id: &DragId) -> &str {
    id.0.strip_suffix("-container").unwrap_or(&id.0)
}

pub(crate) fn reorder_in_vec<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= items.len() || to >= items.len() || from == to {
        return false;
    }
    let item = items.remove(from);
    items.insert(to, item);
    true
}

pub(crate) fn reorder_folder_notes(
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

pub(crate) fn move_note_between_folders(
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

pub(crate) fn ensure_tab_open(tabs: &mut Vec<usize>, note_idx: usize) {
    if !tabs.contains(&note_idx) {
        tabs.push(note_idx);
    }
}

pub(crate) fn close_tab(
    tabs: &mut Vec<usize>,
    active: Option<usize>,
    closing: usize,
) -> Option<usize> {
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

pub(crate) fn replace_trigger_range(
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

pub(crate) fn note_by_index(notes: &[Note], idx: usize) -> Option<&Note> {
    notes.get(idx)
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
