use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`LiveRegion`].
#[derive(Props, Clone, PartialEq)]
pub struct LiveRegionProps<T: TagLike + 'static> {
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Screen reader announcement area. Visually hidden, announces status changes.
///
/// Renders `<div role="status" aria-live="polite" aria-atomic="true">`
/// with `data-slot="live-region"`.
pub fn LiveRegion<T: TagLike>(_props: LiveRegionProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    rsx! {
        div {
            role: "status",
            aria_live: "polite",
            aria_atomic: "true",
            "data-slot": "live-region",
            style: "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0",
            "{ctx.status_message}"
        }
    }
}
