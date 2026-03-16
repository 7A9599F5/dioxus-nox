use dioxus::prelude::*;

use crate::types::{ActivationMode, Orientation, TabEntry, TabsContext};

// ── Root ─────────────────────────────────────────────────────────────────────

/// Context provider for the tabs compound component.
///
/// Wraps a [`List`] and one or more [`Content`] panels. Ships **zero visual
/// styles** — all state is expressed through `data-*` attributes.
///
/// ```text
/// tabs::Root {
///     default_value: "tab1",
///     tabs::List {
///         tabs::Trigger { value: "tab1", "First" }
///         tabs::Trigger { value: "tab2", "Second" }
///     }
///     tabs::Content { value: "tab1", p { "Panel one" } }
///     tabs::Content { value: "tab2", p { "Panel two" } }
/// }
/// ```
///
/// ## Data attributes
/// - `data-tabs-orientation="horizontal|vertical"`
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial active tab (uncontrolled mode).
    #[props(default)]
    default_value: Option<String>,
    /// Controlled active tab value. When provided, the component is controlled.
    #[props(default)]
    value: Option<Signal<String>>,
    /// Fires when the active tab changes.
    #[props(default)]
    on_value_change: Option<EventHandler<String>>,
    /// Layout direction. Determines arrow-key navigation axis and `aria-orientation`.
    #[props(default)]
    orientation: Orientation,
    /// Whether tabs activate on focus (Automatic) or require explicit
    /// Space/Enter (Manual).
    #[props(default)]
    activation_mode: ActivationMode,
    children: Element,
) -> Element {
    let initial = default_value.unwrap_or_default();
    let internal_value = use_signal(|| initial);

    let ctx = TabsContext {
        value: internal_value,
        controlled: value,
        on_value_change,
        orientation,
        activation_mode,
        tabs: use_signal(Vec::new),
    };

    use_context_provider(|| ctx);

    let orient = orientation.as_data_attr();

    rsx! {
        div {
            "data-tabs-orientation": orient,
            ..attributes,
            {children}
        }
    }
}

// ── List ─────────────────────────────────────────────────────────────────────

/// Container for tab triggers.
///
/// Renders a `div` with `role="tablist"` and handles keyboard navigation
/// per the WAI-ARIA Tabs pattern.
///
/// ## Keyboard interaction
/// | Key | Horizontal | Vertical |
/// |-----|-----------|----------|
/// | Arrow Right / Arrow Down | Next tab | Next tab |
/// | Arrow Left / Arrow Up | Previous tab | Previous tab |
/// | Home | First tab | First tab |
/// | End | Last tab | Last tab |
///
/// ## Data attributes
/// - `data-tabs-orientation="horizontal|vertical"`
#[component]
pub fn List(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Optional visible label. Sets `aria-label` on the tablist.
    #[props(default)]
    aria_label: Option<String>,
    children: Element,
) -> Element {
    let ctx: TabsContext = use_context();
    let orient = ctx.orientation.as_data_attr();
    let aria_orient = ctx.orientation.as_aria_attr();

    let onkeydown = move |event: KeyboardEvent| {
        let mut ctx: TabsContext = consume_context();
        let active = ctx.active_value();

        let target_value = match event.key() {
            // Horizontal: Left/Right, Vertical: Up/Down
            Key::ArrowRight if ctx.orientation == Orientation::Horizontal => ctx.next(&active),
            Key::ArrowDown if ctx.orientation == Orientation::Vertical => ctx.next(&active),
            Key::ArrowLeft if ctx.orientation == Orientation::Horizontal => ctx.prev(&active),
            Key::ArrowUp if ctx.orientation == Orientation::Vertical => ctx.prev(&active),
            Key::Home => ctx.first(),
            Key::End => ctx.last(),
            Key::Enter => {
                if ctx.activation_mode == ActivationMode::Manual {
                    ctx.activate(&active);
                }
                return;
            }
            Key::Character(ref c) if c == " " => {
                if ctx.activation_mode == ActivationMode::Manual {
                    ctx.activate(&active);
                }
                return;
            }
            _ => return,
        };

        event.prevent_default();

        if let Some(ref val) = target_value
            && ctx.activation_mode == ActivationMode::Automatic
        {
            ctx.activate(val);
        }
        // Focus the target trigger element via JS eval
        #[cfg(target_arch = "wasm32")]
        if let Some(ref val) = target_value {
            let id = trigger_element_id(val);
            spawn(async move {
                let js = format!(
                    "document.getElementById('{}')?.focus()",
                    id.replace('\'', "\\'")
                );
                _ = document::eval(&js).await;
            });
        }
    };

    rsx! {
        div {
            role: "tablist",
            "aria-orientation": aria_orient,
            "data-tabs-orientation": orient,
            "aria-label": aria_label,
            onkeydown,
            ..attributes,
            {children}
        }
    }
}

// ── Trigger ──────────────────────────────────────────────────────────────────

/// A single tab trigger (button).
///
/// Must be a descendant of [`List`]. Renders a `<button>` with `role="tab"`.
///
/// ## Data attributes
/// - `data-state="active|inactive"`
/// - `data-disabled="true"` (when disabled)
#[component]
pub fn Trigger(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Unique value identifying this tab. Must match a [`Content`]'s `value`.
    value: String,
    /// Prevent activation and skip in keyboard navigation.
    #[props(default)]
    disabled: bool,
    children: Element,
) -> Element {
    let mut ctx: TabsContext = use_context();
    let val = value.clone();

    // Register on mount, deregister on unmount.
    use_hook(|| {
        ctx.register(TabEntry {
            value: val.clone(),
            disabled,
        });
    });
    let val_drop = value.clone();
    use_drop(move || {
        let mut ctx: TabsContext = consume_context();
        ctx.deregister(&val_drop);
    });

    let is_active = ctx.is_active(&value);
    let state = if is_active { "active" } else { "inactive" };
    let tab_index = if is_active { "0" } else { "-1" };
    let panel_id = panel_element_id(&value);
    let trigger_id = trigger_element_id(&value);

    let val_click = value.clone();
    let onclick = move |_: MouseEvent| {
        if !disabled {
            let mut ctx: TabsContext = consume_context();
            ctx.activate(&val_click);
        }
    };

    let val_focus = value.clone();
    let onfocus = move |_: FocusEvent| {
        if !disabled {
            let mut ctx: TabsContext = consume_context();
            if ctx.activation_mode == ActivationMode::Automatic {
                ctx.activate(&val_focus);
            }
        }
    };

    rsx! {
        button {
            id: trigger_id,
            role: "tab",
            "type": "button",
            "aria-selected": if is_active { "true" } else { "false" },
            "aria-controls": panel_id,
            tabindex: tab_index,
            "data-state": state,
            "data-disabled": disabled.then_some("true"),
            disabled: disabled,
            onclick,
            onfocus,
            ..attributes,
            {children}
        }
    }
}

// ── Content ──────────────────────────────────────────────────────────────────

/// Tab panel content associated with a [`Trigger`].
///
/// Only renders children when the matching trigger is active.
///
/// ## Data attributes
/// - `data-state="active|inactive"`
#[component]
pub fn Content(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Must match a [`Trigger`]'s `value`.
    value: String,
    children: Element,
) -> Element {
    let ctx: TabsContext = use_context();
    let is_active = ctx.is_active(&value);
    let state = if is_active { "active" } else { "inactive" };
    let trigger_id = trigger_element_id(&value);
    let panel_id = panel_element_id(&value);

    if !is_active {
        return rsx! {};
    }

    rsx! {
        div {
            id: panel_id,
            role: "tabpanel",
            "aria-labelledby": trigger_id,
            tabindex: "0",
            "data-state": state,
            ..attributes,
            {children}
        }
    }
}

// ── ID helpers ───────────────────────────────────────────────────────────────

/// Deterministic element ID for a trigger button.
pub(crate) fn trigger_element_id(value: &str) -> String {
    format!("{value}-tab")
}

/// Deterministic element ID for a content panel.
pub(crate) fn panel_element_id(value: &str) -> String {
    format!("{value}-panel")
}
