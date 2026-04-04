//! `virtual_list::Viewport` — scrollable container with spacers.

use dioxus::prelude::*;

use super::types::VirtualListContext;

/// Scrollable container for the virtual list.
///
/// Manages scroll tracking via Dioxus-native `onscroll`, measures its own
/// height via `onmounted`, and renders top/bottom spacer divs so the
/// scrollbar reflects total content height.
///
/// ## Functional inline styles
/// - `overflow-y: auto` — required for scroll behavior (FUNCTIONAL, not VISUAL)
///
/// ## Data attributes
/// - `data-virtual-list-viewport` — presence attribute
#[component]
pub fn Viewport(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Optional CSS class on the viewport container.
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let ctx = use_context::<VirtualListContext>();
    let mut scroll_top_sig = ctx.scroll_top;
    let mut container_height_sig = ctx.container_height;
    let scroll_correction_sig = ctx.scroll_correction;

    // Track scroll position.
    let onscroll = move |event: Event<ScrollData>| {
        let pos = event.scroll_top().max(0.0) as u32;
        scroll_top_sig.set(pos);

        // Check infinite scroll from the layout snapshot (read-only).
        if let Some(ref on_end) = ctx.on_end_reached {
            let layout = ctx.layout.read();
            if layout.is_near_end(ctx.end_threshold) && layout.item_count() > 0 {
                let page_size = ctx.page_size.max(1);
                let current_page = layout.item_count() / page_size;
                let next_page = current_page + 1;
                drop(layout);
                let last = *ctx.last_page_requested.read();
                if next_page > last {
                    let mut lpr = ctx.last_page_requested;
                    lpr.set(next_page);
                    on_end.call(next_page);
                }
            }
        }
    };

    // Measure container height on mount + store ref for scroll correction.
    let mounted_ref: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    let onmounted = move |event: MountedEvent| {
        let data = event.data();
        let mut mr = mounted_ref;
        mr.set(Some(data.clone()));
        spawn(async move {
            if let Ok(rect) = data.get_client_rect().await {
                let height = rect.height() as u32;
                container_height_sig.set(height);
            }
        });
    };

    // Apply scroll correction when measurements change item positions.
    use_effect(move || {
        let correction = (scroll_correction_sig)();
        if correction == 0 {
            return;
        }
        let current = (scroll_top_sig)();
        let new_scroll = (current as i32 + correction).max(0) as u32;

        // Reset correction before applying to avoid loops.
        let mut sc = scroll_correction_sig;
        sc.set(0);
        scroll_top_sig.set(new_scroll);

        // Apply the scroll correction to the DOM.
        #[cfg(target_arch = "wasm32")]
        {
            // web_sys used here: confirmed no Dioxus 0.7 native API for
            // programmatic scrollTop assignment as of 2026-04-04.
            // Non-WASM targets: scroll correction is best-effort via signal update.
            if mounted_ref.read().is_some() {
                let script = format!(
                    "document.querySelector('[data-virtual-list-viewport]').scrollTop = {};",
                    new_scroll
                );
                document::eval(&script);
            }
        }
    });

    // Read spacer heights from the layout Memo (read-only, no write lock).
    let layout = ctx.layout.read();
    let top_spacer = layout.top_spacer_height();
    let bottom_spacer = layout.bottom_spacer_height();
    drop(layout);

    rsx! {
        div {
            "data-virtual-list-viewport": "",
            // FUNCTIONAL: overflow-y is required for scroll behavior.
            style: "overflow-y: auto;",
            class: class.unwrap_or_default(),
            onscroll: onscroll,
            onmounted: onmounted,
            ..attributes,

            // Top spacer — fills space for items above the rendered range.
            div {
                style: "height: {top_spacer}px; width: 100%;",
                "data-virtual-spacer": "top",
            }

            {children}

            // Bottom spacer — fills space for items below the rendered range.
            div {
                style: "height: {bottom_spacer}px; width: 100%;",
                "data-virtual-spacer": "bottom",
            }
        }
    }
}
