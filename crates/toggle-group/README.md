# dioxus-nox-toggle-group

Headless toggle group / segmented control with ARIA radiogroup semantics.

## Features

- Single-select (`role="radiogroup"`) and multi-select (`role="group"`) modes
- Keyboard navigation: Arrow keys, Home, End, Space/Enter
- Roving tabindex for accessible focus management
- `data-state="on|off"` and `data-disabled` for CSS targeting

## Usage

```rust,ignore
use dioxus_nox_toggle_group::*;

toggle_group::Root {
    value: "all",
    on_value_change: move |v| active.set(v),
    toggle_group::Item { value: "all", "All" }
    toggle_group::Item { value: "active", "Active" }
}
```
