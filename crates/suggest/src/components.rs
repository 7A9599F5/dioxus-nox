use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::use_drop;

use crate::placement::compute_float_style;
use crate::trigger::{detect_trigger, extract_filter};
use crate::types::{TriggerConfig, TriggerContext, TriggerSelectEvent, next_instance_id};

// ── Root ──────────────────────────────────────────────────────────────────────

/// Context provider for the suggestion primitive.
///
/// Wrap your input area and suggestion list inside this component. A single
/// `Root` can handle multiple trigger characters — each fires the same
/// `on_select` handler with the active [`TriggerSelectEvent`].
///
/// ```text
/// suggest::Root {
///     triggers: vec![TriggerConfig::slash(), TriggerConfig::mention()],
///     on_select: move |evt| { /* handle selection */ },
///     suggest::Trigger { input { … } }
///     suggest::List { … }
/// }
/// ```
#[component]
pub fn Root(
    /// One entry per trigger character to recognise (order matters: first match wins).
    triggers: Vec<TriggerConfig>,
    /// Called when the user selects a suggestion item.
    on_select: EventHandler<TriggerSelectEvent>,
    children: Element,
) -> Element {
    let active_char: Signal<Option<char>> = use_signal(|| None);
    let filter: Signal<String> = use_signal(String::new);
    let trigger_offset: Signal<usize> = use_signal(|| 0);
    let highlighted_index: Signal<Option<usize>> = use_signal(|| None);
    let items: Signal<Vec<String>> = use_signal(Vec::new);
    let on_select_sig: Signal<Option<EventHandler<TriggerSelectEvent>>> =
        use_signal(|| Some(on_select));
    let trigger_configs: Signal<Vec<TriggerConfig>> = use_signal(|| triggers.clone());
    let trigger_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    // Keep on_select and trigger_configs in sync with props across re-renders.
    let mut on_select_sig_mut = on_select_sig;
    on_select_sig_mut.set(Some(on_select));
    let mut trigger_configs_mut = trigger_configs;
    trigger_configs_mut.set(triggers);

    let instance_id = use_hook(next_instance_id);

    let ctx = TriggerContext {
        active_char,
        filter,
        trigger_offset,
        highlighted_index,
        items,
        on_select: on_select_sig,
        trigger_configs,
        trigger_element,
        instance_id,
    };

    use_context_provider(|| ctx);

    rsx! { {children} }
}

// ── Trigger ───────────────────────────────────────────────────────────────────

/// Wraps the consumer's `<input>` or `<textarea>` to detect trigger characters.
///
/// Captures `oninput` (trigger detection) and `onkeydown` (Arrow/Enter/Escape
/// navigation) from the focusable child element via event bubbling.
///
/// ```text
/// suggest::Trigger {
///     textarea { … }
/// }
/// ```
///
/// ## Data attributes
/// - `data-slot="trigger-input"` — always present
/// - `data-trigger="/"` — set to the active trigger char when a trigger is open
///
/// ## Cursor position (wasm32 only)
///
/// Trigger detection reads `document.activeElement.selectionStart` via
/// `document::eval`. On non-wasm targets the trigger stays inactive (v0.1).
///
/// <!-- web_sys used here: confirmed no Dioxus 0.7 native API for selectionStart as of 2026-02-26.
///      Source: Dioxus 0.7 docs + dioxus-primitives source search.
///      Non-WASM targets: trigger detection is a no-op, stays inactive. -->
#[component]
pub fn Trigger(
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
    /// External `(text, cursor_utf16)` signal for non-textarea inputs (e.g. contenteditable).
    /// When `Some`, skips the `oninput` path and runs trigger detection reactively.
    external_input: Option<ReadSignal<(String, usize)>>,
) -> Element {
    let ctx = use_context::<TriggerContext>();

    // Reactive trigger detection from external signal (contenteditable / InlineEditor).
    // Hook is always called unconditionally; Option guard is inside.
    use_effect(move || {
        let Some(ext) = external_input else { return };
        let (text, cursor_utf16) = (ext)();
        let configs = ctx.trigger_configs.read().clone();
        let mut ctx2 = ctx;
        let mut found = false;
        for config in &configs {
            if let Some(offset) =
                detect_trigger(&text, cursor_utf16, config.char, config.line_start_only)
                && let Some(filter) = extract_filter(
                    &text,
                    cursor_utf16,
                    config.char,
                    config.line_start_only,
                    config.allow_spaces,
                    config.max_filter_len,
                )
            {
                ctx2.active_char.set(Some(config.char));
                ctx2.trigger_offset.set(offset);
                ctx2.filter.set(filter);
                ctx2.highlighted_index.set(None);
                found = true;
                break;
            }
        }
        if !found && ctx2.active_char.read().is_some() {
            ctx2.close();
        }
    });

    // Precompute data-trigger attr: Option<String> — absent when no trigger active.
    let trigger_char_attr: Option<String> =
        (*ctx.active_char.read()).map(|c| c.to_string());

    rsx! {
        div {
            "data-slot": "trigger-input",
            "data-trigger": trigger_char_attr,

            // onmounted / event handlers must come before ..attributes spread.
            onmounted: move |evt: MountedEvent| {
                let data = evt.data();
                let mut tel = ctx.trigger_element;
                tel.set(Some(data.clone()));
            },

            // ── oninput: detect trigger chars on every keystroke ──────────
            oninput: move |evt: FormEvent| {
                // Skip when external_input is active — effect handles detection reactively.
                if external_input.is_some() {
                    return;
                }
                let text = evt.value();
                let configs = ctx.trigger_configs.read().clone();
                let mut ctx_inner = ctx;

                // Cursor position requires async JS eval.
                // Spawn so we don't block the synchronous event handler.
                spawn(async move {
                    // web_sys used here: confirmed no Dioxus 0.7 native API for
                    // selectionStart as of 2026-02-26. Source: Dioxus docs search.
                    // Non-WASM targets: trigger detection stays inactive (v0.1).
                    #[cfg(target_arch = "wasm32")]
                    let cursor_utf16 = {
                        let mut ev = document::eval(
                            "dioxus.send(document.activeElement?.selectionStart ?? 0);",
                        );
                        ev.recv::<u64>().await.unwrap_or(0) as usize
                    };

                    #[cfg(not(target_arch = "wasm32"))]
                    let cursor_utf16: usize = 0;

                    let mut found = false;
                    for config in &configs {
                        if let Some(offset) = detect_trigger(
                            &text,
                            cursor_utf16,
                            config.char,
                            config.line_start_only,
                        ) && let Some(filter) = extract_filter(
                            &text,
                            cursor_utf16,
                            config.char,
                            config.line_start_only,
                            config.allow_spaces,
                            config.max_filter_len,
                        ) {
                            ctx_inner.active_char.set(Some(config.char));
                            ctx_inner.trigger_offset.set(offset);
                            ctx_inner.filter.set(filter);
                            ctx_inner.highlighted_index.set(None);
                            found = true;
                            break;
                        }
                    }
                    if !found && ctx_inner.active_char.read().is_some() {
                        ctx_inner.close();
                    }
                });
            },

            // ── onkeydown: navigation and dismiss ────────────────────────
            // CRITICAL: prevent_default() MUST be called synchronously here.
            onkeydown: move |e: KeyboardEvent| {
                if ctx.active_char.read().is_none() {
                    return;
                }
                match e.key().to_string().as_str() {
                    "ArrowDown" => {
                        e.prevent_default();
                        ctx.select_next();
                    }
                    "ArrowUp" => {
                        e.prevent_default();
                        ctx.select_prev();
                    }
                    "Enter" => {
                        if ctx.highlighted_index.read().is_some() {
                            e.prevent_default();
                            ctx.confirm_selection();
                        }
                    }
                    "Escape" | "Tab" => {
                        ctx.close();
                    }
                    _ => {}
                }
            },

            ..attributes,
            {children}
        }
    }
}

// ── List ──────────────────────────────────────────────────────────────────────

/// Floating suggestion list.
///
/// Renders as `data-state="open"` when a trigger is active, `"closed"` otherwise.
/// Positions itself below the [`Trigger`] wrapper via `position:fixed` computed
/// from the trigger element's `get_client_rect()`.
///
/// ```text
/// suggest::List {
///     suggest::Item { value: "heading1", "Heading 1" }
///     suggest::Item { value: "heading2", "Heading 2" }
/// }
/// ```
///
/// ## Data attributes
/// - `data-slot="trigger-list"` — always present
/// - `data-state="open"` / `"closed"` — visibility state
/// - `data-trigger="/"` — active trigger char when open
#[component]
pub fn List(
    /// Gap between the trigger element's bottom edge and the list top. Default: `4.0`.
    #[props(default = 4.0)]
    side_offset: f64,
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx = use_context::<TriggerContext>();
    let float_style: Signal<String> = use_signal(String::new);

    // Recompute position whenever the trigger opens.
    use_effect(move || {
        // Subscribe BEFORE early-return guard (Dioxus 0.7 gotcha).
        let is_open = ctx.active_char.read().is_some();
        if !is_open {
            return;
        }
        let el = ctx.trigger_element.read().clone();
        let mut fs = float_style;
        let offset = side_offset;
        spawn(async move {
            let Some(data) = el else { return };
            let Ok(rect) = data.get_client_rect().await else { return };
            fs.set(compute_float_style(
                rect.min_x(),
                rect.max_y(),
                rect.size.width,
                offset,
                0.0,
            ));
        });
    });

    let is_open = ctx.active_char.read().is_some();
    let trigger_char_attr: Option<String> = (*ctx.active_char.read()).map(|c| c.to_string());
    let style_val = {
        let s = float_style.read().clone();
        if s.is_empty() { None } else { Some(s) }
    };

    rsx! {
        div {
            "data-slot": "trigger-list",
            "data-state": if is_open { "open" } else { "closed" },
            "data-trigger": trigger_char_attr,
            style: style_val,
            ..attributes,
            {children}
        }
    }
}

// ── Item ──────────────────────────────────────────────────────────────────────

/// Selectable suggestion item.
///
/// Self-registers with the surrounding [`Root`] on mount and unregisters on
/// drop. Keyboard navigation (`ArrowDown` / `ArrowUp` in the `Trigger` input)
/// highlights items in mount order.
///
/// Selecting an item calls the `Root`'s `on_select` handler and closes the list.
///
/// ## Data attributes
/// - `data-highlighted="true"` — present when this item is keyboard-highlighted
#[component]
pub fn Item(
    /// Identifies this item in [`TriggerSelectEvent::value`].
    value: String,
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx = use_context::<TriggerContext>();

    // Clone value for each closure before any move captures it.
    let reg_value = value.clone();
    let drop_value = value.clone();
    let click_value = value.clone();

    // Self-register on mount.
    use_hook(move || {
        ctx.register_item(reg_value);
    });

    // Unregister on drop.
    use_drop(move || {
        ctx.unregister_item(&drop_value);
    });

    // Derive highlighted state: compare value at highlighted_index against ours.
    let is_highlighted = use_memo(move || {
        let hi = *ctx.highlighted_index.read();
        let items = ctx.items.read();
        hi.and_then(|idx| items.get(idx)).is_some_and(|v| v == &value)
    });

    rsx! {
        div {
            "data-highlighted": if *is_highlighted.read() { "true" } else { "" },
            role: "option",
            onclick: move |_| {
                let ac = *ctx.active_char.read();
                let filter = ctx.filter.read().clone();
                let offset = *ctx.trigger_offset.read();
                if let Some(trigger_char) = ac {
                    let event = TriggerSelectEvent {
                        trigger_char,
                        value: click_value.clone(),
                        filter,
                        trigger_offset: offset,
                    };
                    if let Some(ref h) = *ctx.on_select.read() {
                        h.call(event);
                    }
                    ctx.close();
                }
            },
            ..attributes,
            {children}
        }
    }
}

// ── Group ─────────────────────────────────────────────────────────────────────

/// Labeled section inside a [`List`].
///
/// Applies `role="group"` and `aria-label` for accessibility.
#[component]
pub fn Group(
    /// Accessible label for this group (rendered as visible text and `aria-label`).
    label: String,
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let label_text = label.clone();
    rsx! {
        div {
            role: "group",
            aria_label: label,
            "data-suggest-group": "",
            ..attributes,
            div {
                "data-suggest-group-label": "",
                {label_text}
            }
            {children}
        }
    }
}

// ── Empty ─────────────────────────────────────────────────────────────────────

/// Slot shown when the item list is empty.
///
/// The consumer is responsible for conditionally rendering this based on
/// whether their filtered item list is empty.
#[component]
pub fn Empty(
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-suggest-empty": "",
            ..attributes,
            {children}
        }
    }
}
