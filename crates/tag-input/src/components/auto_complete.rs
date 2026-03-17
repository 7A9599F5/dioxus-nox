use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`AutoComplete`].
#[derive(Props, Clone, PartialEq)]
pub struct AutoCompleteProps<T: TagLike + 'static> {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Ghost text overlay for tab-to-complete (deprecated — dropdown/suggestion functionality removed).
///
/// Always renders nothing. Kept for API compatibility during migration.
pub fn AutoComplete<T: TagLike>(_props: AutoCompleteProps<T>) -> Element {
    rsx! {}
}
