use std::collections::HashSet;

use dioxus::prelude::*;

use crate::editing::EditState;
use crate::types::{
    CellCoord, CellEditEvent, CellEditor, ColumnDef, GridState, RowEntry, Selection, SortDirection,
    SortState,
};

/// Shared context for the data grid compound component tree.
///
/// Provided by [`super::datagrid::Root`] and consumed by all sub-components.
/// All fields are `Signal` so this is `Copy`.
#[derive(Clone, Copy)]
pub struct DataGridContext {
    /// Column definitions.
    pub(crate) columns: Signal<Vec<ColumnDef>>,
    /// Current sort state (supports multi-column sorting).
    pub(crate) sort_state: Signal<Vec<SortState>>,
    /// Set of selected row IDs.
    pub(crate) selected_rows: Signal<HashSet<String>>,
    /// Current cell editing state.
    pub(crate) edit_state: Signal<EditState>,
    /// Currently focused cell coordinate.
    pub(crate) focused_cell: Signal<Option<CellCoord>>,
    /// Column widths in pixels (indexed by column order).
    pub(crate) column_widths: Signal<Vec<u32>>,
    /// Column display order (indices into the columns vec).
    pub(crate) column_order: Signal<Vec<usize>>,
    /// Registered rows for navigation.
    pub(crate) rows: Signal<Vec<RowEntry>>,
    /// Row selection mode.
    pub(crate) selection_mode: Selection,
    /// Callback when a cell edit is committed.
    pub(crate) on_cell_edit: Option<EventHandler<CellEditEvent>>,
    /// Callback when sort state changes.
    pub(crate) on_sort_change: Option<EventHandler<Vec<SortState>>>,
    /// Callback when row selection changes.
    pub(crate) on_row_select: Option<EventHandler<HashSet<String>>>,
}

impl DataGridContext {
    // ── Column access ───────────────────────────────────────────────────

    /// Get the number of columns.
    pub fn col_count(&self) -> usize {
        (self.columns)().len()
    }

    /// Get the number of registered rows.
    pub fn row_count(&self) -> usize {
        (self.rows)().len()
    }

    /// Get the display-ordered column index for a given position.
    pub fn column_at(&self, display_idx: usize) -> Option<ColumnDef> {
        let order = (self.column_order)();
        let cols = (self.columns)();
        order.get(display_idx).and_then(|&i| cols.get(i).cloned())
    }

    // ── Sorting ─────────────────────────────────────────────────────────

    /// Toggle sort for a column. If already sorted, toggles direction.
    /// If not sorted, adds ascending sort. Fires `on_sort_change`.
    pub fn toggle_sort(&mut self, column_key: &str) {
        let mut state = self.sort_state.write();
        if let Some(existing) = state.iter_mut().find(|s| s.column == column_key) {
            existing.direction = existing.direction.toggle();
        } else {
            state.push(SortState {
                column: column_key.to_string(),
                direction: SortDirection::Ascending,
            });
        }
        let new_state = state.clone();
        drop(state);
        if let Some(handler) = &self.on_sort_change {
            handler.call(new_state);
        }
    }

    /// Get the sort direction for a column, if sorted.
    pub fn sort_direction(&self, column_key: &str) -> Option<SortDirection> {
        (self.sort_state)()
            .iter()
            .find(|s| s.column == column_key)
            .map(|s| s.direction)
    }

    // ── Selection ───────────────────────────────────────────────────────

    /// Whether a row is selected.
    pub fn is_selected(&self, row_id: &str) -> bool {
        (self.selected_rows)().contains(row_id)
    }

    /// Toggle selection for a row. In single mode, deselects others first.
    pub fn toggle_select(&mut self, row_id: &str) {
        match self.selection_mode {
            Selection::None => return,
            Selection::Single => {
                let mut selected = self.selected_rows.write();
                if selected.contains(row_id) {
                    selected.remove(row_id);
                } else {
                    selected.clear();
                    selected.insert(row_id.to_string());
                }
            }
            Selection::Multi => {
                let mut selected = self.selected_rows.write();
                if selected.contains(row_id) {
                    selected.remove(row_id);
                } else {
                    selected.insert(row_id.to_string());
                }
            }
        }
        let new_selection = (self.selected_rows)();
        if let Some(handler) = &self.on_row_select {
            handler.call(new_selection);
        }
    }

    /// Select all rows (multi-select mode only).
    pub fn select_all(&mut self) {
        if self.selection_mode != Selection::Multi {
            return;
        }
        let mut selected = self.selected_rows.write();
        for row in self.rows.read().iter() {
            if !row.disabled {
                selected.insert(row.id.clone());
            }
        }
        drop(selected);
        let new_selection = (self.selected_rows)();
        if let Some(handler) = &self.on_row_select {
            handler.call(new_selection);
        }
    }

    /// Deselect all rows.
    pub fn deselect_all(&mut self) {
        self.selected_rows.write().clear();
        if let Some(handler) = &self.on_row_select {
            handler.call(HashSet::new());
        }
    }

    // ── Focus ───────────────────────────────────────────────────────────

    /// Get the currently focused cell.
    pub fn focused(&self) -> Option<CellCoord> {
        (self.focused_cell)()
    }

    /// Set the focused cell.
    pub fn set_focused(&mut self, coord: Option<CellCoord>) {
        self.focused_cell.set(coord);
    }

    // ── Editing ─────────────────────────────────────────────────────────

    /// Begin editing a cell.
    pub fn begin_edit(
        &mut self,
        coord: CellCoord,
        editor: CellEditor,
        row_id: String,
        column: String,
        current_value: String,
    ) {
        self.edit_state.set(EditState::begin(
            coord,
            editor,
            row_id,
            column,
            current_value,
        ));
    }

    /// Update the current edit value.
    pub fn update_edit_value(&mut self, value: String) {
        self.edit_state.write().update_value(value);
    }

    /// Commit the current edit and fire callback.
    pub fn commit_edit(&mut self) {
        let event = self.edit_state.read().commit();
        self.edit_state.set(EditState::Idle);
        if let Some(edit_event) = event
            && let Some(handler) = &self.on_cell_edit
        {
            handler.call(edit_event);
        }
    }

    /// Cancel the current edit.
    pub fn cancel_edit(&mut self) {
        self.edit_state.set(EditState::Idle);
    }

    /// Whether any cell is being edited.
    pub fn is_editing(&self) -> bool {
        (self.edit_state)().is_editing()
    }

    /// Get the coordinate of the cell being edited.
    pub fn editing_coord(&self) -> Option<CellCoord> {
        (self.edit_state)().editing_coord()
    }

    // ── Grid state ──────────────────────────────────────────────────────

    /// Current grid state for data attribute.
    pub fn grid_state(&self) -> GridState {
        if self.is_editing() {
            GridState::Editing
        } else if !(self.selected_rows)().is_empty() {
            GridState::Selecting
        } else {
            GridState::Idle
        }
    }

    // ── Row registration ────────────────────────────────────────────────

    /// Register a row. Called on mount.
    pub fn register_row(&mut self, entry: RowEntry) {
        let mut rows = self.rows.write();
        if !rows.iter().any(|r| r.id == entry.id) {
            rows.push(entry);
        }
    }

    /// Deregister a row. Called on unmount.
    pub fn deregister_row(&mut self, row_id: &str) {
        self.rows.write().retain(|r| r.id != row_id);
    }

    // ── Column resize ───────────────────────────────────────────────────

    /// Set column width at the given display index.
    pub fn set_column_width(&mut self, display_idx: usize, width: u32) {
        // Read column constraints before borrowing widths mutably.
        let col = self.column_at(display_idx);
        let min = col.as_ref().and_then(|c| c.min_width).unwrap_or(30);
        let max = col.as_ref().and_then(|c| c.max_width).unwrap_or(u32::MAX);

        let mut widths = self.column_widths.write();
        if display_idx < widths.len() {
            widths[display_idx] = width.clamp(min, max);
        }
    }

    /// Get column width at the given display index.
    pub fn column_width(&self, display_idx: usize) -> Option<u32> {
        (self.column_widths)().get(display_idx).copied()
    }

    // ── Column reorder ──────────────────────────────────────────────────

    /// Reorder columns by moving a column from one display position to another.
    pub fn reorder_column(&mut self, from_idx: usize, to_idx: usize) {
        let mut order = self.column_order.write();
        if from_idx < order.len() && to_idx < order.len() {
            let item = order.remove(from_idx);
            order.insert(to_idx, item);
        }
    }
}
