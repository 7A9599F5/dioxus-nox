use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use nucleo_matcher::{Config, Matcher};

use crate::filter;
use crate::navigation::{self, Direction};
use crate::types::*;

/// Shared context for the select compound component tree.
///
/// Provided by [`super::select::Root`] and consumed by all child components.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct SelectContext {
    // ── Value state (single-select) ──────────────────────────
    pub(crate) value: Signal<String>,
    pub(crate) controlled_value: Option<Signal<String>>,

    // ── Value state (multi-select) ───────────────────────────
    pub(crate) values: Signal<Vec<String>>,
    pub(crate) controlled_values: Option<Signal<Vec<String>>>,

    // ── Open state ───────────────────────────────────────────
    pub(crate) open: Signal<bool>,
    pub(crate) controlled_open: Option<Signal<bool>>,

    // ── Search / filter ──────────────────────────────────────
    pub(crate) search_query: Signal<String>,
    pub(crate) scored_items: Memo<Vec<ScoredItem>>,
    pub(crate) visible_values: Memo<Vec<String>>,

    // ── Highlight (visual focus in listbox) ──────────────────
    pub(crate) highlighted: Signal<Option<String>>,

    // ── Registration ─────────────────────────────────────────
    pub(crate) items: Signal<Vec<ItemEntry>>,
    pub(crate) groups: Signal<Vec<GroupEntry>>,

    // ── Configuration ────────────────────────────────────────
    pub(crate) multiple: bool,
    pub(crate) disabled: bool,
    pub(crate) autocomplete: AutoComplete,
    pub(crate) open_on_focus: bool,
    pub(crate) custom_filter: Signal<Option<CustomFilter>>,

    // ── Callbacks ────────────────────────────────────────────
    pub(crate) on_value_change: Option<EventHandler<String>>,
    pub(crate) on_values_change: Option<EventHandler<Vec<String>>>,
    pub(crate) on_open_change: Option<EventHandler<bool>>,

    // ── Identity ─────────────────────────────────────────────
    pub(crate) instance_id: u32,

    /// Whether an `Input` child has been mounted (makes this a combobox).
    pub(crate) has_input: Signal<bool>,
}

impl SelectContext {
    // ── Value read/write ─────────────────────────────────────

    /// Current single-select value.
    pub fn current_value(&self) -> String {
        match self.controlled_value {
            Some(sig) => (sig)(),
            None => (self.value)(),
        }
    }

    /// Current multi-select values.
    pub fn current_values(&self) -> Vec<String> {
        match self.controlled_values {
            Some(sig) => (sig)(),
            None => (self.values)(),
        }
    }

    /// Current multi-select values without subscribing (peek).
    pub fn current_values_peek(&self) -> Vec<String> {
        match self.controlled_values {
            Some(sig) => sig.peek().clone(),
            None => self.values.peek().clone(),
        }
    }

    /// Select a value in single-select mode: sets value, closes popup, fires callback.
    pub fn select_single(&mut self, val: &str) {
        if self.disabled {
            return;
        }
        if let Some(mut controlled) = self.controlled_value {
            controlled.set(val.to_string());
        } else {
            self.value.set(val.to_string());
        }
        if let Some(handler) = &self.on_value_change {
            handler.call(val.to_string());
        }
        self.set_open(false);
        self.search_query.set(String::new());
    }

    /// Toggle a value in multi-select mode: adds if absent, removes if present.
    /// Does NOT close the popup.
    pub fn toggle_value(&mut self, val: &str) {
        if self.disabled {
            return;
        }
        let mut current = self.current_values();
        if let Some(pos) = current.iter().position(|v| v == val) {
            current.remove(pos);
        } else {
            current.push(val.to_string());
        }
        if let Some(mut controlled) = self.controlled_values {
            controlled.set(current.clone());
        } else {
            self.values.set(current.clone());
        }
        if let Some(handler) = &self.on_values_change {
            handler.call(current);
        }
    }

    /// Check if a value is currently selected.
    pub fn is_selected(&self, val: &str) -> bool {
        if self.multiple {
            self.current_values().iter().any(|v| v == val)
        } else {
            self.current_value() == val
        }
    }

    // ── Open/close ───────────────────────────────────────────

    /// Whether the popup is currently open.
    pub fn is_open(&self) -> bool {
        match self.controlled_open {
            Some(sig) => (sig)(),
            None => (self.open)(),
        }
    }

    /// Set the open state.
    pub fn set_open(&mut self, is_open: bool) {
        if let Some(mut controlled) = self.controlled_open {
            controlled.set(is_open);
        } else {
            self.open.set(is_open);
        }
        if let Some(handler) = &self.on_open_change {
            handler.call(is_open);
        }
        if !is_open {
            self.highlighted.set(None);
        }
    }

    /// Toggle the open state.
    pub fn toggle_open(&mut self) {
        let current = self.is_open();
        self.set_open(!current);
    }

    // ── Highlight navigation ─────────────────────────────────

    /// Move highlight to the next visible non-disabled item.
    pub fn highlight_next(&mut self) {
        let visible = self.visible_values.read();
        let items = self.items.read();
        let current = self.highlighted.read();
        let next = navigation::navigate(&items, &visible, current.as_deref(), Direction::Forward);
        drop(visible);
        drop(items);
        drop(current);
        self.highlighted.set(next.clone());
        if let Some(ref val) = next {
            self.scroll_item_into_view(val);
        }
    }

    /// Move highlight to the previous visible non-disabled item.
    pub fn highlight_prev(&mut self) {
        let visible = self.visible_values.read();
        let items = self.items.read();
        let current = self.highlighted.read();
        let prev = navigation::navigate(&items, &visible, current.as_deref(), Direction::Backward);
        drop(visible);
        drop(items);
        drop(current);
        self.highlighted.set(prev.clone());
        if let Some(ref val) = prev {
            self.scroll_item_into_view(val);
        }
    }

    /// Move highlight to the first visible non-disabled item.
    pub fn highlight_first(&mut self) {
        let visible = self.visible_values.read();
        let items = self.items.read();
        let target = navigation::first(&items, &visible);
        drop(visible);
        drop(items);
        self.highlighted.set(target.clone());
        if let Some(ref val) = target {
            self.scroll_item_into_view(val);
        }
    }

    /// Move highlight to the last visible non-disabled item.
    pub fn highlight_last(&mut self) {
        let visible = self.visible_values.read();
        let items = self.items.read();
        let target = navigation::last(&items, &visible);
        drop(visible);
        drop(items);
        self.highlighted.set(target.clone());
        if let Some(ref val) = target {
            self.scroll_item_into_view(val);
        }
    }

    /// Type-ahead: find the first matching item by prefix.
    pub fn type_ahead(&mut self, prefix: &str) {
        let visible = self.visible_values.read();
        let items = self.items.read();
        let current = self.highlighted.read();
        let target = navigation::type_ahead(&items, &visible, current.as_deref(), prefix);
        drop(visible);
        drop(items);
        drop(current);
        if let Some(ref val) = target {
            self.highlighted.set(Some(val.clone()));
            self.scroll_item_into_view(val);
        }
    }

    /// Confirm the currently highlighted item (select or toggle).
    pub fn confirm_highlighted(&mut self) {
        let highlighted = self.highlighted.read().clone();
        if let Some(val) = highlighted {
            // Check disabled
            let is_disabled = self
                .items
                .read()
                .iter()
                .any(|e| e.value == val && e.disabled);
            if is_disabled {
                return;
            }
            if self.multiple {
                self.toggle_value(&val);
            } else {
                self.select_single(&val);
            }
        }
    }

    // ── Registration ─────────────────────────────────────────

    /// Register an item. Called on mount.
    pub fn register_item(&mut self, entry: ItemEntry) {
        let mut items = self.items.write();
        if !items.iter().any(|e| e.value == entry.value) {
            items.push(entry);
        }
    }

    /// Deregister an item. Called on unmount.
    pub fn deregister_item(&mut self, value: &str) {
        let mut items = self.items.write();
        items.retain(|e| e.value != value);
    }

    /// Register a group. Called on mount.
    pub fn register_group(&mut self, entry: GroupEntry) {
        let mut groups = self.groups.write();
        if !groups.iter().any(|g| g.id == entry.id) {
            groups.push(entry);
        }
    }

    /// Deregister a group. Called on unmount.
    pub fn deregister_group(&mut self, id: &str) {
        let mut groups = self.groups.write();
        groups.retain(|g| g.id != id);
    }

    /// Mark that an `Input` child has been mounted (switches to combobox mode).
    pub fn mark_has_input(&mut self) {
        self.has_input.set(true);
    }

    /// Check if this select has a search input (combobox variant).
    pub fn has_search_input(&self) -> bool {
        (self.has_input)()
    }

    // ── ID generation ────────────────────────────────────────

    /// ID for the trigger button element.
    pub fn trigger_id(&self) -> String {
        format!("nox-select-{}-trigger", self.instance_id)
    }

    /// ID for the listbox popup element.
    pub fn listbox_id(&self) -> String {
        format!("nox-select-{}-listbox", self.instance_id)
    }

    /// ID for a specific option element.
    pub fn item_id(&self, value: &str) -> String {
        format!("nox-select-{}-item-{}", self.instance_id, value)
    }

    /// ID for the search input element.
    pub fn input_id(&self) -> String {
        format!("nox-select-{}-input", self.instance_id)
    }

    /// ID for a group label element.
    pub fn group_label_id(&self, group_id: &str) -> String {
        format!("nox-select-{}-group-{}", self.instance_id, group_id)
    }

    /// The `aria-activedescendant` value (highlighted item ID, or empty).
    pub fn active_descendant(&self) -> String {
        match self.highlighted.read().as_ref() {
            Some(val) => self.item_id(val),
            None => String::new(),
        }
    }

    // ── Public accessors for cross-crate use ────────────────

    /// Current autocomplete mode.
    pub fn autocomplete(&self) -> AutoComplete {
        self.autocomplete
    }

    /// Whether dropdown opens on focus.
    pub fn open_on_focus(&self) -> bool {
        self.open_on_focus
    }

    /// Whether this is a multi-select.
    pub fn is_multiple(&self) -> bool {
        self.multiple
    }

    /// Whether an item is currently highlighted.
    pub fn has_highlighted(&self) -> bool {
        self.highlighted.read().is_some()
    }

    /// Get the currently highlighted value (if any).
    pub fn highlighted_value(&self) -> Option<String> {
        self.highlighted.read().clone()
    }

    /// Set the search query text.
    pub fn set_search_query(&mut self, query: String) {
        self.search_query.set(query);
    }

    // ── DOM helpers (WASM only) ──────────────────────────────

    /// Scroll the highlighted item into view.
    fn scroll_item_into_view(&self, value: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let id = self.item_id(value);
            spawn(async move {
                let js = format!(
                    "document.getElementById('{}')?.scrollIntoView({{block:'nearest'}})",
                    id.replace('\'', "\\'")
                );
                _ = document::eval(&js).await;
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            _ = value;
        }
    }

    /// Focus the combobox element (trigger or input).
    pub(crate) fn focus_combobox(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let id = if self.has_search_input() {
                self.input_id()
            } else {
                self.trigger_id()
            };
            spawn(async move {
                let js = format!(
                    "document.getElementById('{}')?.focus()",
                    id.replace('\'', "\\'")
                );
                _ = document::eval(&js).await;
            });
        }
    }
}

/// Initialise the [`SelectContext`] and provide it via Dioxus context.
///
/// Called inside [`super::select::Root`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn init_select_context(
    default_value: Option<String>,
    controlled_value: Option<Signal<String>>,
    on_value_change: Option<EventHandler<String>>,
    default_values: Option<Vec<String>>,
    controlled_values: Option<Signal<Vec<String>>>,
    on_values_change: Option<EventHandler<Vec<String>>>,
    multiple: bool,
    disabled: bool,
    default_open: bool,
    controlled_open: Option<Signal<bool>>,
    on_open_change: Option<EventHandler<bool>>,
    autocomplete: AutoComplete,
    open_on_focus: bool,
    custom_filter: Option<CustomFilter>,
) -> SelectContext {
    let instance_id = use_hook(next_instance_id);

    let value = use_signal(|| default_value.unwrap_or_default());
    let values = use_signal(|| default_values.unwrap_or_default());
    let open = use_signal(|| default_open);
    let search_query = use_signal(String::new);
    let highlighted = use_signal(|| None::<String>);
    let items: Signal<Vec<ItemEntry>> = use_signal(Vec::new);
    let groups: Signal<Vec<GroupEntry>> = use_signal(Vec::new);
    let has_input = use_signal(|| false);
    let custom_filter_sig = use_signal(|| custom_filter);

    // Persist the nucleo Matcher across renders — allocated once (same pattern as cmdk).
    let matcher = use_hook(|| Rc::new(RefCell::new(Matcher::new(Config::DEFAULT))));

    // Reactive memo for scored items (recalculates on items/query/filter change).
    let scored_items = use_memo(move || {
        let query = search_query.read().clone();
        let all_items = items.read();
        let cf = custom_filter_sig.read();
        let mut m = matcher.borrow_mut();
        filter::score_items(&all_items, &query, cf.as_ref(), &mut m)
    });

    let visible_values = use_memo(move || filter::visible_values(&scored_items.read()));

    let ctx = SelectContext {
        value,
        controlled_value,
        values,
        controlled_values,
        open,
        controlled_open,
        search_query,
        scored_items,
        visible_values,
        highlighted,
        items,
        groups,
        multiple,
        disabled,
        autocomplete,
        open_on_focus,
        custom_filter: custom_filter_sig,
        on_value_change,
        on_values_change,
        on_open_change,
        instance_id,
        has_input,
    };

    use_context_provider(|| ctx);

    ctx
}
