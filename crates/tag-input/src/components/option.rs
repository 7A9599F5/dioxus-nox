use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Option`].
#[derive(Props, Clone, PartialEq)]
pub struct OptionProps<T: TagLike + 'static> {
    pub tag: T,
    pub index: usize,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

/// Single suggestion item. Handles highlight on hover and selection on mousedown.
///
/// Renders `<div role="option">` with `aria-selected` and
/// `data-slot="option"`, `data-state` ("highlighted"/"idle"), `data-value`.
pub fn Option<T: TagLike>(props: OptionProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();
    let index = props.index;
    let is_highlighted = *ctx.highlighted_index.read() == Some(index);
    let suggestion_id = ctx.suggestion_id(index);
    let tag_value = props.tag.id().to_string();
    let tag = props.tag.clone();

    rsx! {
        div {
            role: "option",
            id: "{suggestion_id}",
            aria_selected: if is_highlighted { "true" } else { "false" },
            "data-slot": "option",
            "data-state": if is_highlighted { "highlighted" } else { "idle" },
            "data-value": "{tag_value}",
            onmouseenter: move |_| { ctx.highlighted_index.set(Some(index)); },
            onmousedown: move |evt: Event<MouseData>| {
                evt.prevent_default();
                ctx.add_tag(tag.clone());
            },
            ..props.attributes,
            {props.children}
        }
    }
}
