//! Convenience wrapper that pre-wires `dioxus-nox-select` with `tag-input`
//! for the common "searchable multi-select with pills" pattern.
//!
//! Enabled by the `combobox` feature (on by default).
//!
//! # Example
//!
//! ```ignore
//! use dioxus_nox_tag_input::combo;
//!
//! combo::Root::<MyTag> {
//!     available_tags: tags,
//!     combo::Control::<MyTag> { /* pills + input */ }
//!     combo::Dropdown { /* select::Item options */ }
//! }
//! ```

use std::option::Option;

use dioxus::prelude::*;
use dioxus_nox_select::{select, AutoComplete, SelectContext};

use crate::components as tag_input;
use crate::hook::{TagInputState, extract_clipboard_text, is_denied};
use crate::tag::TagLike;

/// Shared available tags, provided via context by ComboWiring.
#[derive(Clone)]
struct ComboAvailable<T: TagLike + 'static>(Signal<Vec<T>>);

/// Values that should be disabled in the dropdown (computed from deny_list).
#[derive(Clone)]
struct ComboDisabledValues(Memo<Vec<String>>);

/// Configuration for combo behavior.
#[derive(Clone, Copy)]
struct ComboConfig {
    close_on_select: bool,
}

/// Props for the combo root component.
#[derive(Props, Clone, PartialEq)]
pub struct RootProps<T: TagLike + 'static> {
    /// Tags available for selection.
    pub available_tags: Vec<T>,
    /// Initially selected tags (uncontrolled).
    #[props(default)]
    pub initial_selected: Vec<T>,
    /// Whether the component is disabled.
    #[props(default)]
    pub disabled: bool,
    /// Placeholder text for the search input.
    #[props(default = "Type to search\u{2026}".to_string())]
    pub placeholder: String,
    /// Optional callback for creating new tags from typed text.
    #[props(default)]
    pub on_create: Option<Callback<String, Option<T>>>,
    /// Maximum number of tags.
    #[props(default)]
    pub max_tags: Option<usize>,
    /// Whether to allow duplicates.
    #[props(default)]
    pub allow_duplicates: bool,
    /// Tag names to block from selection (case-insensitive).
    #[props(default)]
    pub deny_list: Option<Vec<String>>,
    /// Whether to close the dropdown after selecting an item (default: true).
    #[props(default = true)]
    pub close_on_select: bool,
    /// Fire-and-forget notification after a tag is successfully added.
    #[props(default)]
    pub on_add: Option<EventHandler<T>>,
    /// Fire-and-forget notification after a tag is removed.
    #[props(default)]
    pub on_remove: Option<EventHandler<T>>,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

/// Combined select + tag-input root.
///
/// Creates both a `tag_input::Root` and a `select::Root` internally,
/// wiring the select's value changes to `tag_state.add_tag()` and
/// the tag-input's search query to the select's filter.
#[allow(non_snake_case)]
pub fn Root<T: TagLike>(props: RootProps<T>) -> Element {
    let available = props.available_tags.clone();
    let initial = props.initial_selected.clone();
    let placeholder = props.placeholder.clone();
    let deny_list = props.deny_list.clone();
    let close_on_select = props.close_on_select;

    rsx! {
        tag_input::Root::<T> {
            available_tags: available.clone(),
            initial_selected: initial,
            disabled: props.disabled,
            max_tags: props.max_tags,
            allow_duplicates: props.allow_duplicates,
            deny_list: deny_list,
            on_create: props.on_create,
            on_add: props.on_add,
            on_remove: props.on_remove,

            select::Root {
                multiple: true,
                disabled: props.disabled,
                autocomplete: AutoComplete::List,

                ComboWiring::<T> {
                    available: available,
                    placeholder: placeholder,
                    close_on_select: close_on_select,
                    {props.children}
                }
            }
        }
    }
}

/// Internal wiring component that bridges select and tag-input contexts.
#[component]
fn ComboWiring<T: TagLike>(
    available: Vec<T>,
    placeholder: String,
    #[props(default = true)]
    close_on_select: bool,
    children: Element,
) -> Element {
    let mut tag_ctx = use_context::<TagInputState<T>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Share available tags via context so combo::Input can access them.
    let available_sig = use_signal(|| available.clone());
    use_context_provider(|| ComboAvailable::<T>(available_sig));

    // Provide combo config
    use_context_provider(|| ComboConfig { close_on_select });

    // Compute disabled values from tag_ctx.deny_list + available tags
    let avail_for_deny = available.clone();
    let denied_values = use_memo(move || {
        let deny = tag_ctx.deny_list.read();
        match &*deny {
            Some(deny_list) => avail_for_deny
                .iter()
                .filter(|t| is_denied(t.name(), deny_list))
                .map(|t| t.id().to_string())
                .collect(),
            None => Vec::new(),
        }
    });
    use_context_provider(|| ComboDisabledValues(denied_values));

    // Forward sync: select values ↔ tag-input (for mouse clicks on dropdown items)
    // select::Item's onmousedown calls toggle_value(), so we detect changes here.
    // Use peek() for selected_tags to avoid subscribing — this effect should only
    // re-run when select values change, not when tags change (which would loop).
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = tag_ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();

        let mut changed = false;

        // Add tags for newly selected values
        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = available.iter().find(|t| t.id() == val.as_str())
            {
                tag_ctx.add_tag(tag.clone());
                changed = true;
            }
        }

        // Remove tags that were deselected in dropdown (toggle off)
        for tag_id in &tag_ids {
            if !selected_values.contains(tag_id) {
                tag_ctx.remove_tag(tag_id);
                changed = true;
            }
        }

        // Close dropdown after selection change if configured
        if changed
            && try_use_context::<ComboConfig>().is_none_or(|c| c.close_on_select)
        {
            select_ctx.set_open(false);
        }
    });

    // Reverse sync: tag-input removals → select values
    // When a tag is removed via pill X button, update select's values to match.
    // Subscribe to selected_tags (read) but peek select values (no subscription).
    use_effect(move || {
        let tag_ids: Vec<String> = tag_ctx
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        let select_values = select_ctx.current_values_peek();

        // Toggle off any select values that are no longer in tag-input
        for val in &select_values {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

// ── ComboInput: bridges select + tag-input keyboard handling ──────────────

/// Props for [`Input`].
#[derive(Props, Clone, PartialEq)]
pub struct InputProps<T: TagLike + 'static> {
    #[props(default = "Type to search\u{2026}".to_string())]
    pub placeholder: String,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Toggle the highlighted tag: add if not selected, remove if already selected.
fn toggle_highlighted_tag<T: TagLike>(
    select_ctx: &mut SelectContext,
    tag_ctx: &mut TagInputState<T>,
    available: &[T],
) {
    let highlighted = select_ctx.highlighted_value();
    if let Some(val) = highlighted
        && let Some(tag) = available.iter().find(|t| t.id() == val.as_str())
    {
        if select_ctx.is_selected(&val) {
            // Already selected — remove
            tag_ctx.remove_tag(&val);
            select_ctx.toggle_value(&val);
        } else {
            // Not selected — add
            tag_ctx.add_tag(tag.clone());
            select_ctx.toggle_value(&val);
        }
    }
}

/// Combined search input that handles both select dropdown navigation
/// (ArrowUp/Down, Enter to confirm) and tag-input pill navigation
/// (ArrowLeft, Backspace, Escape, delimiter chars).
#[allow(non_snake_case)]
pub fn Input<T: TagLike>(props: InputProps<T>) -> Element {
    let mut tag_ctx = use_context::<TagInputState<T>>();
    let mut select_ctx = use_context::<SelectContext>();
    let combo_available = use_context::<ComboAvailable<T>>();

    // Mark select as having a combobox input on mount
    use_hook(|| {
        select_ctx.mark_has_input();
    });

    let listbox_id = select_ctx.listbox_id();
    let input_id = select_ctx.input_id();

    rsx! {
        input {
            r#type: "text",
            id: "{input_id}",
            role: "combobox",
            disabled: *tag_ctx.is_disabled.read(),
            readonly: *tag_ctx.is_readonly.read(),
            placeholder: "{props.placeholder}",
            value: "{tag_ctx.search_query}",
            autocomplete: "off",
            aria_autocomplete: select_ctx.autocomplete().as_aria_attr(),
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            "data-slot": "input",
            "data-select-input": "true",
            "data-disabled": *tag_ctx.is_disabled.read(),
            "data-readonly": *tag_ctx.is_readonly.read(),
            "data-placeholder-shown": tag_ctx.search_query.read().is_empty(),

            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                // Update both contexts
                tag_ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                // Open dropdown on typing
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                // Reset highlight to first match
                select_ctx.highlight_first();
            },

            onkeydown: move |evt: Event<KeyboardData>| {
                let key = evt.key();
                match key {
                    // ── Dropdown navigation (select context) ──────────
                    Key::ArrowDown => {
                        evt.prevent_default();
                        if !select_ctx.is_open() {
                            select_ctx.set_open(true);
                            select_ctx.highlight_first();
                        } else {
                            select_ctx.highlight_next();
                        }
                    }
                    Key::ArrowUp => {
                        if select_ctx.is_open() {
                            evt.prevent_default();
                            select_ctx.highlight_prev();
                        }
                    }
                    Key::Enter => {
                        evt.prevent_default();
                        if select_ctx.is_open()
                            && select_ctx.has_highlighted()
                        {
                            // Directly add the highlighted tag
                            let available = combo_available.0.read();
                            toggle_highlighted_tag(&mut select_ctx, &mut tag_ctx, &available);
                            // Clear query after selection
                            tag_ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                            // Close dropdown if configured
                            if try_use_context::<ComboConfig>().is_none_or(|c| c.close_on_select) {
                                select_ctx.set_open(false);
                            }
                        } else {
                            // No highlighted item — delegate to tag-input
                            // (handles on_create / on_commit)
                            tag_ctx.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        tag_ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        // Close dropdown, let focus move naturally
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    // ── Tag-input pill navigation ────────────────────
                    Key::ArrowLeft | Key::Backspace => {
                        // Delegate to tag-input for pill mode entry
                        tag_ctx.handle_input_keydown(evt);
                    }
                    _ => {
                        // Delegate to tag-input for delimiter handling etc
                        tag_ctx.handle_input_keydown(evt);
                    }
                }
            },

            onfocus: move |_| {
                if select_ctx.open_on_focus() {
                    select_ctx.set_open(true);
                    select_ctx.highlight_first();
                }
            },

            onblur: move |_| {
                select_ctx.set_open(false);
            },

            onclick: move |_| {
                tag_ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },

            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    tag_ctx.handle_paste(text);
                }
            },

            ..props.attributes,
        }
    }
}

// ── Re-exported compound parts ──────────────────────────────────────────────

/// Tag pill list.
pub use tag_input::TagList;
/// Individual tag pill.
pub use tag_input::Tag;
/// Tag remove button.
pub use tag_input::TagRemove;
/// Control area.
pub use tag_input::Control;
/// Hidden form value.
pub use tag_input::FormValue;
/// Overflow count badge.
pub use tag_input::Count;
/// Screen reader announcements.
pub use tag_input::LiveRegion;

/// Dropdown content — wraps `select::Content`.
pub use select::Content as Dropdown;
/// Dropdown group — wraps `select::Group`.
pub use select::Group as DropdownGroup;
/// Dropdown group label — wraps `select::Label`.
pub use select::Label;
/// Empty state — wraps `select::Empty`.
pub use select::Empty;

/// Dropdown option that automatically merges deny_list disabled state.
///
/// Wraps `select::Item`, adding `disabled` when the item's value appears
/// in the combo root's deny_list.
#[allow(non_snake_case)]
pub fn Item(props: ItemProps) -> Element {
    let effective_disabled = if let Some(combo_disabled) = try_use_context::<ComboDisabledValues>() {
        props.disabled || combo_disabled.0.read().contains(&props.value)
    } else {
        props.disabled
    };

    // Build select::Item props and call its component function directly,
    // since RSX `..attributes` spread only works on HTML elements.
    select::Item(dioxus_nox_select::select::ItemProps {
        attributes: props.attributes,
        value: props.value,
        label: props.label,
        keywords: props.keywords,
        disabled: effective_disabled,
        children: props.children,
    })
}

/// Props for [`Item`].
#[derive(Props, Clone, PartialEq)]
pub struct ItemProps {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// Unique value identifying this option.
    pub value: String,
    /// Searchable text label. Falls back to `value` if not provided.
    #[props(default)]
    pub label: Option<String>,
    /// Additional keywords for fuzzy matching (space-separated).
    #[props(default)]
    pub keywords: Option<String>,
    /// Prevent selection and skip in keyboard navigation.
    #[props(default)]
    pub disabled: bool,
    pub children: Element,
}
