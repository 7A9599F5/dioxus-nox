use dioxus::prelude::*;

use crate::context::{SelectContext, init_select_context};
use crate::types::*;

// ── ItemContext (inner) ─────────────────────────────────────────────────────

/// Inner context provided by [`Item`] to child components like [`ItemIndicator`].
#[derive(Clone)]
pub(crate) struct ItemContext {
    pub value: String,
}

// ── GroupContext (inner) ─────────────────────────────────────────────────────

/// Inner context provided by [`Group`] so child items can associate with a group.
#[derive(Clone)]
pub(crate) struct GroupContext {
    pub id: String,
}

// ── Root ────────────────────────────────────────────────────────────────────

/// Context provider for the select compound component.
///
/// Wraps a [`Trigger`] (or [`Input`]) and a [`Content`] popup. Ships **zero
/// visual styles** — all state is expressed through `data-*` attributes.
///
/// ## Variants
///
/// - **Select-only**: Compose `Trigger` + `Value` inside Root (no `Input`).
/// - **Combobox**: Compose `Input` inside Root (enables search/filter).
/// - **Multiselect**: Set `multiple: true`.
///
/// ## Data attributes
///
/// - `data-select-state="open|closed"`
/// - `data-select-disabled="true"` (when disabled)
#[component]
pub fn Root(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Initial value (uncontrolled, single-select).
    #[props(default)]
    default_value: Option<String>,
    /// Controlled single-select value.
    #[props(default)]
    value: Option<Signal<String>>,
    /// Fires when single-select value changes.
    #[props(default)]
    on_value_change: Option<EventHandler<String>>,
    /// Initial values (uncontrolled, multi-select).
    #[props(default)]
    default_values: Option<Vec<String>>,
    /// Controlled multi-select values.
    #[props(default)]
    values: Option<Signal<Vec<String>>>,
    /// Fires when multi-select values change.
    #[props(default)]
    on_values_change: Option<EventHandler<Vec<String>>>,
    /// Enable multi-select mode.
    #[props(default)]
    multiple: bool,
    /// Disable the entire select.
    #[props(default)]
    disabled: bool,
    /// Whether popup starts open.
    #[props(default)]
    default_open: bool,
    /// Controlled open state.
    #[props(default)]
    open: Option<Signal<bool>>,
    /// Fires when open state changes.
    #[props(default)]
    on_open_change: Option<EventHandler<bool>>,
    /// Autocomplete mode (only relevant when `Input` child is present).
    #[props(default)]
    autocomplete: AutoComplete,
    /// Auto-open dropdown when input receives focus (combobox variant).
    #[props(default = true)]
    open_on_focus: bool,
    /// Custom filter function. Overrides built-in nucleo fuzzy matching.
    #[props(default)]
    filter: Option<CustomFilter>,
    children: Element,
) -> Element {
    let ctx = init_select_context(
        default_value,
        value,
        on_value_change,
        default_values,
        values,
        on_values_change,
        multiple,
        disabled,
        default_open,
        open,
        on_open_change,
        autocomplete,
        open_on_focus,
        filter,
    );

    let state = if ctx.is_open() { "open" } else { "closed" };

    rsx! {
        div {
            "data-select-state": state,
            "data-select-disabled": disabled.then_some("true"),
            ..attributes,
            {children}
        }
    }
}

// ── Trigger ─────────────────────────────────────────────────────────────────

/// The button that opens/closes the select popup.
///
/// For the **select-only** variant (no `Input` child). Renders a `<button>`
/// with `role="combobox"` and full keyboard handling per the WAI-ARIA
/// select-only combobox pattern.
///
/// ## Data attributes
///
/// - `data-state="open|closed"`
/// - `data-disabled="true"` (when disabled)
#[component]
pub fn Trigger(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Disable the trigger.
    #[props(default)]
    disabled: bool,
    children: Element,
) -> Element {
    let ctx: SelectContext = use_context();

    let is_open = ctx.is_open();
    let state = if is_open { "open" } else { "closed" };
    let trigger_id = ctx.trigger_id();
    let listbox_id = ctx.listbox_id();
    let active_desc = ctx.active_descendant();
    let is_disabled = disabled || ctx.disabled;

    let onkeydown = move |event: KeyboardEvent| {
        if is_disabled {
            return;
        }
        let mut ctx: SelectContext = consume_context();
        let was_open = ctx.is_open();

        match event.key() {
            Key::Enter => {
                event.prevent_default();
                if was_open {
                    ctx.confirm_highlighted();
                } else {
                    ctx.set_open(true);
                    let current = ctx.current_value();
                    if !current.is_empty() {
                        ctx.highlighted.set(Some(current));
                    } else {
                        ctx.highlight_first();
                    }
                }
            }
            Key::Character(ref c) if c == " " => {
                event.prevent_default();
                if was_open {
                    ctx.confirm_highlighted();
                } else {
                    ctx.set_open(true);
                    let current = ctx.current_value();
                    if !current.is_empty() {
                        ctx.highlighted.set(Some(current));
                    } else {
                        ctx.highlight_first();
                    }
                }
            }
            Key::ArrowDown => {
                event.prevent_default();
                if !was_open {
                    ctx.set_open(true);
                    let current = ctx.current_value();
                    if !current.is_empty() {
                        ctx.highlighted.set(Some(current));
                    }
                    ctx.highlight_next();
                } else {
                    ctx.highlight_next();
                }
            }
            Key::ArrowUp => {
                event.prevent_default();
                if !was_open {
                    ctx.set_open(true);
                    let current = ctx.current_value();
                    if !current.is_empty() {
                        ctx.highlighted.set(Some(current));
                    }
                    ctx.highlight_prev();
                } else {
                    ctx.highlight_prev();
                }
            }
            Key::Home => {
                event.prevent_default();
                if !was_open {
                    ctx.set_open(true);
                }
                ctx.highlight_first();
            }
            Key::End => {
                event.prevent_default();
                if !was_open {
                    ctx.set_open(true);
                }
                ctx.highlight_last();
            }
            Key::Escape => {
                if was_open {
                    event.prevent_default();
                    ctx.set_open(false);
                }
            }
            Key::Tab => {
                // Close on Tab (do NOT prevent default — let focus move)
                if was_open {
                    ctx.confirm_highlighted();
                }
            }
            // Type-ahead for printable characters
            Key::Character(ref c) if c != " " => {
                event.prevent_default();
                if !was_open {
                    ctx.set_open(true);
                }
                ctx.type_ahead(c);
            }
            _ => {}
        }
    };

    let onclick = move |_: MouseEvent| {
        if !is_disabled {
            let mut ctx: SelectContext = consume_context();
            ctx.toggle_open();
            if ctx.is_open() {
                let current = ctx.current_value();
                if !current.is_empty() {
                    ctx.highlighted.set(Some(current));
                }
            }
        }
    };

    // Close on blur (click-outside).
    // Content uses `onmousedown: prevent_default()` which prevents the browser
    // from moving focus away when clicking inside the listbox. So blur only
    // fires when focus genuinely leaves the select — exactly when we want to close.
    let onblur = move |_: FocusEvent| {
        let mut ctx: SelectContext = consume_context();
        ctx.set_open(false);
    };

    rsx! {
        button {
            id: "{trigger_id}",
            role: "combobox",
            r#type: "button",
            aria_expanded: if is_open { "true" } else { "false" },
            aria_haspopup: "listbox",
            aria_controls: "{listbox_id}",
            aria_activedescendant: if !active_desc.is_empty() { "{active_desc}" },
            tabindex: "0",
            "data-state": state,
            "data-disabled": is_disabled.then_some("true"),
            disabled: is_disabled,
            onkeydown,
            onclick,
            onblur,
            ..attributes,
            {children}
        }
    }
}

// ── Value ───────────────────────────────────────────────────────────────────

/// Displays the current selected value text, or a placeholder.
///
/// In single-select mode, looks up the selected item's label.
/// In multi-select mode, consumers should provide their own rendering via
/// children (the component provides context access for selected values).
///
/// ## Data attributes
///
/// - `data-select-placeholder` — present when showing placeholder text
#[component]
pub fn Value(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Placeholder text shown when no value is selected.
    #[props(default)]
    placeholder: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    let ctx: SelectContext = use_context();

    let has_children = children != VNode::empty();

    // Determine display text for single-select mode
    let (display_text, is_placeholder) = if !has_children {
        if ctx.multiple {
            let vals = ctx.current_values();
            if vals.is_empty() {
                (placeholder.clone().unwrap_or_default(), true)
            } else {
                // Show comma-separated labels
                let items = ctx.items.read();
                let labels: Vec<String> = vals
                    .iter()
                    .filter_map(|v| {
                        items
                            .iter()
                            .find(|e| &e.value == v)
                            .map(|e| e.label.clone())
                    })
                    .collect();
                if labels.is_empty() {
                    (vals.join(", "), false)
                } else {
                    (labels.join(", "), false)
                }
            }
        } else {
            let current = ctx.current_value();
            if current.is_empty() {
                (placeholder.clone().unwrap_or_default(), true)
            } else {
                // Look up label from registered items
                let items = ctx.items.read();
                let label = items
                    .iter()
                    .find(|e| e.value == current)
                    .map(|e| e.label.clone())
                    .unwrap_or(current);
                (label, false)
            }
        }
    } else {
        (String::new(), false)
    };

    rsx! {
        span {
            "data-select-placeholder": is_placeholder.then_some("true"),
            ..attributes,
            if has_children {
                {children}
            } else {
                "{display_text}"
            }
        }
    }
}

// ── Input ───────────────────────────────────────────────────────────────────

/// Search input for the combobox variant.
///
/// Its presence inside `Root` switches the component from select-only to
/// combobox mode. Renders an `<input>` with `role="combobox"` and full
/// ARIA attributes.
///
/// ## Data attributes
///
/// - `data-select-input` — always present
#[component]
pub fn Input(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Placeholder text for the input.
    #[props(default)]
    placeholder: Option<String>,
) -> Element {
    let mut ctx: SelectContext = use_context();

    // Mark that we have an input (switches to combobox mode)
    use_hook(|| {
        ctx.mark_has_input();
    });

    let is_open = ctx.is_open();
    let input_id = ctx.input_id();
    let listbox_id = ctx.listbox_id();
    let active_desc = ctx.active_descendant();
    let autocomplete_attr = ctx.autocomplete.as_aria_attr();
    let is_disabled = ctx.disabled;

    let oninput = move |evt: Event<FormData>| {
        let mut ctx: SelectContext = consume_context();
        ctx.search_query.set(evt.value());
        if !ctx.is_open() {
            ctx.set_open(true);
        }
        // Reset highlight to first match
        ctx.highlight_first();
    };

    let onkeydown = move |event: KeyboardEvent| {
        if is_disabled {
            return;
        }
        let mut ctx: SelectContext = consume_context();
        let was_open = ctx.is_open();

        match event.key() {
            Key::ArrowDown => {
                event.prevent_default();
                if event.modifiers().alt() {
                    // Alt+ArrowDown: open without highlighting
                    if !was_open {
                        ctx.set_open(true);
                    }
                } else if !was_open {
                    ctx.set_open(true);
                    ctx.highlight_first();
                } else {
                    ctx.highlight_next();
                }
            }
            Key::ArrowUp => {
                if was_open {
                    event.prevent_default();
                    ctx.highlight_prev();
                }
            }
            Key::Enter => {
                if was_open && ctx.highlighted.read().is_some() {
                    event.prevent_default();
                    ctx.confirm_highlighted();
                }
            }
            Key::Escape => {
                if was_open {
                    event.prevent_default();
                    ctx.set_open(false);
                }
            }
            // Home/End: let the input handle cursor movement (no preventDefault)
            Key::Tab => {
                if was_open {
                    ctx.set_open(false);
                }
            }
            _ => {}
        }
    };

    let onfocus = move |_: FocusEvent| {
        let mut ctx: SelectContext = consume_context();
        if ctx.open_on_focus && !ctx.disabled && !ctx.is_open() {
            ctx.set_open(true);
            ctx.highlight_first();
        }
    };

    let onblur = move |_: FocusEvent| {
        let mut ctx: SelectContext = consume_context();
        ctx.set_open(false);
    };

    rsx! {
        input {
            id: "{input_id}",
            r#type: "text",
            role: "combobox",
            aria_expanded: if is_open { "true" } else { "false" },
            aria_haspopup: "listbox",
            aria_controls: "{listbox_id}",
            aria_activedescendant: if !active_desc.is_empty() { "{active_desc}" },
            aria_autocomplete: "{autocomplete_attr}",
            disabled: is_disabled,
            placeholder: placeholder,
            value: "{ctx.search_query}",
            "data-select-input": "true",
            oninput,
            onkeydown,
            onfocus,
            onblur,
            ..attributes,
        }
    }
}

// ── ClearButton ─────────────────────────────────────────────────────────────

/// Button to clear the search query.
///
/// ## Data attributes
///
/// - `data-select-clear` — always present
#[component]
pub fn ClearButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let onclick = move |evt: MouseEvent| {
        evt.prevent_default();
        let mut ctx: SelectContext = consume_context();
        ctx.search_query.set(String::new());
        ctx.focus_combobox();
    };

    rsx! {
        button {
            r#type: "button",
            aria_label: "Clear",
            tabindex: "-1",
            "data-select-clear": "true",
            onclick,
            ..attributes,
            {children}
        }
    }
}

// ── Content ─────────────────────────────────────────────────────────────────

/// The popup containing the listbox of options.
///
/// Only renders when the select is open.
///
/// ## Data attributes
///
/// - `data-select-content` — always present
/// - `data-state="open|closed"`
#[component]
pub fn Content(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Optional accessible label for the listbox.
    #[props(default)]
    aria_label: Option<String>,
    children: Element,
) -> Element {
    let ctx: SelectContext = use_context();

    if !ctx.is_open() {
        return rsx! {};
    }

    let listbox_id = ctx.listbox_id();
    let multi = ctx.multiple;

    rsx! {
        div {
            id: "{listbox_id}",
            role: "listbox",
            aria_label: aria_label,
            aria_multiselectable: if multi { "true" } else { "false" },
            "data-select-content": "true",
            "data-state": "open",
            // Prevent focus leaving the combobox when clicking inside content
            onmousedown: |evt: MouseEvent| { evt.prevent_default(); },
            ..attributes,
            {children}
        }
    }
}

// ── Item ────────────────────────────────────────────────────────────────────

/// A single selectable option in the listbox.
///
/// Registers itself with the context on mount and deregisters on unmount.
/// Only renders if it passes the current filter.
///
/// ## Data attributes
///
/// - `data-state="checked|unchecked"`
/// - `data-highlighted` — present when this item has visual focus
/// - `data-disabled="true"` — when disabled
#[component]
pub fn Item(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Unique value identifying this option.
    value: String,
    /// Searchable text label. Falls back to `value` if not provided.
    #[props(default)]
    label: Option<String>,
    /// Additional keywords for fuzzy matching (space-separated).
    #[props(default)]
    keywords: Option<String>,
    /// Prevent selection and skip in keyboard navigation.
    #[props(default)]
    disabled: bool,
    children: Element,
) -> Element {
    let mut ctx: SelectContext = use_context();
    let display_label = label.clone().unwrap_or_else(|| value.clone());
    let val = value.clone();

    // Get group context if nested inside a Group
    let group_id = try_use_context::<GroupContext>().map(|g| g.id.clone());

    // Register on mount
    use_hook(|| {
        ctx.register_item(ItemEntry {
            value: val.clone(),
            label: display_label.clone(),
            keywords: keywords.clone().unwrap_or_default(),
            disabled,
            group_id: group_id.clone(),
        });
    });
    let val_drop = value.clone();
    use_drop(move || {
        let mut ctx: SelectContext = consume_context();
        ctx.deregister_item(&val_drop);
    });

    // Check if this item is visible (passes filter)
    let visible = ctx.visible_values.read();
    if !visible.iter().any(|v| v == &value) {
        return rsx! {};
    }

    let is_selected = ctx.is_selected(&value);
    let is_highlighted = ctx.highlighted.read().as_deref() == Some(value.as_str());
    let item_id = ctx.item_id(&value);
    let state = if is_selected { "checked" } else { "unchecked" };

    // Provide inner context for ItemIndicator
    let item_ctx = ItemContext {
        value: value.clone(),
    };
    use_context_provider(|| item_ctx);

    let val_click = value.clone();
    let onmousedown = move |evt: MouseEvent| {
        evt.prevent_default();
        if !disabled {
            let mut ctx: SelectContext = consume_context();
            if ctx.multiple {
                ctx.toggle_value(&val_click);
            } else {
                ctx.select_single(&val_click);
            }
        }
    };

    let val_enter = value.clone();
    let onpointerenter = move |_| {
        let mut ctx: SelectContext = consume_context();
        ctx.highlighted.set(Some(val_enter.clone()));
    };

    rsx! {
        div {
            id: "{item_id}",
            role: "option",
            aria_selected: if is_selected { "true" } else { "false" },
            aria_disabled: disabled.then_some("true"),
            "data-state": state,
            "data-highlighted": is_highlighted.then_some("true"),
            "data-disabled": disabled.then_some("true"),
            onmousedown,
            onpointerenter,
            ..attributes,
            {children}
        }
    }
}

// ── ItemText ────────────────────────────────────────────────────────────────

/// The text content of an option item.
///
/// ## Data attributes
///
/// - `data-select-item-text` — always present
#[component]
pub fn ItemText(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        span {
            "data-select-item-text": "true",
            ..attributes,
            {children}
        }
    }
}

// ── ItemIndicator ───────────────────────────────────────────────────────────

/// Renders its children only when the parent [`Item`] is selected.
///
/// Requires being nested inside an [`Item`] component.
///
/// ## Data attributes
///
/// - `data-select-item-indicator` — always present
#[component]
pub fn ItemIndicator(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: SelectContext = use_context();
    let item_ctx: ItemContext = use_context();

    if !ctx.is_selected(&item_ctx.value) {
        return rsx! {};
    }

    rsx! {
        span {
            "data-select-item-indicator": "true",
            ..attributes,
            {children}
        }
    }
}

// ── Group ───────────────────────────────────────────────────────────────────

/// Groups related options with an optional label.
///
/// ## Data attributes
///
/// - `data-select-group` — always present
#[component]
pub fn Group(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    /// Unique group identifier.
    id: String,
    /// Optional heading for the group (used for `aria-labelledby`).
    #[props(default)]
    label: Option<String>,
    children: Element,
) -> Element {
    let mut ctx: SelectContext = use_context();
    let group_id = id.clone();

    // Register on mount
    use_hook(|| {
        ctx.register_group(GroupEntry {
            id: group_id.clone(),
            label: label.clone(),
        });
    });
    let id_drop = id.clone();
    use_drop(move || {
        let mut ctx: SelectContext = consume_context();
        ctx.deregister_group(&id_drop);
    });

    // Provide group context for child items
    let group_ctx = GroupContext { id: id.clone() };
    use_context_provider(|| group_ctx);

    let label_id = if label.is_some() {
        Some(ctx.group_label_id(&id))
    } else {
        None
    };

    rsx! {
        div {
            role: "group",
            aria_labelledby: label_id,
            "data-select-group": "true",
            ..attributes,
            {children}
        }
    }
}

// ── Label ───────────────────────────────────────────────────────────────────

/// Heading label for a [`Group`].
///
/// ## Data attributes
///
/// - `data-select-label` — always present
#[component]
pub fn Label(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: SelectContext = use_context();
    let group_ctx: GroupContext = use_context();
    let label_id = ctx.group_label_id(&group_ctx.id);

    rsx! {
        div {
            id: "{label_id}",
            "data-select-label": "true",
            ..attributes,
            {children}
        }
    }
}

// ── Separator ───────────────────────────────────────────────────────────────

/// Visual separator between items or groups.
///
/// ## Data attributes
///
/// - `data-select-separator` — always present
#[component]
pub fn Separator(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    rsx! {
        div {
            role: "separator",
            aria_orientation: "horizontal",
            "data-select-separator": "true",
            ..attributes,
        }
    }
}

// ── Empty ───────────────────────────────────────────────────────────────────

/// Rendered when no items match the current filter query.
///
/// ## Data attributes
///
/// - `data-select-empty` — always present
#[component]
pub fn Empty(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: SelectContext = use_context();

    if !ctx.visible_values.read().is_empty() {
        return rsx! {};
    }

    // Don't show when there's no query (all items are visible when unfiltered)
    if ctx.search_query.read().is_empty() {
        return rsx! {};
    }

    // Don't show if no items have registered yet — they may still be mounting
    if ctx.items.read().is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            role: "status",
            "data-select-empty": "true",
            ..attributes,
            {children}
        }
    }
}
