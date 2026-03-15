use dioxus::prelude::*;

use crate::swipe::use_swipe_gesture;
use crate::types::{SwipeConfig, SwipeHandle};

/// Root wrapper for a swipe-to-reveal item.
///
/// Wires pointer events and provides [`SwipeHandle`] via context so child
/// components ([`Content`], [`Actions`]) can read swipe state. Ships only
/// functional inline styles (`touch-action`, `position`, `overflow`).
///
/// ```rust,ignore
/// use dioxus_nox_gestures::{swipe_actions, SwipeConfig};
///
/// swipe_actions::Root {
///     config: SwipeConfig::default(),
///     on_commit: move |_| { delete_item(); },
///     swipe_actions::Content {
///         div { "Main content" }
///     }
///     swipe_actions::Actions {
///         button { "Delete" }
///     }
/// }
/// ```
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    #[props(default)] config: SwipeConfig,
    on_commit: EventHandler<()>,
    children: Element,
) -> Element {
    let handle = use_swipe_gesture(config, on_commit);
    use_context_provider(|| handle.clone());

    let phase = (handle.phase)();

    rsx! {
        div {
            "data-swipe-phase": phase.as_data_attr(),
            style: "touch-action: none; position: relative; overflow: hidden;",
            onpointerdown: handle.onpointerdown,
            onpointermove: handle.onpointermove,
            onpointerup: handle.onpointerup,
            onpointercancel: handle.onpointercancel,
            ..attributes,
            {children}
        }
    }
}

/// The main content area that translates horizontally during swipe.
///
/// Reads [`SwipeHandle`] from context (provided by [`Root`]) and applies a
/// CSS `transform: translateX(...)` based on the current offset.
#[component]
pub fn Content(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let handle: SwipeHandle = use_context();
    let offset = (handle.offset_px)();
    let phase = (handle.phase)();

    // Only apply transition when closing (spring-back animation), not while dragging.
    let transition = if phase == crate::types::SwipePhase::Closing
        || phase == crate::types::SwipePhase::Open
    {
        "transform 0.2s ease"
    } else {
        "none"
    };

    rsx! {
        div {
            "data-swipe-content": "true",
            style: "transform: translateX({offset}px); transition: {transition}; position: relative; z-index: 1;",
            ..attributes,
            {children}
        }
    }
}

/// Action buttons revealed behind the content when swiped.
///
/// Positioned absolutely to the right side of the [`Root`] container.
/// Ships only functional positioning styles.
#[component]
pub fn Actions(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-swipe-actions": "true",
            style: "position: absolute; top: 0; right: 0; bottom: 0; display: flex; align-items: stretch;",
            ..attributes,
            {children}
        }
    }
}
