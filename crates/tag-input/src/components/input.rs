use crate::hook::{TagInputState, extract_clipboard_text};
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Input`].
#[derive(Props, Clone, PartialEq)]
pub struct InputProps<T: TagLike + 'static> {
    #[props(default = "Type to search\u{2026}".to_string())]
    pub placeholder: String,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Text input with built-in ARIA and keyboard handling.
///
/// Renders `<input>` wiring `oninput`, `onkeydown`, `onclick`,
/// `onpaste`, and relevant ARIA attributes.
///
/// Emits `data-slot="input"`, `data-disabled`, `data-readonly`, `data-placeholder-shown`.
pub fn Input<T: TagLike>(props: InputProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();

    rsx! {
        input {
            r#type: "text",
            disabled: *ctx.is_disabled.read(),
            readonly: *ctx.is_readonly.read(),
            placeholder: "{props.placeholder}",
            value: "{ctx.search_query}",
            "data-slot": "input",
            "data-disabled": *ctx.is_disabled.read(),
            "data-readonly": *ctx.is_readonly.read(),
            "data-placeholder-shown": ctx.search_query.read().is_empty(),
            oninput: move |evt| ctx.set_query(evt.value()),
            onkeydown: move |evt| ctx.handle_input_keydown(evt),
            onclick: move |_| ctx.handle_click(),
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    ctx.handle_paste(text);
                }
            },
            ..props.attributes,
        }
    }
}
