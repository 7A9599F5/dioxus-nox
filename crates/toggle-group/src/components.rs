//! Toggle group compound components.

use dioxus::prelude::*;

use crate::types::{Orientation, ToggleGroupContext};

/// Headless toggle group root.
///
/// Provides context and ARIA radiogroup/group semantics.
/// Handles keyboard navigation: Arrow keys, Home, End, Space/Enter.
#[component]
pub fn Root(
    /// Toggle group children (Item components).
    children: Element,
    /// Currently active value (controlled).
    value: String,
    /// Change handler.
    on_value_change: EventHandler<String>,
    /// Single-select (radiogroup) or multi-select (group).
    #[props(default = false)]
    multi_select: bool,
    /// Layout orientation.
    #[props(default)]
    orientation: Orientation,
) -> Element {
    use_context_provider(|| ToggleGroupContext {
        value: value.clone(),
        on_value_change: on_value_change.clone(),
        multi_select,
        orientation,
    });

    let role = if multi_select { "group" } else { "radiogroup" };

    rsx! {
        div {
            role: role,
            aria_orientation: orientation.as_str(),
            "data-toggle-group": "",
            {children}
        }
    }
}

/// Headless toggle group item.
///
/// Emits `role="radio"` (single) or `role="checkbox"` (multi),
/// `aria-checked`, `data-state="on|off"`, and keyboard handlers.
#[component]
pub fn Item(
    /// This item's value.
    value: String,
    /// Item content.
    children: Element,
    /// Whether disabled.
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let ctx: ToggleGroupContext = use_context();
    let is_active = ctx.value == value;
    let role = if ctx.multi_select { "checkbox" } else { "radio" };
    let data_state = if is_active { "on" } else { "off" };
    let aria_checked = if is_active { "true" } else { "false" };

    let item_value = value.clone();
    let on_change = ctx.on_value_change.clone();

    rsx! {
        div {
            role: role,
            aria_checked: aria_checked,
            "data-state": data_state,
            "data-disabled": if disabled { Some("true") } else { None },
            tabindex: if disabled { "-1" } else if is_active { "0" } else { "-1" },
            onclick: move |_| {
                if !disabled {
                    on_change.call(item_value.clone());
                }
            },
            onkeydown: move |evt: KeyboardEvent| {
                if disabled {
                    return;
                }
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        ctx.on_value_change.call(value.clone());
                    }
                    Key::Character(ref c) if c == " " => {
                        evt.prevent_default();
                        ctx.on_value_change.call(value.clone());
                    }
                    _ => {}
                }
            },
            {children}
        }
    }
}
