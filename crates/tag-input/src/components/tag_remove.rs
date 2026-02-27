use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`TagRemove`].
#[derive(Props, Clone, PartialEq)]
pub struct TagRemoveProps<T: TagLike + 'static> {
    /// The tag this button removes. The component extracts `id()` and `name()`
    /// internally for the `onmousedown` handler and `aria-label`.
    pub tag: T,
    /// Custom content for the remove button. If not provided, defaults to
    /// `"\u{00D7}"` (multiplication sign).
    #[props(default)]
    pub children: Option<Element>,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Remove button for a tag pill.
///
/// Renders `<button>` with `aria-label="Remove {name}"` and `onmousedown`
/// (mousedown prevents input blur). Emits `data-slot="tag-remove"`, `data-disabled`.
///
/// If children are provided, they are rendered inside the button. Otherwise a
/// default `"\u{00D7}"` (multiplication sign) is used.
pub fn TagRemove<T: TagLike>(props: TagRemoveProps<T>) -> Element {
    let mut ctx = use_context::<TagInputState<T>>();
    let is_disabled = *ctx.is_disabled.read() || *ctx.is_readonly.read();
    let tag_id = props.tag.id().to_string();
    let label = format!("Remove {}", props.tag.name());

    rsx! {
        button {
            r#type: "button",
            aria_label: "{label}",
            tabindex: "-1",
            disabled: is_disabled,
            "data-slot": "tag-remove",
            "data-disabled": is_disabled,
            onmousedown: move |evt: Event<MouseData>| {
                evt.prevent_default();
                ctx.remove_tag(&tag_id);
            },
            ..props.attributes,
            if let Some(children) = props.children {
                {children}
            } else {
                {"\u{00D7}"}
            }
        }
    }
}
