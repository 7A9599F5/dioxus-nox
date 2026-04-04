use std::collections::HashSet;

use dioxus::prelude::*;

use crate::context::DataGridContext;
use crate::editing::EditState;
use crate::navigation::{navigate_grid, GridNavKey};
use crate::types::{
    CellCoord, CellEditEvent, ColumnDef, RowEntry, Selection, SortState,
};

// ── Root ────────────────────────────────────────────────────────────────────

/// Context provider for the data grid compound component.
///
/// ```text
/// datagrid::Root {
///     columns: vec![ColumnDef::new("name").header("Name").sortable(true)],
///     datagrid::Header {
///         datagrid::HeaderRow {
///             datagrid::HeaderCell { col_index: 0 }
///         }
///     }
///     datagrid::Body {
///         datagrid::Row { row_id: "1",
///             datagrid::Cell { col_index: 0, "Alice" }
///         }
///     }
/// }
/// ```
///
/// ## Data attributes
/// - `data-grid-state="idle|editing|selecting"`
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Column definitions for the grid.
    columns: Vec<ColumnDef>,
    /// Row selection mode.
    #[props(default)]
    selection: Selection,
    /// Callback when a cell edit is committed.
    #[props(default)]
    on_cell_edit: Option<EventHandler<CellEditEvent>>,
    /// Callback when sort state changes.
    #[props(default)]
    on_sort_change: Option<EventHandler<Vec<SortState>>>,
    /// Callback when row selection changes.
    #[props(default)]
    on_row_select: Option<EventHandler<HashSet<String>>>,
    children: Element,
) -> Element {
    let col_count = columns.len();
    let default_widths: Vec<u32> = columns
        .iter()
        .map(|c| c.width.unwrap_or(150))
        .collect();
    let default_order: Vec<usize> = (0..col_count).collect();

    let ctx = DataGridContext {
        columns: use_signal(|| columns),
        sort_state: use_signal(Vec::new),
        selected_rows: use_signal(HashSet::new),
        edit_state: use_signal(EditState::default),
        focused_cell: use_signal(|| None),
        column_widths: use_signal(|| default_widths),
        column_order: use_signal(|| default_order),
        rows: use_signal(Vec::new),
        selection_mode: selection,
        on_cell_edit,
        on_sort_change,
        on_row_select,
    };

    use_context_provider(|| ctx);

    let grid_state = ctx.grid_state();

    rsx! {
        div {
            role: "grid",
            "data-grid-state": grid_state.as_data_attr(),
            ..attributes,
            {children}
        }
    }
}

// ── Header ──────────────────────────────────────────────────────────────────

/// Container for the grid header area.
///
/// Wraps one or more [`HeaderRow`] components.
#[component]
pub fn Header(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "rowgroup",
            ..attributes,
            {children}
        }
    }
}

// ── HeaderRow ───────────────────────────────────────────────────────────────

/// A single row of column headers.
///
/// Renders with `role="row"`.
#[component]
pub fn HeaderRow(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "row",
            ..attributes,
            {children}
        }
    }
}

// ── HeaderCell ──────────────────────────────────────────────────────────────

/// A column header cell.
///
/// Displays the column header text, handles sort toggling on click,
/// and optionally renders a resize handle.
///
/// ## Data attributes
/// - `data-sort="asc|desc"` (when sorted)
/// - `data-resizable="true"` (when resizable)
/// - `data-pinned="left|right"` (when pinned)
/// - `data-col-index` — column display index
#[component]
pub fn HeaderCell(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Column display index (0-based position in the rendered grid).
    col_index: usize,
    children: Element,
) -> Element {
    let ctx: DataGridContext = use_context();

    let col = ctx.column_at(col_index);
    let sort_dir = col
        .as_ref()
        .and_then(|c| ctx.sort_direction(&c.key));
    let is_resizable = col.as_ref().is_some_and(|c| c.resizable);
    let pinned = col.as_ref().and_then(|c| c.pinned);

    let col_key = col.as_ref().map(|c| c.key.clone());
    let is_sortable = col.as_ref().is_some_and(|c| c.sortable);

    let onclick = move |_: MouseEvent| {
        if is_sortable
            && let Some(ref key) = col_key
        {
            let mut ctx: DataGridContext = consume_context();
            ctx.toggle_sort(key);
        }
    };

    let width = ctx.column_width(col_index);
    let width_style = width.map(|w| format!("width:{w}px"));
    let col_idx_str = col_index.to_string();

    rsx! {
        div {
            role: "columnheader",
            "aria-sort": match sort_dir {
                Some(crate::types::SortDirection::Ascending) => Some("ascending"),
                Some(crate::types::SortDirection::Descending) => Some("descending"),
                None => if is_sortable { Some("none") } else { None },
            },
            "data-sort": sort_dir.map(|d| d.as_data_attr()),
            "data-resizable": is_resizable.then_some("true"),
            "data-pinned": pinned.map(|p| p.as_data_attr()),
            "data-col-index": col_idx_str,
            style: width_style,
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Body ────────────────────────────────────────────────────────────────────

/// Container for the grid body (scrollable row area).
///
/// Wraps [`Row`] components. When the `virtualize` feature is enabled,
/// integrates with `use_virtual_list` for virtual scrolling.
#[component]
pub fn Body(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            role: "rowgroup",
            ..attributes,
            {children}
        }
    }
}

// ── Row ─────────────────────────────────────────────────────────────────────

/// A data row in the grid.
///
/// Self-registers on mount and deregisters on unmount for keyboard navigation.
///
/// ## Data attributes
/// - `data-selected="true"` (when selected)
/// - `data-disabled="true"` (when disabled)
/// - `data-row-index` — row display index
#[component]
pub fn Row(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Unique row identifier (must match `RowData::row_id()`).
    row_id: String,
    /// Row display index (0-based).
    row_index: usize,
    /// Whether this row is disabled.
    #[props(default)]
    disabled: bool,
    children: Element,
) -> Element {
    let mut ctx: DataGridContext = use_context();
    let rid = row_id.clone();

    // Register on mount.
    use_hook(|| {
        ctx.register_row(RowEntry {
            id: rid.clone(),
            disabled,
        });
    });
    let rid_drop = row_id.clone();
    use_drop(move || {
        let mut ctx: DataGridContext = consume_context();
        ctx.deregister_row(&rid_drop);
    });

    let is_selected = ctx.is_selected(&row_id);
    let row_idx_str = row_index.to_string();

    let rid_click = row_id.clone();
    let onclick = move |_: MouseEvent| {
        let mut ctx: DataGridContext = consume_context();
        ctx.toggle_select(&rid_click);
    };

    rsx! {
        div {
            role: "row",
            "aria-selected": if is_selected { "true" } else { "false" },
            "data-selected": is_selected.then_some("true"),
            "data-disabled": disabled.then_some("true"),
            "data-row-index": row_idx_str,
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Cell ────────────────────────────────────────────────────────────────────

/// A data cell in the grid.
///
/// Handles keyboard focus and inline editing activation.
///
/// ## Data attributes
/// - `data-focused="true"` (when focused)
/// - `data-editing="true"` (when being edited)
/// - `data-pinned="left|right"` (when column is pinned)
/// - `data-col-index` — column display index
#[component]
pub fn Cell(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Row identifier.
    row_id: String,
    /// Row display index.
    row_index: usize,
    /// Column display index.
    col_index: usize,
    /// Current cell value (for editing).
    #[props(default)]
    value: Option<String>,
    children: Element,
) -> Element {
    let ctx: DataGridContext = use_context();

    let coord = CellCoord {
        row_idx: row_index,
        col_idx: col_index,
    };
    let is_focused = ctx.focused() == Some(coord);
    let is_editing = ctx.editing_coord() == Some(coord);
    let col = ctx.column_at(col_index);
    let pinned = col.as_ref().and_then(|c| c.pinned);
    let editable = col.as_ref().and_then(|c| c.editable.clone());
    let col_key = col.as_ref().map(|c| c.key.clone()).unwrap_or_default();

    let tab_index = if is_focused { "0" } else { "-1" };
    let col_idx_str = col_index.to_string();

    let rid = row_id.clone();
    let val = value.clone().unwrap_or_default();
    let ck = col_key.clone();
    let ed = editable.clone();

    let onkeydown = move |event: KeyboardEvent| {
        let mut ctx: DataGridContext = consume_context();

        if ctx.is_editing() {
            match event.key() {
                Key::Escape => {
                    event.prevent_default();
                    ctx.cancel_edit();
                }
                Key::Enter => {
                    event.prevent_default();
                    ctx.commit_edit();
                }
                _ => {}
            }
            return;
        }

        let nav_key = match event.key() {
            Key::ArrowUp => Some(GridNavKey::Up),
            Key::ArrowDown => Some(GridNavKey::Down),
            Key::ArrowLeft => Some(GridNavKey::Left),
            Key::ArrowRight => Some(GridNavKey::Right),
            Key::Home if event.modifiers().contains(Modifiers::CONTROL) => {
                Some(GridNavKey::CtrlHome)
            }
            Key::End if event.modifiers().contains(Modifiers::CONTROL) => {
                Some(GridNavKey::CtrlEnd)
            }
            Key::Home => Some(GridNavKey::Home),
            Key::End => Some(GridNavKey::End),
            _ => None,
        };

        if let Some(key) = nav_key {
            event.prevent_default();
            let new_coord =
                navigate_grid(ctx.row_count(), ctx.col_count(), coord, key);
            ctx.set_focused(Some(new_coord));

            // Focus the target cell element via JS eval
            #[cfg(target_arch = "wasm32")]
            {
                let id = cell_element_id(new_coord.row_idx, new_coord.col_idx);
                spawn(async move {
                    let js = format!(
                        "document.getElementById('{}')?.focus()",
                        id.replace('\'', "\\'")
                    );
                    _ = document::eval(&js).await;
                });
            }
            return;
        }

        // Enter or F2 begins editing
        match event.key() {
            Key::Enter | Key::F2 => {
                if let Some(ref editor) = ed {
                    event.prevent_default();
                    ctx.begin_edit(
                        coord,
                        editor.clone(),
                        rid.clone(),
                        ck.clone(),
                        val.clone(),
                    );
                }
            }
            Key::Character(ref c) if c == " " => {
                // Space toggles row selection
                event.prevent_default();
                ctx.toggle_select(&rid);
            }
            _ => {}
        }
    };

    let onfocus = move |_: FocusEvent| {
        let mut ctx: DataGridContext = consume_context();
        ctx.set_focused(Some(coord));
    };

    rsx! {
        div {
            id: cell_element_id(row_index, col_index),
            role: "gridcell",
            tabindex: tab_index,
            "data-focused": is_focused.then_some("true"),
            "data-editing": is_editing.then_some("true"),
            "data-pinned": pinned.map(|p| p.as_data_attr()),
            "data-col-index": col_idx_str,
            onkeydown,
            onfocus,
            ..attributes,
            {children}
        }
    }
}

// ── ResizeHandle ────────────────────────────────────────────────────────────

/// Drag handle for column resizing.
///
/// Place inside a [`HeaderCell`]. Emits pointer events to track column
/// width changes. The resize is functional (inline `width` style) with
/// no visual styling.
#[component]
pub fn ResizeHandle(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Column display index to resize.
    col_index: usize,
    children: Element,
) -> Element {
    let start_x = use_signal(|| None::<i32>);
    let start_width = use_signal(|| 0u32);

    let onpointerdown = move |event: PointerEvent| {
        let ctx: DataGridContext = consume_context();
        let mut sx = start_x;
        let mut sw = start_width;
        sx.set(Some(event.client_coordinates().x as i32));
        sw.set(ctx.column_width(col_index).unwrap_or(150));
    };

    let onpointermove = move |event: PointerEvent| {
        let sx_val = (start_x)();
        if let Some(sx) = sx_val {
            let mut ctx: DataGridContext = consume_context();
            let delta = event.client_coordinates().x as i32 - sx;
            let new_width = ((start_width)() as i32 + delta).max(0) as u32;
            ctx.set_column_width(col_index, new_width);
        }
    };

    let onpointerup = move |_: PointerEvent| {
        let mut sx = start_x;
        sx.set(None);
    };

    rsx! {
        div {
            onpointerdown,
            onpointermove,
            onpointerup,
            ..attributes,
            {children}
        }
    }
}

// ── ID helpers ──────────────────────────────────────────────────────────────

/// Deterministic element ID for a grid cell.
pub(crate) fn cell_element_id(row_idx: usize, col_idx: usize) -> String {
    format!("grid-cell-{row_idx}-{col_idx}")
}
