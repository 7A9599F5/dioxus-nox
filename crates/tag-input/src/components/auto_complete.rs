use crate::hook::TagInputState;
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

/// Ghost text overlay for tab-to-complete. Conditionally renders when `auto_complete_text` is non-empty.
///
/// Renders `<span aria-hidden="true">` with `data-slot="autocomplete"`, `data-state="visible"`.
pub fn AutoComplete<T: TagLike>(props: AutoCompleteProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    let text = ctx.auto_complete_text.read();
    let is_visible = !text.is_empty();

    if !is_visible {
        return rsx! {};
    }

    rsx! {
        span {
            "data-slot": "autocomplete",
            "data-state": "visible",
            aria_hidden: "true",
            ..props.attributes,
            "{text}"
        }
    }
}
