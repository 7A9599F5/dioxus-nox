//! DragContextProvider component.
//!
//! This module contains the provider component that wraps application content
//! with drag-and-drop context, pointer event handlers, keyboard navigation,
//! auto-scroll, and ARIA announcements.

use dioxus::prelude::*;
use dioxus_core::Task;

#[cfg(target_arch = "wasm32")]
use super::keyboard::keyboard_focus_item;
use super::{DragContext, sorted_items_in_container};
use crate::collision::CollisionStrategy;
#[cfg(target_arch = "wasm32")]
use crate::patterns::sortable::item::NextAnimationFrame;
use crate::types::{AnnouncementEvent, DragData, DropEvent, DropLocation, Position};
use crate::utils::{extract_attribute, filter_class_style};

/// Shared keyboard-drop logic invoked by both Space and Enter handlers.
///
/// Reads the active drag's id, current keyboard container, and index, then
/// (if the drag ends successfully) dispatches a `Dropped` announcement,
/// fires `on_drop`, and focuses the dropped item.
/// Reconstruct the items list for a keyboard-drag container, with the
/// currently dragged item appended at the end. Returns `None` if no drag
/// is active.
fn items_in_keyboard_container(
    context: DragContext,
    cid: &super::DragId,
) -> Option<Vec<super::DragId>> {
    let active_id = context.active.peek().as_ref().map(|a| a.data.id.clone())?;
    let zones = context.drop_zones.peek();
    let mut items = sorted_items_in_container(&zones, cid, Some(&active_id));
    drop(zones);
    items.push(active_id);
    Some(items)
}

/// Start a keyboard drag from the currently focused `SortableItem`. Single
/// owner of keyboard activation — `SortableItem` only registers focus, this
/// is the only place that calls `start_keyboard_drag`. Returns true if a
/// drag was started so the caller can `prevent_default()` on the event.
///
/// Note: standalone `Draggable::onkeydown` also handles Space/Enter activation
/// for non-sortable draggables. Those use `ctx.start_drag` (pointer-drag
/// state), so `is_keyboard_drag()` stays false and the provider's keyboard
/// branch never fires for them — no double-handle race.
fn start_keyboard_drag_from_focus(context: DragContext) -> bool {
    let Some(focused) = context.focused_sortable() else {
        return false;
    };
    if focused.disabled || context.is_dragging() {
        return false;
    }
    let data = DragData::with_types(focused.id.clone(), focused.drag_types.clone());
    context.start_keyboard_drag(
        data,
        focused.id.clone(),
        focused.container_id.clone(),
        &focused.items,
        focused.index,
    );
    true
}

/// Shared keyboard-drop logic invoked by both Space and Enter handlers.
fn handle_keyboard_drop(context: DragContext, on_drop: EventHandler<DropEvent>) {
    let (item_id, cid, index, total) = {
        let a = context.active.peek();
        let id = a.as_ref().map(|a| a.data.id.clone());
        let c = context.keyboard_container();
        let i = *context.keyboard_index.peek();
        // Compute real total: items in container + dragged item
        let t = c
            .as_ref()
            .and_then(|cid| items_in_keyboard_container(context, cid).map(|i| i.len()));
        (id, c, i, t)
    };
    let _focus_id = item_id.clone();
    if let Some(event) = context.end_keyboard_drag() {
        if let (Some(item_id), Some(cid), Some(idx), Some(total)) = (item_id, cid, index, total) {
            context.dispatch_announcement(AnnouncementEvent::Dropped {
                item_id,
                position: idx + 1,
                total,
                container_id: cid,
            });
        }
        on_drop.call(event);

        // Focus the dropped item at its new position
        #[cfg(target_arch = "wasm32")]
        if let Some(focus_id) = _focus_id {
            keyboard_focus_item(focus_id);
        }
    }
}

/// Props for the DragContextProvider component
#[derive(Props, Clone)]
pub struct DragContextProviderProps {
    /// Children elements
    pub children: Element,

    /// Callback when an item is dropped
    #[props(default)]
    pub on_drop: EventHandler<DropEvent>,

    /// Collision detection strategy
    #[props(default)]
    pub collision_detection: CollisionStrategy,

    /// Whether items displace to create gaps (true, default) or stay in place
    /// with line indicators (false). Controls visual feedback only — collision
    /// strategy is independent.
    #[props(default = true)]
    pub gap_displacement: bool,

    /// Optional callback for structured keyboard drag announcements.
    ///
    /// When provided, called at each keyboard drag lifecycle point
    /// (grab, move, switch container, drop, cancel). If not provided,
    /// default English text is used via `AnnouncementEvent::default_text()`.
    ///
    /// Use this callback for i18n or custom announcement wording.
    #[props(default)]
    pub on_announce: EventHandler<AnnouncementEvent>,

    /// Additional HTML attributes (class, style, data-*, aria-*, etc.)
    ///
    /// Forwarded to the wrapper div. The provider uses `display: contents`
    /// and pointer/keyboard event handlers — these are preserved while
    /// consumer attributes are merged.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// Compare data-bearing fields only. `Element`, `EventHandler`, and `Attribute`
/// are diffed separately by Dioxus — returning `true` here lets the framework
/// skip re-rendering the subtree when only callbacks/children changed reference
/// identity but the configuration is the same.
impl PartialEq for DragContextProviderProps {
    fn eq(&self, other: &Self) -> bool {
        self.collision_detection == other.collision_detection
            && self.gap_displacement == other.gap_displacement
    }
}

/// Provider component for the drag-and-drop context
///
/// Wrap your application (or the part that needs drag-and-drop) with this
/// component. It provides:
/// - Global drag state management
/// - Pointer event handling for drag updates
/// - Drop event dispatching
///
/// # Example
///
/// ```ignore
/// rsx! {
///     DragContextProvider {
///         on_drop: move |event: DropEvent| {
///             // Handle the drop
///         },
///         // Your draggables and drop zones here
///     }
/// }
/// ```
#[component]
pub fn DragContextProvider(props: DragContextProviderProps) -> Element {
    // Create the context with the specified collision strategy and gap displacement mode
    // DragContext is Copy (contains only Signal handles), so no outer Signal wrapper needed
    let context =
        use_hook(|| DragContext::with_options(props.collision_detection, props.gap_displacement));

    // Provide DragContext directly via context (not wrapped in Signal)
    use_context_provider(|| context);

    // Auto-scroll effect: starts a RAF loop while dragging to scroll the
    // viewport when the pointer is near the top/bottom edge. The loop
    // continues even when the pointer is stationary (unlike pointermove).
    //
    // The task handle is stored so it can be cancelled on unmount.
    {
        let active_sig = context.active_signal();
        #[cfg(target_arch = "wasm32")]
        let scroll_vel = context.scroll_velocity_signal();
        #[cfg(target_arch = "wasm32")]
        let ctx_for_scroll = context;
        let mut raf_task: Signal<Option<Task>> = use_signal(|| None);
        use_effect(move || {
            // Cancel any previous RAF task before potentially starting a new one.
            if let Some(old) = raf_task.write().take() {
                old.cancel();
            }
            let is_active = active_sig.read().is_some();
            if is_active {
                #[cfg(target_arch = "wasm32")]
                {
                    // Spawn RAF loop for auto-scroll and viewport-driven rect refresh.
                    let task = spawn(async move {
                        loop {
                            NextAnimationFrame::new().await;
                            // Check if drag is still active (non-reactive peek)
                            if active_sig.peek().is_none() {
                                break;
                            }

                            // Keep geometry fresh while dragging even if the pointer
                            // is stationary (e.g., auto-scroll / viewport motion).
                            ctx_for_scroll.maybe_refresh_measurements();

                            let vel = *scroll_vel.peek();
                            if vel == 0.0 {
                                continue; // No scroll needed, but keep loop alive
                            }
                            if let Some(window) = web_sys::window() {
                                window.scroll_by_with_x_and_y(0.0, vel);
                            }
                        }
                    });
                    raf_task.write().replace(task);
                }
            }
        });
        use_drop(move || {
            if let Some(task) = raf_task.write().take() {
                task.cancel();
            }
        });
    }

    // Clone handler for use in closure
    let on_drop = props.on_drop;

    // Store the on_announce callback on the context so keyboard lifecycle
    // methods (start_keyboard_drag, etc.) can dispatch structured events.
    context.set_on_announce(props.on_announce);

    let is_keyboard_active = context.is_keyboard_drag();

    // Extract consumer class and style, merge with library styles
    let consumer_class = extract_attribute(&props.attributes, "class");
    let consumer_style = extract_attribute(&props.attributes, "style");
    let base_style = "display: contents;";
    let merged_style = match consumer_style {
        Some(s) if !s.is_empty() => format!("{} {}", base_style, s),
        _ => base_style.to_string(),
    };
    let merged_class = consumer_class.unwrap_or_default();
    let remaining_attrs = filter_class_style(props.attributes);

    rsx! {
        div {
            class: "{merged_class}",
            style: "{merged_style}",
            "data-keyboard-active": if is_keyboard_active { "true" },

            onpointermove: move |e| {
                context.update_drag_with_pointer(
                    Position {
                        x: e.client_coordinates().x,
                        y: e.client_coordinates().y,
                    },
                    Some(e.data().pointer_id()),
                );
            },
            onpointerup: move |e| {
                if let Some(event) = context.end_drag_with_pointer(Some(e.data().pointer_id())) {
                    on_drop.call(event);
                }
            },
            onpointercancel: move |e| {
                context.cancel_drag_with_pointer(Some(e.data().pointer_id()));
            },
            onkeydown: move |e: KeyboardEvent| {
                let key = e.key();

                // During keyboard drag: handle navigation and drop
                if context.is_keyboard_drag() {
                    match key {
                        Key::Escape => {
                            let item_id = context.active.peek().as_ref().map(|a| a.data.id.clone());
                            context.cancel_drag();
                            if let Some(item_id) = item_id {
                                context.dispatch_announcement(AnnouncementEvent::Cancelled { item_id });
                            }
                            e.prevent_default();
                        }
                        Key::ArrowDown | Key::ArrowRight => {
                            if let Some(cid) = context.keyboard_container()
                                && let Some(item_id) = context.active.peek().as_ref().map(|a| a.data.id.clone())
                                && let Some(all_items) = items_in_keyboard_container(context, &cid)
                            {
                                if let Some((pos, total)) = context.keyboard_move(1, &all_items) {
                                    // Check if the new target is a group item — enter it
                                    if let Some((inner_cid, inner_pos, inner_total)) = context.keyboard_enter_nested() {
                                        context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                            item_id, position: inner_pos, total: inner_total, container_id: inner_cid,
                                        });
                                    } else {
                                        context.dispatch_announcement(AnnouncementEvent::Moved {
                                            item_id, position: pos, total, container_id: cid,
                                        });
                                    }
                                } else {
                                    // At boundary — try exiting nested container
                                    if let Some((parent_cid, pos, total)) = context.keyboard_exit_to_parent() {
                                        context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                            item_id, position: pos, total, container_id: parent_cid,
                                        });
                                    }
                                }
                            }
                            e.prevent_default();
                        }
                        Key::ArrowUp | Key::ArrowLeft => {
                            if let Some(cid) = context.keyboard_container()
                                && let Some(item_id) = context.active.peek().as_ref().map(|a| a.data.id.clone())
                                && let Some(all_items) = items_in_keyboard_container(context, &cid)
                            {
                                if let Some((pos, total)) = context.keyboard_move(-1, &all_items) {
                                    // Check if the new target is a group item — enter it
                                    if let Some((inner_cid, inner_pos, inner_total)) = context.keyboard_enter_nested() {
                                        context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                            item_id, position: inner_pos, total: inner_total, container_id: inner_cid,
                                        });
                                    } else {
                                        context.dispatch_announcement(AnnouncementEvent::Moved {
                                            item_id, position: pos, total, container_id: cid,
                                        });
                                    }
                                } else {
                                    // At boundary — try exiting nested container
                                    if let Some((parent_cid, pos, total)) = context.keyboard_exit_to_parent() {
                                        context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                            item_id, position: pos, total, container_id: parent_cid,
                                        });
                                    }
                                }
                            }
                            e.prevent_default();
                        }
                        Key::Character(ref c) if c == " " => {
                            handle_keyboard_drop(context, on_drop);
                            e.prevent_default();
                        }
                        Key::Enter => {
                            handle_keyboard_drop(context, on_drop);
                            e.prevent_default();
                        }
                        Key::Tab => {
                            let forward = !e.modifiers().shift();
                            let item_id = context.active.peek().as_ref().map(|a| a.data.id.clone());
                            if let Some((cid, pos, total)) = context.keyboard_switch_container(forward)
                                && let Some(item_id) = item_id
                            {
                                context.dispatch_announcement(AnnouncementEvent::MovedToContainer {
                                    item_id, position: pos, total, container_id: cid,
                                });
                            }
                            e.prevent_default();
                        }
                        // M key: toggle merge target (Before/After ↔ IntoItem)
                        Key::Character(ref c) if c == "m" || c == "M" => {
                            if let Some(new_target) = context.keyboard_toggle_merge() {
                                let is_merge = matches!(new_target, DropLocation::IntoItem { .. });
                                if is_merge {
                                    let target_item = match &new_target {
                                        DropLocation::IntoItem { item_id, .. } => item_id.0.clone(),
                                        _ => unreachable!(),
                                    };
                                    context.set_announcement(format!(
                                        "Merge with {}",
                                        target_item
                                    ));
                                } else {
                                    let pos = (*context.keyboard_index.peek()).map(|i| i + 1).unwrap_or(1);
                                    context.set_announcement(format!(
                                        "Position before item, position {}",
                                        pos,
                                    ));
                                }
                            }
                            e.prevent_default();
                        }
                        _ => {}
                    }

                    // scrollIntoView for the current target after keyboard navigation
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(target) = context.current_target.peek().clone() {
                            let target_item_id = match &target {
                                DropLocation::IntoItem { item_id, .. } => Some(item_id.as_str()),
                                DropLocation::AtIndex { container_id, index } => {
                                    // Resolve item at this index for scrolling
                                    let zones = context.drop_zones.peek();
                                    let active = context.active.peek();
                                    let dragged_id = active.as_ref().map(|a| &a.data.id);
                                    let items = sorted_items_in_container(&zones, container_id, dragged_id);
                                    drop(active);
                                    drop(zones);
                                    // We can't return a reference into items, so we use the selector directly below
                                    if *index < items.len() {
                                        // Use query selector with the item's data-dnd-id
                                        let id_str = items[*index].0.clone();
                                        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                                            let selector = format!("[data-dnd-item][data-dnd-id=\"{}\"]", id_str);
                                            if let Ok(Some(el)) = document.query_selector(&selector) {
                                                let opts = web_sys::ScrollIntoViewOptions::new();
                                                opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                                                el.scroll_into_view_with_scroll_into_view_options(&opts);
                                            }
                                        }
                                    }
                                    None // Already handled scroll above
                                }
                                _ => None,
                            };
                            if let Some(id) = target_item_id
                                && let Some(document) = web_sys::window().and_then(|w| w.document())
                            {
                                let selector = format!("[data-dnd-item][data-dnd-id=\"{}\"]", id);
                                if let Ok(Some(el)) = document.query_selector(&selector) {
                                    let opts = web_sys::ScrollIntoViewOptions::new();
                                    opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                                    el.scroll_into_view_with_scroll_into_view_options(&opts);
                                }
                            }
                        }
                    }

                    return;
                }

                // Not in keyboard drag yet.
                match key {
                    // Activation: Space/Enter on a focused SortableItem starts
                    // a keyboard drag. This is the single owner of activation —
                    // SortableItem only registers focus, never starts drags.
                    Key::Character(ref c) if c == " " => {
                        if start_keyboard_drag_from_focus(context) {
                            e.prevent_default();
                        }
                    }
                    Key::Enter => {
                        if start_keyboard_drag_from_focus(context) {
                            e.prevent_default();
                        }
                    }
                    Key::Escape if context.is_dragging() => {
                        context.cancel_drag();
                        e.prevent_default();
                    }
                    _ => {}
                }
            },
            ..remaining_attrs,

            span {
                id: "{context.instructions_id().read()}",
                style: "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                "Press space bar to start a drag. While dragging, use arrow keys to move, tab to switch lists, M to toggle merge. Press space bar to drop, or press escape to cancel."
            }

            // ARIA live region for screen reader announcements.
            // Announced each time the text changes (assertive priority).
            div {
                role: "status",
                aria_live: "assertive",
                aria_atomic: "true",
                style: "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                {context.announcement_signal().read().clone()}
            }

            {props.children}
        }
    }
}
