use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Dropdown`].
#[derive(Props, Clone, PartialEq)]
pub struct DropdownProps<T: TagLike + 'static> {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Suggestion dropdown. Conditionally renders when open and has suggestions.
///
/// Renders `<div role="listbox">` with `aria-multiselectable` and
/// `data-slot="listbox"`, `data-state="open"`, `data-empty`.
pub fn Dropdown<T: TagLike>(props: DropdownProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    let is_open = *ctx.is_dropdown_open.read();
    let has_suggestions = !ctx.filtered_suggestions.read().is_empty();
    let is_select_mode = *ctx.select_mode.read();
    let listbox_id = ctx.listbox_id();

    if !is_open || !has_suggestions {
        return rsx! {};
    }

    rsx! {
        div {
            role: "listbox",
            id: "{listbox_id}",
            aria_label: "Suggestions",
            aria_multiselectable: if is_select_mode { "false" } else { "true" },
            "data-slot": "listbox",
            "data-state": "open",
            "data-empty": ctx.has_no_matches.read().to_string(),
            ..props.attributes,
            {props.children}
        }
    }
}
