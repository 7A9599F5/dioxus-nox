//! `virtual_list::Item` — per-item wrapper that self-measures.

use dioxus::prelude::*;

use super::types::VirtualListContext;

/// Wrapper for each item in the virtual list.
///
/// Measures its own height on mount via `MountedData::get_client_rect()` and
/// reports it to the shared [`VirtualListContext`]. If the measured height
/// differs from the estimate and the item is above the current scroll
/// position, a scroll correction is accumulated.
///
/// ## Re-measurement
/// If item content changes dynamically, use a unique `key` on this component
/// so Dioxus unmounts/remounts it, triggering a fresh measurement.
///
/// ## Data attributes
/// - `data-virtual-list-item` — presence attribute
/// - `data-index` — zero-based item index
#[component]
pub fn Item(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Zero-based index of this item in the full list.
    index: usize,
    /// Optional CSS class on the item wrapper.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let ctx = use_context::<VirtualListContext>();
    let mut viewport_sig = ctx.viewport;
    let scroll_top_sig = ctx.scroll_top;
    let mut measure_gen_sig = ctx.measure_gen;
    let mut scroll_correction_sig = ctx.scroll_correction;

    let onmounted = move |event: MountedEvent| {
        let data = event.data();
        let idx = index;
        spawn(async move {
            let Ok(rect) = data.get_client_rect().await else {
                return;
            };
            let height = (rect.height() as u32).max(1);

            let mut vp = viewport_sig.write();
            let delta = vp.set_measured_height_with_delta(idx, height);
            if delta == 0 {
                return;
            }

            // If this item is above the current scroll position,
            // accumulate a scroll correction to keep content stable.
            let item_top = vp.offset_for_idx(idx);
            let mgen = vp.measure_gen();
            drop(vp);

            let current_scroll = *scroll_top_sig.read();
            if item_top < current_scroll {
                let current_correction = *scroll_correction_sig.read();
                scroll_correction_sig.set(current_correction + delta);
            }

            measure_gen_sig.set(mgen);
        });
    };

    let posinset = (index + 1).to_string();

    rsx! {
        div {
            role: "listitem",
            "aria-posinset": "{posinset}",
            "data-virtual-list-item": "",
            "data-index": "{index}",
            class: class.unwrap_or_default(),
            onmounted: onmounted,
            ..attributes,
            {children}
        }
    }
}
