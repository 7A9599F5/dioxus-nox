// Unit tests for datagrid.
// Navigation and editing tests live in their respective modules.
// This file holds integration-level tests.

use crate::types::*;

#[test]
fn column_def_builder() {
    let col = ColumnDef::new("name")
        .header("Full Name")
        .sortable(true)
        .resizable(true)
        .width(200)
        .min_width(100)
        .max_width(500)
        .editable(CellEditor::Text)
        .pinned(PinSide::Left);

    assert_eq!(col.key, "name");
    assert_eq!(col.header_text, "Full Name");
    assert!(col.sortable);
    assert!(col.resizable);
    assert_eq!(col.width, Some(200));
    assert_eq!(col.min_width, Some(100));
    assert_eq!(col.max_width, Some(500));
    assert_eq!(col.editable, Some(CellEditor::Text));
    assert_eq!(col.pinned, Some(PinSide::Left));
}

#[test]
fn column_def_defaults() {
    let col = ColumnDef::new("col1");
    assert_eq!(col.key, "col1");
    assert_eq!(col.header_text, "col1");
    assert!(!col.sortable);
    assert!(!col.resizable);
    assert_eq!(col.editable, None);
    assert_eq!(col.pinned, None);
    assert_eq!(col.width, None);
}

#[test]
fn sort_direction_toggle() {
    assert_eq!(SortDirection::Ascending.toggle(), SortDirection::Descending);
    assert_eq!(SortDirection::Descending.toggle(), SortDirection::Ascending);
}

#[test]
fn sort_direction_data_attr() {
    assert_eq!(SortDirection::Ascending.as_data_attr(), "asc");
    assert_eq!(SortDirection::Descending.as_data_attr(), "desc");
}

#[test]
fn pin_side_data_attr() {
    assert_eq!(PinSide::Left.as_data_attr(), "left");
    assert_eq!(PinSide::Right.as_data_attr(), "right");
}

#[test]
fn grid_state_data_attr() {
    assert_eq!(GridState::Idle.as_data_attr(), "idle");
    assert_eq!(GridState::Editing.as_data_attr(), "editing");
    assert_eq!(GridState::Selecting.as_data_attr(), "selecting");
}

#[test]
fn cell_coord_default() {
    let coord = CellCoord::default();
    assert_eq!(coord.row_idx, 0);
    assert_eq!(coord.col_idx, 0);
}
