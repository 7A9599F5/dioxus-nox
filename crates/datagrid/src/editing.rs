use crate::types::{CellCoord, CellEditEvent, CellEditor};

/// Cell editing state.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum EditState {
    /// No cell is being edited.
    #[default]
    Idle,
    /// A cell is actively being edited.
    Active {
        coord: CellCoord,
        editor: CellEditor,
        row_id: String,
        column: String,
        original_value: String,
        current_value: String,
    },
}

impl EditState {
    /// Begin editing a cell.
    pub fn begin(
        coord: CellCoord,
        editor: CellEditor,
        row_id: String,
        column: String,
        current_value: String,
    ) -> Self {
        Self::Active {
            coord,
            editor,
            row_id,
            column,
            original_value: current_value.clone(),
            current_value,
        }
    }

    /// Update the current value during editing.
    pub fn update_value(&mut self, new_value: String) {
        if let Self::Active { current_value, .. } = self {
            *current_value = new_value;
        }
    }

    /// Commit the edit and return the event (if value changed).
    pub fn commit(&self) -> Option<CellEditEvent> {
        if let Self::Active {
            row_id,
            column,
            original_value,
            current_value,
            ..
        } = self
            && original_value != current_value
        {
            return Some(CellEditEvent {
                row_id: row_id.clone(),
                column: column.clone(),
                old_value: original_value.clone(),
                new_value: current_value.clone(),
            });
        }
        None
    }

    /// Cancel editing — returns to Idle.
    pub fn cancel() -> Self {
        Self::Idle
    }

    /// Whether a cell is currently being edited.
    pub fn is_editing(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    /// Get the coordinate of the cell being edited, if any.
    pub fn editing_coord(&self) -> Option<CellCoord> {
        if let Self::Active { coord, .. } = self {
            Some(*coord)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_edit() {
        let state = EditState::begin(
            CellCoord { row_idx: 1, col_idx: 2 },
            CellEditor::Text,
            "row-1".to_string(),
            "name".to_string(),
            "hello".to_string(),
        );
        assert!(state.is_editing());
        assert_eq!(state.editing_coord(), Some(CellCoord { row_idx: 1, col_idx: 2 }));
    }

    #[test]
    fn commit_unchanged_returns_none() {
        let state = EditState::begin(
            CellCoord::default(),
            CellEditor::Text,
            "r1".to_string(),
            "col".to_string(),
            "value".to_string(),
        );
        assert_eq!(state.commit(), None);
    }

    #[test]
    fn commit_changed_returns_event() {
        let mut state = EditState::begin(
            CellCoord::default(),
            CellEditor::Text,
            "r1".to_string(),
            "col".to_string(),
            "old".to_string(),
        );
        state.update_value("new".to_string());
        let event = state.commit().unwrap();
        assert_eq!(event.old_value, "old");
        assert_eq!(event.new_value, "new");
        assert_eq!(event.row_id, "r1");
        assert_eq!(event.column, "col");
    }

    #[test]
    fn cancel_returns_idle() {
        let state = EditState::cancel();
        assert!(!state.is_editing());
        assert_eq!(state.editing_coord(), None);
    }
}
