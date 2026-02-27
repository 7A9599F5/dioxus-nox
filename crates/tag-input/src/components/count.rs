use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`Count`].
#[derive(Props, Clone, PartialEq)]
pub struct CountProps<T: TagLike + 'static> {
    #[props(default)]
    pub children: Element,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Overflow indicator. Renders "+N more" when `overflow_count > 0`.
///
/// Renders `<span>` with `data-slot="count"`. Hidden when count is zero.
pub fn Count<T: TagLike>(props: CountProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    let count = *ctx.overflow_count.read();

    if count == 0 {
        return rsx! {};
    }

    rsx! {
        span {
            "data-slot": "count",
            ..props.attributes,
            "+{count} more"
        }
    }
}
