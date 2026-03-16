# dioxus-nox-select — Design

## Component Parts

| Component | Element | ARIA Role | Purpose |
|-----------|---------|-----------|---------|
| `select::Root` | `div` | — | Context provider, outer container |
| `select::Trigger` | `button` | `combobox` | Opens/closes popup (select-only variant) |
| `select::Value` | `span` | — | Displays selected value or placeholder |
| `select::Input` | `input` | `combobox` | Search input (combobox variant) |
| `select::ClearButton` | `button` | — | Clears search query |
| `select::Content` | `div` | `listbox` | Popup containing options |
| `select::Item` | `div` | `option` | Selectable option |
| `select::ItemText` | `span` | — | Text content of an option |
| `select::ItemIndicator` | `span` | — | Selection checkmark (renders when selected) |
| `select::Group` | `div` | `group` | Groups related options |
| `select::Label` | `div` | — | Group heading label |
| `select::Separator` | `div` | `separator` | Visual separator |
| `select::Empty` | `div` | `status` | Shown when no items match filter |

## Variants

All variants use the same component family. The variant is determined by which children are composed:

- **Select-only**: `Root > Trigger > Value` + `Content > Item` (no Input child)
- **Combobox**: `Root > Input` + `Content > Item` (Input child present, enables search)
- **Multiselect**: Any variant with `multiple=true` on Root
- **Autocomplete**: Combobox with `autocomplete: AutoComplete::Both` (inline completion)

## Context Struct

```rust
pub struct SelectContext {
    // Value (single)
    value: Signal<String>,
    controlled_value: Option<Signal<String>>,
    // Value (multi)
    values: Signal<Vec<String>>,
    controlled_values: Option<Signal<Vec<String>>>,
    // Open state
    open: Signal<bool>,
    controlled_open: Option<Signal<bool>>,
    // Search
    search_query: Signal<String>,
    // Highlight (visual focus in listbox)
    highlighted: Signal<Option<String>>,
    // Registration
    items: Signal<Vec<ItemEntry>>,
    groups: Signal<Vec<GroupEntry>>,
    // Filtering (reactive)
    scored_items: Memo<Vec<ScoredItem>>,
    visible_values: Memo<Vec<String>>,
    // Config
    multiple: bool,
    disabled: bool,
    autocomplete: AutoComplete,
    open_on_focus: bool,
    custom_filter: Signal<Option<CustomFilter>>,
    // Callbacks
    on_value_change: Option<EventHandler<String>>,
    on_values_change: Option<EventHandler<Vec<String>>>,
    on_open_change: Option<EventHandler<bool>>,
    // Identity
    instance_id: u32,
    has_input: Signal<bool>,
}
```

## Controlled / Uncontrolled Contract

| Prop | Controlled | Uncontrolled |
|------|-----------|-------------|
| `value: Signal<String>` | Parent owns state | — |
| `default_value: String` | — | Initial value, Root owns state |
| `values: Signal<Vec<String>>` | Parent owns state | — |
| `default_values: Vec<String>` | — | Initial values, Root owns state |
| `open: Signal<bool>` | Parent owns state | — |
| `default_open: bool` | — | Initial open state |

When controlled signals are provided, reads come from them and writes go to them. When absent, internal signals are used. Callbacks (`on_value_change`, etc.) always fire regardless of mode.

## Keyboard Interaction Table

### Select-Only (on Trigger)

| Key | Closed | Open |
|-----|--------|------|
| Enter / Space | Open, highlight current | Select highlighted, close |
| ArrowDown | Open, highlight next | Highlight next |
| ArrowUp | Open, highlight prev | Highlight prev |
| Home | Open, highlight first | Highlight first |
| End | Open, highlight last | Highlight last |
| Escape | — | Close |
| Printable | Type-ahead | Type-ahead |

### Combobox (on Input)

| Key | Closed | Open |
|-----|--------|------|
| ArrowDown | Open, highlight first | Highlight next |
| ArrowUp | — | Highlight prev |
| Alt+ArrowDown | Open | — |
| Enter | — | Select highlighted |
| Escape | — | Close |
| Home / End | Cursor | Cursor |
| Printable | Filter + open | Filter |

### Multi-select modifier

- Space/Enter toggle selection (don't close)
- Highlight ≠ select (selection does NOT follow focus)

## Data Attributes

| Attribute | Components | Values |
|-----------|-----------|--------|
| `data-select-state` | Root | `"open"` / `"closed"` |
| `data-select-disabled` | Root | `"true"` (when disabled) |
| `data-state` | Trigger, Content, Item | `"open"/"closed"` or `"checked"/"unchecked"` |
| `data-highlighted` | Item | present when highlighted |
| `data-disabled` | Trigger, Item | `"true"` (when disabled) |
| `data-select-placeholder` | Value | present when showing placeholder |
| `data-select-input` | Input | present |
| `data-select-content` | Content | present |
| `data-select-item-text` | ItemText | present |
| `data-select-item-indicator` | ItemIndicator | present |
| `data-select-group` | Group | present |
| `data-select-label` | Label | present |
| `data-select-separator` | Separator | present |
| `data-select-empty` | Empty | present |
| `data-select-clear` | ClearButton | present |

## File Layout

```
crates/select/src/
├── lib.rs           # Crate docs, mod declarations, re-exports
├── types.rs         # AutoComplete, ItemEntry, GroupEntry, ScoredItem, CustomFilter
├── context.rs       # SelectContext struct, init function, methods
├── components.rs    # Root, Trigger, Value, Input, ClearButton, Content, Item,
│                    # ItemText, ItemIndicator, Group, Label, Separator, Empty
├── navigation.rs    # Pure: navigate, first, last, type_ahead
├── filter.rs        # Pure: score_items (nucleo), custom filter support
└── tests.rs         # Navigation, filter, ID, registration tests
```
