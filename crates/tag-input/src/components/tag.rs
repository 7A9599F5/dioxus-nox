use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Tag`].
#[derive(Props, Clone, PartialEq)]
pub struct TagProps<T: TagLike + 'static> {
    pub tag: T,
    pub index: usize,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

/// Individual tag pill with focus management and keyboard handling.
///
/// Renders `<div role="listitem">` with pill-mode keyboard handler.
/// Emits `data-slot="tag"`, `data-state` ("active"/"inactive"),
/// `data-locked`, `data-editing`, `data-popover-open`, `data-disabled`.
pub fn Tag<T: TagLike>(props: TagProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();
    let index = props.index;
    let is_active = *ctx.active_pill.read() == Some(index);
    let is_locked = props.tag.is_locked();
    let is_editing = *ctx.editing_pill.read() == Some(index);
    let is_popover_open = *ctx.popover_pill.read() == Some(index);
    let is_disabled = *ctx.is_disabled.read();
    let tag_name = props.tag.name().to_string();
    let pill_id = ctx.pill_id(index);

    let mut mounted_el = use_signal(|| None::<MountedEvent>);

    // Focus management: focus this pill when it becomes active
    use_effect(move || {
        if is_active && let Some(ref el) = *mounted_el.read() {
            drop(el.set_focus(true));
        }
    });

    rsx! {
        div {
            role: "listitem",
            id: "{pill_id}",
            tabindex: if is_active { "0" } else { "-1" },
            aria_label: "{tag_name}",
            "data-slot": "tag",
            "data-state": if is_active { "active" } else { "inactive" },
            "data-locked": is_locked,
            "data-editing": is_editing,
            "data-popover-open": is_popover_open,
            "data-disabled": is_disabled,
            onmounted: move |evt: MountedEvent| mounted_el.set(Some(evt)),
            onfocus: move |_| { ctx.active_pill.set(Some(index)); },
            onkeydown: move |evt| ctx.handle_pill_keydown(evt, index),
            ..props.attributes,
            {props.children}
        }
    }
}
