use crate::hook::{TagInputConfig, TagInputGroupConfig, use_tag_input_grouped, use_tag_input_with};
use crate::tag::TagLike;
use dioxus::prelude::*;
use std::cmp::Ordering;

/// Props for [`Root`].
///
/// Accepts all tag input configuration: available tags, constraints, callbacks,
/// grouping, and controlled-mode signals. Spreads extra attributes onto the outer `<div>`.
#[derive(Props, Clone, PartialEq)]
#[allow(clippy::type_complexity, unpredictable_function_pointer_comparisons)]
pub struct RootProps<T: TagLike + 'static> {
    pub available_tags: Vec<T>,
    #[props(default)]
    pub initial_selected: Vec<T>,

    // Controlled mode props
    #[props(default)]
    pub value: Option<Signal<Vec<T>>>,
    #[props(default)]
    pub query: Option<Signal<String>>,

    // Config props
    #[props(default)]
    pub max_tags: Option<usize>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub readonly: bool,
    #[props(default)]
    pub allow_duplicates: bool,
    #[props(default)]
    pub enforce_allow_list: bool,
    #[props(default)]
    pub select_mode: bool,
    #[props(default)]
    pub deny_list: Option<Vec<String>>,
    #[props(default)]
    pub paste_delimiters: Option<Vec<char>>,
    #[props(default)]
    pub delimiters: Option<Vec<char>>,
    #[props(default)]
    pub min_tags: Option<usize>,
    #[props(default)]
    pub max_tag_length: Option<usize>,
    #[props(default)]
    pub max_visible_tags: Option<usize>,
    #[props(default)]
    pub filter: Option<fn(&T, &str) -> bool>,
    #[props(default)]
    pub sort_selected: Option<fn(&T, &T) -> Ordering>,
    #[props(default)]
    pub validate: Option<Callback<T, Result<(), String>>>,

    // Callback props
    //
    // `Callback<In, Out>` is used when the hook needs a return value from the
    // consumer (e.g. the created tag, parsed paste results, or an edited tag).
    // `EventHandler<T>` is fire-and-forget — the hook does not use the return.
    /// Called when the user presses Enter with a non-empty query and no matching
    /// suggestion. Returns `Some(T)` to add the created tag, or `None` to reject.
    #[props(default)]
    pub on_create: Option<Callback<String, Option<T>>>,
    /// Fire-and-forget notification after a tag is successfully added.
    #[props(default)]
    pub on_add: Option<EventHandler<T>>,
    /// Fire-and-forget notification after a tag is removed.
    #[props(default)]
    pub on_remove: Option<EventHandler<T>>,
    /// Fire-and-forget notification when a duplicate tag addition is attempted.
    #[props(default)]
    pub on_duplicate: Option<EventHandler<T>>,
    /// Called when the user pastes text. Returns the parsed `Vec<T>` to add.
    #[props(default)]
    pub on_paste: Option<Callback<String, Vec<T>>>,
    /// Called when a pill edit is committed. Returns `Some(T)` with the updated
    /// tag, or `None` to cancel the edit.
    #[props(default)]
    pub on_edit: Option<Callback<(T, String), Option<T>>>,
    /// Fire-and-forget notification when tags are reordered (from_index, to_index).
    #[props(default)]
    pub on_reorder: Option<EventHandler<(usize, usize)>>,
    /// Fire-and-forget notification when the search query changes.
    #[props(default)]
    pub on_query_change: Option<EventHandler<String>>,
    /// Fire-and-forget notification when user commits text without on_create.
    #[props(default)]
    pub on_commit: Option<EventHandler<String>>,

    // Grouping config
    #[props(default)]
    pub sort_items: Option<fn(&T, &T) -> Ordering>,
    #[props(default)]
    pub sort_groups: Option<fn(&str, &str) -> Ordering>,
    #[props(default)]
    pub max_items_per_group: Option<usize>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

/// Top-level provider that creates tag input state and shares it with children via context.
///
/// Renders a `<div role="group">` with data attributes:
/// `data-disabled`, `data-readonly`, `data-state` ("valid"/"invalid"),
/// `data-at-limit`, `data-below-minimum`.
pub fn Root<T: TagLike>(props: RootProps<T>) -> Element {
    let has_grouping = props.sort_items.is_some()
        || props.sort_groups.is_some()
        || props.max_items_per_group.is_some();

    let mut state = if has_grouping {
        use_tag_input_grouped(TagInputGroupConfig {
            available_tags: props.available_tags.clone(),
            initial_selected: props.initial_selected.clone(),
            filter: props.filter,
            sort_items: props.sort_items,
            sort_groups: props.sort_groups,
            max_items_per_group: props.max_items_per_group,
            value: props.value,
            query: props.query,
        })
    } else {
        use_tag_input_with(TagInputConfig {
            available_tags: props.available_tags.clone(),
            initial_selected: props.initial_selected.clone(),
            value: props.value,
            query: props.query,
        })
    };

    // Sync config props to state signals. use_effect runs on each render
    // with the latest prop values.
    let max_tags = props.max_tags;
    let disabled = props.disabled;
    let readonly = props.readonly;
    let allow_duplicates = props.allow_duplicates;
    let enforce_allow_list = props.enforce_allow_list;
    let select_mode = props.select_mode;
    let deny_list = props.deny_list.clone();
    let paste_delimiters = props.paste_delimiters.clone();
    let delimiters = props.delimiters.clone();
    let min_tags = props.min_tags;
    let max_tag_length = props.max_tag_length;
    let max_visible_tags = props.max_visible_tags;
    let filter = props.filter;
    let sort_selected = props.sort_selected;
    let validate = props.validate;
    let on_create = props.on_create;
    let on_paste = props.on_paste;
    let on_edit = props.on_edit;
    let on_add = props.on_add;
    let on_remove = props.on_remove;
    let on_duplicate = props.on_duplicate;
    let on_reorder = props.on_reorder;
    let on_query_change = props.on_query_change;
    let on_commit = props.on_commit;

    use_effect(move || {
        state.max_tags.set(max_tags);
        state.is_disabled.set(disabled);
        state.is_readonly.set(readonly);
        state.allow_duplicates.set(allow_duplicates);
        state.enforce_allow_list.set(enforce_allow_list);
        state.select_mode.set(select_mode);
        state.deny_list.set(deny_list.clone());
        state.paste_delimiters.set(paste_delimiters.clone());
        state.delimiters.set(delimiters.clone());
        state.min_tags.set(min_tags);
        state.max_tag_length.set(max_tag_length);
        state.max_visible_tags.set(max_visible_tags);
        state.filter.set(filter);
        state.sort_selected.set(sort_selected);

        // Wire validate
        if let Some(cb) = validate {
            state.validate.set(Some(cb));
        } else {
            state.validate.set(None);
        }

        // Wire callbacks
        if let Some(cb) = on_create {
            state.on_create.set(Some(cb));
        } else {
            state.on_create.set(None);
        }

        if let Some(cb) = on_paste {
            state.on_paste.set(Some(cb));
        } else {
            state.on_paste.set(None);
        }

        if let Some(cb) = on_edit {
            state.on_edit.set(Some(cb));
        } else {
            state.on_edit.set(None);
        }

        // Wire EventHandler callbacks by converting to Callback
        if let Some(handler) = on_add {
            state.on_add.set(Some(Callback::new(move |tag: T| {
                handler.call(tag);
            })));
        } else {
            state.on_add.set(None);
        }

        if let Some(handler) = on_remove {
            state.on_remove.set(Some(Callback::new(move |tag: T| {
                handler.call(tag);
            })));
        } else {
            state.on_remove.set(None);
        }

        if let Some(handler) = on_duplicate {
            state.on_duplicate.set(Some(Callback::new(move |tag: T| {
                handler.call(tag);
            })));
        } else {
            state.on_duplicate.set(None);
        }

        if let Some(handler) = on_reorder {
            state
                .on_reorder
                .set(Some(Callback::new(move |pair: (usize, usize)| {
                    handler.call(pair);
                })));
        } else {
            state.on_reorder.set(None);
        }

        // Wire new EventHandler callbacks directly
        state.on_query_change.set(on_query_change);
        state.on_commit.set(on_commit);
    });

    // Provide state via context for child components
    use_context_provider(|| state);

    rsx! {
        div {
            "data-slot": "root",
            role: "group",
            "data-disabled": *state.is_disabled.read(),
            "data-readonly": *state.is_readonly.read(),
            "data-state": if state.validation_error.read().is_some() { "invalid" } else { "valid" },
            "data-at-limit": *state.is_at_limit.read(),
            "data-below-minimum": *state.is_below_minimum.read(),
            ..props.attributes,
            {props.children}
        }
    }
}
