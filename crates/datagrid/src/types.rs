use std::rc::Rc;

use dioxus::prelude::*;

/// Trait for row data types used in the data grid.
///
/// Consumers implement this for their domain types. The grid uses `row_id()`
/// for selection tracking, keyboard focus, and virtual scroll keying.
///
/// ```rust,ignore
/// struct Workout { id: String, name: String, reps: u32 }
///
/// impl RowData for Workout {
///     fn row_id(&self) -> &str { &self.id }
/// }
/// ```
pub trait RowData: Clone + PartialEq + 'static {
    /// Unique identifier for this row.
    fn row_id(&self) -> &str;
}

/// Newtype wrapper for cell render functions.
///
/// Wraps `Rc<dyn Fn() -> Element>` with `PartialEq` returning `false`
/// (function pointers cannot be compared).
#[derive(Clone)]
pub struct RenderCell(pub Rc<dyn Fn() -> Element>);

impl RenderCell {
    pub fn new(f: impl Fn() -> Element + 'static) -> Self {
        Self(Rc::new(f))
    }
}

impl PartialEq for RenderCell {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for RenderCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RenderCell(..)")
    }
}

/// Cell editor variant for inline editing.
#[derive(Clone, Debug, PartialEq)]
pub enum CellEditor {
    /// Plain text input.
    Text,
    /// Numeric input.
    Number,
    /// Dropdown selection from fixed options.
    Select(Vec<String>),
    /// Custom editor render function.
    Custom(RenderCell),
}

/// Column pinning side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PinSide {
    Left,
    Right,
}

impl PinSide {
    /// Value for `data-pinned` attribute.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// Sort direction for a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// Value for `data-sort` attribute.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }

    /// Toggle between ascending and descending.
    pub fn toggle(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// Row selection mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Selection {
    /// No row selection.
    #[default]
    None,
    /// Single row selection.
    Single,
    /// Multi-row selection (Ctrl+click, Shift+click).
    Multi,
}

/// Coordinate of a cell in the grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub row_idx: usize,
    pub col_idx: usize,
}

/// Active sort state for a single column.
#[derive(Clone, Debug, PartialEq)]
pub struct SortState {
    /// Column key being sorted.
    pub column: String,
    /// Current sort direction.
    pub direction: SortDirection,
}

/// Event emitted when a cell edit is committed.
#[derive(Clone, Debug, PartialEq)]
pub struct CellEditEvent {
    /// Row identifier.
    pub row_id: String,
    /// Column key.
    pub column: String,
    /// Value before editing.
    pub old_value: String,
    /// New value after editing.
    pub new_value: String,
}

/// Column definition for the data grid.
///
/// Use the builder methods to configure column behavior:
/// ```rust,ignore
/// ColumnDef::new("name")
///     .header("Exercise Name")
///     .sortable(true)
///     .resizable(true)
///     .width(200)
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnDef {
    /// Unique column identifier.
    pub key: String,
    /// Display header text.
    pub header_text: String,
    /// Whether this column can be sorted.
    pub sortable: bool,
    /// Whether this column can be resized.
    pub resizable: bool,
    /// Inline editor type (None = read-only).
    pub editable: Option<CellEditor>,
    /// Pin column to left or right edge.
    pub pinned: Option<PinSide>,
    /// Initial width in pixels.
    pub width: Option<u32>,
    /// Minimum width in pixels during resize.
    pub min_width: Option<u32>,
    /// Maximum width in pixels during resize.
    pub max_width: Option<u32>,
}

impl ColumnDef {
    /// Create a new column definition with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        let header_text = key.clone();
        Self {
            key,
            header_text,
            sortable: false,
            resizable: false,
            editable: None,
            pinned: None,
            width: None,
            min_width: None,
            max_width: None,
        }
    }

    /// Set the display header text.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header_text = header.into();
        self
    }

    /// Enable or disable sorting.
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Enable or disable resizing.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set the cell editor type.
    pub fn editable(mut self, editor: CellEditor) -> Self {
        self.editable = Some(editor);
        self
    }

    /// Pin this column to a side.
    pub fn pinned(mut self, side: PinSide) -> Self {
        self.pinned = Some(side);
        self
    }

    /// Set the initial width in pixels.
    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the minimum width during resize.
    pub fn min_width(mut self, min: u32) -> Self {
        self.min_width = Some(min);
        self
    }

    /// Set the maximum width during resize.
    pub fn max_width(mut self, max: u32) -> Self {
        self.max_width = Some(max);
        self
    }
}

/// Registration entry for a row (used for keyboard navigation and selection).
#[derive(Clone, Debug, PartialEq)]
pub struct RowEntry {
    /// Row identifier (from `RowData::row_id()`).
    pub id: String,
    /// Whether the row is disabled.
    pub disabled: bool,
}

/// Grid state for `data-grid-state` attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GridState {
    #[default]
    Idle,
    Editing,
    Selecting,
}

impl GridState {
    /// Value for `data-grid-state` attribute.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Editing => "editing",
            Self::Selecting => "selecting",
        }
    }
}
