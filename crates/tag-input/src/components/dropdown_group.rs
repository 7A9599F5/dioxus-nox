use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`DropdownGroup`].
#[derive(Props, Clone, PartialEq)]
pub struct DropdownGroupProps<T: TagLike + 'static> {
    pub label: String,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Labelled group within the dropdown for grouped suggestions.
///
/// Renders `<div role="group">` with `aria-label` and `data-slot="dropdown-group"`, `data-group`.
pub fn DropdownGroup<T: TagLike>(props: DropdownGroupProps<T>) -> Element {
    rsx! {
        div {
            role: "group",
            aria_label: "{props.label}",
            "data-slot": "dropdown-group",
            "data-group": "{props.label}",
            ..props.attributes,
            {props.children}
        }
    }
}
