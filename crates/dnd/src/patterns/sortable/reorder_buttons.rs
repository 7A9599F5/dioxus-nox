//! Accessible reorder buttons — keyboard-friendly alternative to drag-and-drop.
//!
//! Provides move-up and move-down callbacks that fire [`ReorderEvent`]s,
//! allowing users to reorder items without pointer-based drag interactions.

use dioxus::prelude::*;

use crate::types::ReorderEvent;

/// Headless reorder buttons component.
///
/// Provides `move_up` and `move_down` elements with appropriate ARIA attributes.
/// At index 0, move-up is disabled; at the last index, move-down is disabled.
///
/// This is a **headless** component — it renders `div` wrappers with ARIA
/// attributes. Consumers style the buttons via data attributes.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_nox_dnd::ReorderButtons;
///
/// ReorderButtons {
///     index: 2,
///     total: 5,
///     on_reorder: move |evt: ReorderEvent| {
///         // Handle the reorder
///     },
/// }
/// ```
#[component]
pub fn ReorderButtons(
    /// Current index of the item in the list (0-based).
    index: usize,
    /// Total number of items in the list.
    total: usize,
    /// Handler for reorder events.
    on_reorder: EventHandler<ReorderEvent>,
    /// Optional content for the move-up button.
    #[props(default)]
    move_up_label: Option<String>,
    /// Optional content for the move-down button.
    #[props(default)]
    move_down_label: Option<String>,
) -> Element {
    let can_move_up = index > 0;
    let can_move_down = index + 1 < total;

    let up_label = move_up_label.unwrap_or_else(|| "Move item up".to_string());
    let down_label = move_down_label.unwrap_or_else(|| "Move item down".to_string());

    rsx! {
        div {
            "data-reorder-buttons": "",
            button {
                role: "button",
                aria_label: "{up_label}",
                aria_disabled: if !can_move_up { "true" },
                tabindex: "0",
                "data-reorder-up": "",
                disabled: !can_move_up,
                onclick: move |_| {
                    if can_move_up {
                        on_reorder.call(ReorderEvent {
                            from_index: index,
                            to_index: index - 1,
                            item_id: crate::types::DragId::new(""),
                            container_id: crate::types::DragId::new(""),
                        });
                    }
                },
                onkeydown: move |evt: KeyboardEvent| {
                    if can_move_up && (evt.key() == Key::Enter || evt.key() == Key::Character(" ".to_string())) {
                        evt.prevent_default();
                        on_reorder.call(ReorderEvent {
                            from_index: index,
                            to_index: index - 1,
                            item_id: crate::types::DragId::new(""),
                            container_id: crate::types::DragId::new(""),
                        });
                    }
                },
            }
            button {
                role: "button",
                aria_label: "{down_label}",
                aria_disabled: if !can_move_down { "true" },
                tabindex: "0",
                "data-reorder-down": "",
                disabled: !can_move_down,
                onclick: move |_| {
                    if can_move_down {
                        on_reorder.call(ReorderEvent {
                            from_index: index,
                            to_index: index + 1,
                            item_id: crate::types::DragId::new(""),
                            container_id: crate::types::DragId::new(""),
                        });
                    }
                },
                onkeydown: move |evt: KeyboardEvent| {
                    if can_move_down && (evt.key() == Key::Enter || evt.key() == Key::Character(" ".to_string())) {
                        evt.prevent_default();
                        on_reorder.call(ReorderEvent {
                            from_index: index,
                            to_index: index + 1,
                            item_id: crate::types::DragId::new(""),
                            container_id: crate::types::DragId::new(""),
                        });
                    }
                },
            }
        }
    }
}
