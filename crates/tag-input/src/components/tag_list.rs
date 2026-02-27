use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`TagList`].
#[derive(Props, Clone, PartialEq)]
pub struct TagListProps<T: TagLike + 'static> {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Semantic wrapper for tag pills.
///
/// Renders `<div role="list">` with `data-slot="tag-list"`.
pub fn TagList<T: TagLike>(props: TagListProps<T>) -> Element {
    rsx! {
        div {
            role: "list",
            "data-slot": "tag-list",
            ..props.attributes,
            {props.children}
        }
    }
}
