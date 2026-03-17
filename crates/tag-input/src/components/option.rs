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

/// Single suggestion item (deprecated — dropdown/suggestion functionality removed).
///
/// Still renders the item and handles mousedown selection, but highlight
/// tracking is no longer managed internally. Consumers should manage
/// their own suggestion list externally.
pub fn Option<T: TagLike>(props: OptionProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();
    let suggestion_id = ctx.suggestion_id(props.index);
    let tag_value = props.tag.id().to_string();
    let tag = props.tag.clone();

    rsx! {
        div {
            role: "option",
            id: "{suggestion_id}",
            "data-slot": "option",
            "data-value": "{tag_value}",
            onmousedown: move |evt: Event<MouseData>| {
                evt.prevent_default();
                ctx.add_tag(tag.clone());
            },
            ..props.attributes,
            {props.children}
        }
    }
}
