//! `virtual_list::Root` — context provider for the compound component.

use dioxus::prelude::*;

use super::types::VirtualListContext;
use crate::VariableViewport;

/// Context provider for the virtual list compound component.
///
/// Wraps a [`super::Viewport`] and provides shared state to all child
/// components. Ships **zero visual styles** — all state is expressed through
/// `data-*` attributes.
///
/// ```text
/// virtual_list::Root {
///     item_count: 1000,
///     estimate_item_height: 48,
///     virtual_list::Viewport {
///         // render items using use_visible_range()
///     }
/// }
/// ```
///
/// ## Data attributes
/// - `data-virtual-list` — presence attribute on root
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Total number of items in the list.
    item_count: usize,
    /// Estimated height in pixels for unmeasured items.
    #[props(default = 40)]
    estimate_item_height: u32,
    /// Extra items to render beyond the visible window on each side.
    #[props(default = 5)]
    overscan: usize,
    /// Called with the next page number when scroll nears the end.
    #[props(default)]
    on_end_reached: Option<EventHandler<usize>>,
    /// How many items from the end to trigger `on_end_reached`.
    #[props(default = 5)]
    end_threshold: usize,
    /// Optional CSS class on the root element.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let mut heights = use_signal(|| {
        let mut vp = VariableViewport::new(item_count, estimate_item_height, 0);
        vp.set_overscan(overscan);
        vp
    });

    let scroll_top = use_signal(|| 0u32);
    let container_height = use_signal(|| 0u32);
    let measure_gen = use_signal(|| 0u64);

    // Memo computes layout snapshot — pure derivation, no side effects.
    // Runs once per dependency change (scroll, measurement, resize).
    // No .write(), no signal mutation — just reads signals + returns derived value.
    let layout = use_memo(move || {
        let _ = (measure_gen)(); // subscribe to measurement changes
        let st = (scroll_top)();
        let ch = (container_height)();
        heights.read().snapshot(st, ch)
    });

    let viewport_height_for_page = (container_height)();
    let page_size = if viewport_height_for_page > 0 && estimate_item_height > 0 {
        (viewport_height_for_page / estimate_item_height) as usize
    } else {
        20
    };

    let ctx = VirtualListContext {
        heights,
        layout,
        scroll_top,
        container_height,
        measure_gen,
        scroll_correction: use_signal(|| 0i32),
        item_count: use_signal(|| item_count),
        on_end_reached,
        end_threshold,
        last_page_requested: use_signal(|| 0usize),
        page_size,
    };

    use_context_provider(|| ctx);

    // Sync item_count changes into the heights signal.
    use_effect(move || {
        let count = item_count;
        heights.write().set_item_count(count);
        let mut ic = ctx.item_count;
        ic.set(count);
    });

    let set_size = item_count.to_string();

    rsx! {
        div {
            role: "list",
            "aria-setsize": "{set_size}",
            "data-virtual-list": "",
            class: class.unwrap_or_default(),
            ..attributes,
            {children}
        }
    }
}
