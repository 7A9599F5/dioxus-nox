use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`TagPopover`].
#[derive(Props, Clone, PartialEq)]
pub struct TagPopoverProps<T: TagLike + 'static> {
    pub index: usize,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Popover container for a tag pill. Conditionally renders when `popover_pill` matches the index.
///
/// Renders `<div role="dialog">` with `data-slot="tag-popover"`, `data-state="open"`.
pub fn TagPopover<T: TagLike>(props: TagPopoverProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    let is_open = *ctx.popover_pill.read() == Some(props.index);

    if !is_open {
        return rsx! {};
    }

    let popover_id = format!("dti-popover-{}", props.index);

    rsx! {
        div {
            role: "dialog",
            id: "{popover_id}",
            aria_label: "Tag actions",
            aria_modal: "false",
            "data-slot": "tag-popover",
            "data-state": "open",
            ..props.attributes,
            {props.children}
        }
    }
}
