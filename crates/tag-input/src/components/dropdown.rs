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

/// Suggestion dropdown (deprecated — dropdown/suggestion functionality removed).
///
/// Always renders nothing. Kept for API compatibility during migration.
/// Consumers should manage their own dropdown/listbox externally using
/// `on_query_change` and `on_commit` callbacks.
pub fn Dropdown<T: TagLike>(_props: DropdownProps<T>) -> Element {
    rsx! {}
}
