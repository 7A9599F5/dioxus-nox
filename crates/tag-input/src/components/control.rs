use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Control`].
#[derive(Props, Clone, PartialEq)]
pub struct ControlProps<T: TagLike + 'static> {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Wrapper for the input area (pills + text input). Clicks focus the input.
///
/// Renders a `<div>` with `data-slot="control"`, `data-disabled`,
/// `data-at-limit`, and `data-focus-within` (tracks focus via `onfocusin`/`onfocusout`).
pub fn Control<T: TagLike>(props: ControlProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();
    let mut has_focus = use_signal(|| false);

    rsx! {
        div {
            "data-slot": "control",
            "data-disabled": *ctx.is_disabled.read(),
            "data-at-limit": *ctx.is_at_limit.read(),
            "data-focus-within": *has_focus.read(),
            onclick: move |_| ctx.handle_click(),
            onfocusin: move |_| has_focus.set(true),
            onfocusout: move |_| has_focus.set(false),
            ..props.attributes,
            {props.children}
        }
    }
}
