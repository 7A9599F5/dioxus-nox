use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;
use keyboard_types::{Key, Modifiers};
use nucleo_matcher::{Config, Matcher};

#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

use crate::helpers::scroll_item_into_view;
use crate::navigation::{
    find_next, find_next_by, find_next_group, find_prev, find_prev_by, find_prev_group,
};
use crate::scoring::score_items;
use crate::types::{
    ActionPanelState, ActionRegistration, CustomFilter, GroupRegistration, ItemRegistration,
    ModeRegistration, PageRegistration, ScoredItem, ScoringStrategy,
};

/// Lazily initialized page navigation state.
/// Created on first `register_page` / `push_page` call — simple palettes that
/// don't use pages never allocate these signals.
#[derive(Clone, Copy)]
pub struct PageFeature {
    pub pages: Signal<Vec<PageRegistration>>,
    pub page_stack: Signal<Vec<String>>,
    pub page_data: Signal<Option<Rc<dyn Any>>>,
}

/// Lazily initialized command mode state.
/// Created on first `register_mode` call.
#[derive(Clone, Copy)]
pub struct ModeFeature {
    pub modes: Signal<Vec<ModeRegistration>>,
}

/// Lazily initialized action panel state (P-039).
/// Created on first `open_action_panel` or `CommandAction` mount.
#[derive(Clone, Copy)]
pub struct ActionPanelFeature {
    pub panel: Signal<Option<ActionPanelState>>,
    pub items: Signal<Vec<ActionRegistration>>,
}

/// Central state for a command palette instance.
#[derive(Clone, Copy)]
pub struct CommandContext {
    pub search: Signal<String>,
    pub active_item: Signal<Option<String>>,
    pub is_open: Signal<bool>,
    pub is_loading: Signal<bool>,
    /// P-051: Items stored as `Rc<ItemRegistration>` — cloning the Vec clones only pointers.
    pub items: Signal<Vec<Rc<ItemRegistration>>>,
    /// P-050: O(1) lookup from item id → index in `items`.
    pub(crate) item_index: Signal<HashMap<String, usize>>,
    pub groups: Signal<Vec<GroupRegistration>>,
    pub scored_items: Memo<Vec<ScoredItem>>,
    pub filtered_count: Memo<usize>,
    /// P-052: Merged memo computing both the ordered Vec and HashSet in one pass.
    /// Exposed for testing and future direct use; currently accessed via derived memos.
    #[allow(dead_code)]
    pub(crate) visible_items: Memo<(Vec<String>, HashSet<String>)>,
    pub visible_item_ids: Memo<Vec<String>>,
    pub visible_item_set: Memo<HashSet<String>>,
    pub visible_group_ids: Memo<HashSet<String>>,
    pub status_message: Signal<String>,
    pub on_select: Signal<Option<EventHandler<String>>>,
    pub custom_filter: Signal<Option<CustomFilter>>,
    pub input_element: Signal<Option<Rc<MountedData>>>,
    /// Set by `CommandAnchor` on mount. Used by `CommandList` (floating mode) to
    /// measure the reference element's bounding rect for placement computation.
    /// Falls back to `input_element` when `None`.
    pub anchor_element: Signal<Option<Rc<MountedData>>>,
    /// Lazily initialized page navigation. `None` until first page registration or push.
    pub page_feature: Signal<Option<PageFeature>>,
    pub active_page: Memo<Option<String>>,
    pub scoring_strategy: Signal<Option<Rc<dyn ScoringStrategy>>>,
    /// Lazily initialized command modes. `None` until first mode registration.
    pub mode_feature: Signal<Option<ModeFeature>>,
    /// The currently active mode, derived from the search prefix.
    pub active_mode: Memo<Option<ModeRegistration>>,
    /// The search query with the mode prefix stripped.
    pub mode_query: Memo<String>,
    /// Accessible label for the command palette.
    pub label: Signal<Option<String>>,
    /// When `true`, pointer (mouse/touch) events on items do not update
    /// the active item. Keyboard navigation still works.
    pub disable_pointer_selection: Signal<bool>,
    pub vim_bindings: Signal<bool>,
    /// When `true` (default), navigation wraps around from last to first and vice versa.
    /// When `false`, navigation stops at list boundaries.
    pub loop_navigation: Signal<bool>,
    /// Number of items to skip per PageDown/PageUp key press. Defaults to 10.
    pub page_size: Signal<usize>,
    /// When `true` (default), applies fuzzy scoring to filter items.
    /// When `false`, all non-hidden mode-matching items are returned in registration order,
    /// ignoring the search query for filtering purposes.
    pub should_filter: Signal<bool>,
    /// Called when the resolved value of the active item changes.
    /// Fires with `Some(value)` when an item is active, not at all when active is `None`.
    pub on_value_change: Signal<Option<EventHandler<String>>>,
    /// P-004: Whether the background (non-palette siblings) is currently marked `inert`.
    /// Set to `true` when the palette opens, `false` when it closes.
    pub inert_background: Signal<bool>,
    /// P-023: Current screen reader announcement text. Updated by the palette on
    /// state transitions (item change, page navigation, empty state, etc.).
    pub announcer: Signal<String>,
    /// P-028: DOM `id` of the element that was focused before the palette opened.
    /// Used to restore focus when the palette closes. `None` on non-wasm or if
    /// the previously focused element had no `id` attribute.
    pub focused_before_id: Signal<Option<String>>,
    /// P-039: Lazily initialized action panel. `None` until first action registration or open.
    pub action_panel_feature: Signal<Option<ActionPanelFeature>>,
    pub(crate) instance_id: u32,
}

impl PartialEq for CommandContext {
    fn eq(&self, other: &Self) -> bool {
        self.instance_id == other.instance_id
    }
}

impl CommandContext {
    // ── Lazy feature initialization ─────────────────────────────────────

    /// Lazily initialize the page navigation feature, returning the inner struct.
    pub(crate) fn ensure_pages(&self) -> PageFeature {
        if let Some(feat) = *self.page_feature.peek() {
            return feat;
        }
        let feat = PageFeature {
            pages: Signal::new(Vec::new()),
            page_stack: Signal::new(Vec::new()),
            page_data: Signal::new(None),
        };
        let mut pf = self.page_feature;
        pf.set(Some(feat));
        feat
    }

    /// Lazily initialize the mode feature, returning the inner struct.
    pub(crate) fn ensure_modes(&self) -> ModeFeature {
        if let Some(feat) = *self.mode_feature.peek() {
            return feat;
        }
        let feat = ModeFeature {
            modes: Signal::new(Vec::new()),
        };
        let mut mf = self.mode_feature;
        mf.set(Some(feat));
        feat
    }

    /// Lazily initialize the action panel feature, returning the inner struct.
    pub(crate) fn ensure_action_panel(&self) -> ActionPanelFeature {
        if let Some(feat) = *self.action_panel_feature.peek() {
            return feat;
        }
        let feat = ActionPanelFeature {
            panel: Signal::new(None),
            items: Signal::new(Vec::new()),
        };
        let mut af = self.action_panel_feature;
        af.set(Some(feat));
        feat
    }

    // ── Feature read helpers ────────────────────────────────────────────

    /// Check if the action panel is currently open (non-reactive peek).
    pub(crate) fn peek_action_panel_open(&self) -> bool {
        let feat = self.action_panel_feature.peek();
        if let Some(ref af) = *feat {
            af.panel.peek().is_some()
        } else {
            false
        }
    }

    /// Check if there are registered action items (non-reactive peek).
    pub(crate) fn peek_has_action_items(&self) -> bool {
        let feat = self.action_panel_feature.peek();
        if let Some(ref af) = *feat {
            !af.items.peek().is_empty()
        } else {
            false
        }
    }

    /// Read the action panel state reactively.
    pub fn read_action_panel(&self) -> Option<ActionPanelState> {
        let feat = *self.action_panel_feature.read();
        feat.and_then(|af| af.panel.read().clone())
    }

    // ── Item registration ───────────────────────────────────────────────

    /// Register an item. Called by CommandItem on mount.
    pub fn register_item(&self, reg: ItemRegistration) {
        let id = reg.id.clone();
        let mut items = self.items;
        items.write().push(Rc::new(reg));
        let idx = items.read().len() - 1;
        let mut index = self.item_index;
        index.write().insert(id, idx);
    }

    /// Unregister an item by ID. Called by CommandItem on drop.
    pub fn unregister_item(&self, id: &str) {
        let mut items = self.items;
        items.write().retain(|item| item.id != id);
        // Rebuild index after retain
        let mut index = self.item_index;
        let mut map = index.write();
        map.clear();
        for (i, item) in items.read().iter().enumerate() {
            map.insert(item.id.clone(), i);
        }
    }

    /// Register a group.
    pub fn register_group(&self, reg: GroupRegistration) {
        let mut groups = self.groups;
        groups.write().push(reg);
    }

    /// Unregister a group by ID.
    pub fn unregister_group(&self, id: &str) {
        let mut groups = self.groups;
        groups.write().retain(|g| g.id != id);
    }

    /// Register a page. Called by CommandPage on mount.
    pub fn register_page(&self, reg: PageRegistration) {
        let pf = self.ensure_pages();
        let mut pages = pf.pages;
        pages.write().push(reg);
    }

    /// Unregister a page by ID. Called by CommandPage on drop.
    /// Also removes the page from the stack if present.
    pub fn unregister_page(&self, id: &str) {
        if let Some(pf) = *self.page_feature.peek() {
            let mut pages = pf.pages;
            pages.write().retain(|p| p.id != id);
            let mut stack = pf.page_stack;
            stack.write().retain(|s| s != id);
        }
    }

    /// Push a page onto the navigation stack and clear search.
    pub fn push_page(&self, page_id: &str) {
        let pf = self.ensure_pages();
        let mut stack = pf.page_stack;
        stack.write().push(page_id.to_string());
        let mut search = self.search;
        search.set(String::new());
    }

    /// Push a page with associated data.
    pub fn push_page_with_data(&self, page_id: &str, data: Rc<dyn Any>) {
        let pf = self.ensure_pages();
        let mut pd = pf.page_data;
        pd.set(Some(data));
        self.push_page(page_id);
    }

    /// Get the current page's data, downcast to the expected type.
    pub fn get_page_data<T: 'static>(&self) -> Option<Rc<T>> {
        let feat = self.page_feature.peek();
        let pf = (*feat).as_ref()?;
        let data = pf.page_data.read();
        data.as_ref().and_then(|d| d.clone().downcast::<T>().ok())
    }

    /// Pop the top page from the stack and clear search.
    /// Returns the popped page ID, or None if stack was empty.
    pub fn pop_page(&self) -> Option<String> {
        let pf = (*self.page_feature.peek())?;
        let mut stack = pf.page_stack;
        let popped = stack.write().pop();
        if popped.is_some() {
            let mut search = self.search;
            search.set(String::new());
            let mut pd = pf.page_data;
            pd.set(None);
        }
        popped
    }

    /// Clear the page stack (return to root) and clear search.
    pub fn clear_pages(&self) {
        if let Some(pf) = *self.page_feature.peek() {
            let mut stack = pf.page_stack;
            if !stack.read().is_empty() {
                stack.write().clear();
                let mut search = self.search;
                search.set(String::new());
                let mut pd = pf.page_data;
                pd.set(None);
            }
        }
    }

    /// Check if a page is the currently active page.
    pub fn is_page_active(&self, page_id: &str) -> bool {
        self.active_page.read().as_deref() == Some(page_id)
    }

    /// Check if an item is visible after filtering. O(1) via HashSet.
    pub fn is_item_visible(&self, id: &str) -> bool {
        self.visible_item_set.read().contains(id)
    }

    /// Check if a group has any visible items. O(1) via pre-computed HashSet.
    pub fn is_group_visible(&self, group_id: &str) -> bool {
        self.visible_group_ids.read().contains(group_id)
    }

    /// Get the active item's full registration data.
    /// Returns a cloned `Rc` — cheap pointer clone, no data copy.
    pub fn active_item_data(&self) -> Option<Rc<ItemRegistration>> {
        let active = self.active_item.read();
        active.as_ref().and_then(|id| {
            let index = self.item_index.read();
            index.get(id.as_str()).map(|&i| {
                let items = self.items.read();
                items[i].clone()
            })
        })
    }

    /// Register a mode.
    pub fn register_mode(&self, reg: ModeRegistration) {
        let mf = self.ensure_modes();
        let mut modes = mf.modes;
        modes.write().push(reg);
    }

    /// Unregister a mode by ID.
    pub fn unregister_mode(&self, id: &str) {
        if let Some(mf) = *self.mode_feature.peek() {
            let mut modes = mf.modes;
            modes.write().retain(|m| m.id != id);
        }
    }

    /// Navigate to the next visible item.
    pub fn select_next(&self) {
        let visible = self.visible_item_ids.read();
        if visible.is_empty() {
            return;
        }
        let current = self.active_item.read();
        let next = match &*current {
            None => visible.first().cloned(),
            Some(current_id) => {
                let pos = visible.iter().position(|id| id == current_id);
                match pos {
                    Some(i) => {
                        let items = self.items.read();
                        let loop_nav = (self.loop_navigation)();
                        find_next(&visible, i, &items, loop_nav).map(|idx| visible[idx].clone())
                    }
                    None => visible.first().cloned(),
                }
            }
        };
        drop(current);
        if let Some(ref id) = next {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active = self.active_item;
        active.set(next);
    }

    /// Navigate to the previous visible item.
    pub fn select_prev(&self) {
        let visible = self.visible_item_ids.read();
        if visible.is_empty() {
            return;
        }
        let current = self.active_item.read();
        let prev = match &*current {
            None => visible.last().cloned(),
            Some(current_id) => {
                let pos = visible.iter().position(|id| id == current_id);
                match pos {
                    Some(i) => {
                        let items = self.items.read();
                        let loop_nav = (self.loop_navigation)();
                        find_prev(&visible, i, &items, loop_nav).map(|idx| visible[idx].clone())
                    }
                    None => visible.last().cloned(),
                }
            }
        };
        drop(current);
        if let Some(ref id) = prev {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active = self.active_item;
        active.set(prev);
    }

    /// P-023: Announce a message to screen readers via the hidden `aria-live` region.
    ///
    /// Replaces the current announcement text. The `aria-live="polite"` region in
    /// `CommandRoot`'s RSX renders this signal value.
    pub fn announce(&mut self, msg: impl Into<String>) {
        self.announcer.set(msg.into());
    }

    /// Navigate forward by `steps` non-disabled items (PageDown behaviour).
    pub fn select_next_by(&self, steps: usize) {
        let visible = self.visible_item_ids.read();
        if visible.is_empty() {
            return;
        }
        let current = self.active_item.read();
        let next = match &*current {
            None => visible.first().cloned(),
            Some(current_id) => {
                let pos = visible.iter().position(|id| id == current_id);
                match pos {
                    Some(i) => {
                        let items = self.items.read();
                        let loop_nav = (self.loop_navigation)();
                        find_next_by(&visible, i, &items, steps, loop_nav)
                            .map(|idx| visible[idx].clone())
                    }
                    None => visible.first().cloned(),
                }
            }
        };
        drop(current);
        if let Some(ref id) = next {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active = self.active_item;
        active.set(next);
    }

    /// Navigate backward by `steps` non-disabled items (PageUp behaviour).
    pub fn select_prev_by(&self, steps: usize) {
        let visible = self.visible_item_ids.read();
        if visible.is_empty() {
            return;
        }
        let current = self.active_item.read();
        let prev = match &*current {
            None => visible.last().cloned(),
            Some(current_id) => {
                let pos = visible.iter().position(|id| id == current_id);
                match pos {
                    Some(i) => {
                        let items = self.items.read();
                        let loop_nav = (self.loop_navigation)();
                        find_prev_by(&visible, i, &items, steps, loop_nav)
                            .map(|idx| visible[idx].clone())
                    }
                    None => visible.last().cloned(),
                }
            }
        };
        drop(current);
        if let Some(ref id) = prev {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active = self.active_item;
        active.set(prev);
    }

    /// P-021: Navigate to the first item of the next visible group.
    ///
    /// Key binding: `Alt+Shift+ArrowDown` in `CommandInput`.
    /// (`Alt+Arrow` is reserved for history navigation.)
    pub fn select_next_group(&self) {
        let items = self.items.read();
        let groups = self.groups.read();
        let visible_set = self.visible_item_set.read();
        let active = self.active_item.read();
        let active_id = active.as_deref();
        let loop_nav = (self.loop_navigation)();

        let result = find_next_group(&items, &groups, active_id, &visible_set, loop_nav);
        drop(active);
        drop(visible_set);
        drop(groups);
        drop(items);

        if let Some(ref id) = result {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active_item = self.active_item;
        active_item.set(result);
    }

    /// P-021: Navigate to the last item of the previous visible group.
    ///
    /// Key binding: `Alt+Shift+ArrowUp` in `CommandInput`.
    /// (`Alt+Arrow` is reserved for history navigation.)
    pub fn select_prev_group(&self) {
        let items = self.items.read();
        let groups = self.groups.read();
        let visible_set = self.visible_item_set.read();
        let active = self.active_item.read();
        let active_id = active.as_deref();
        let loop_nav = (self.loop_navigation)();

        let result = find_prev_group(&items, &groups, active_id, &visible_set, loop_nav);
        drop(active);
        drop(visible_set);
        drop(groups);
        drop(items);

        if let Some(ref id) = result {
            scroll_item_into_view(self.instance_id, id);
        }
        let mut active_item = self.active_item;
        active_item.set(result);
    }

    /// Try to find and execute an item whose shortcut matches the given key event.
    ///
    /// Returns `true` if a matching, non-disabled item was found and executed.
    /// Shortcuts match even if the item is currently filtered out.
    pub(crate) fn try_execute_shortcut(&self, key: &Key, modifiers: Modifiers) -> bool {
        let items = self.items.read();
        let matched = items.iter().find(|item| {
            !item.disabled
                && item
                    .shortcut
                    .as_ref()
                    .is_some_and(|hk| hk.matches(key, modifiers))
        });
        if let Some(item) = matched {
            let item_id = item.id.clone();
            let resolved = item.value.clone().unwrap_or_else(|| item.id.clone());
            let item_handler = item.on_select.clone();
            drop(items);

            // Update active item
            let mut active = self.active_item;
            active.set(Some(item_id));

            // Fire on_value_change directly — the deferred effect won't run
            // in time because on_select typically closes the palette synchronously.
            let value_handler = self.on_value_change.peek();
            if let Some(ref h) = *value_handler {
                h.call(resolved.clone());
            }
            drop(value_handler);

            if let Some(cb) = item_handler {
                cb.0.call(resolved);
            } else {
                let handler = self.on_select.read();
                if let Some(ref h) = *handler {
                    h.call(resolved);
                }
            }
            true
        } else {
            false
        }
    }

    /// Confirm selection of the currently active item.
    ///
    /// Item-level `on_select` takes precedence over the root `on_select` handler.
    pub fn confirm_selection(&self) {
        let active = self.active_item.read();
        if let Some(id) = active.clone() {
            let items = self.items.read();
            let item = items.iter().find(|it| it.id == id);
            if item.is_some_and(|it| it.disabled) {
                return;
            }
            let resolved = item
                .and_then(|it| it.value.clone())
                .unwrap_or_else(|| id.clone());
            let item_handler = item.and_then(|it| it.on_select.clone());
            drop(items);
            drop(active);
            // Item-level on_select takes precedence over root on_select
            if let Some(cb) = item_handler {
                cb.0.call(resolved);
            } else {
                let handler = self.on_select.read();
                if let Some(ref h) = *handler {
                    h.call(resolved);
                }
            }
        }
    }

    // ── P-039: Action panel methods ──────────────────────────────────────

    /// Open the action panel for the item with the given ID.
    pub fn open_action_panel(&mut self, item_id: String) {
        let mut af = self.ensure_action_panel();
        af.panel.set(Some(ActionPanelState {
            item_id,
            active_idx: 0,
        }));
    }

    /// Close the action panel.
    pub fn close_action_panel(&mut self) {
        if let Some(mut af) = *self.action_panel_feature.peek() {
            af.panel.set(None);
        }
    }

    /// Move selection to the next action in the panel.
    pub fn select_next_action(&mut self) {
        let Some(mut af) = *self.action_panel_feature.peek() else {
            return;
        };
        let count = af.items.read().len();
        if count == 0 {
            return;
        }
        let mut panel = af.panel.write();
        if let Some(ref mut state) = *panel {
            state.active_idx = (state.active_idx + 1) % count;
        }
    }

    /// Move selection to the previous action in the panel.
    pub fn select_prev_action(&mut self) {
        let Some(mut af) = *self.action_panel_feature.peek() else {
            return;
        };
        let count = af.items.read().len();
        if count == 0 {
            return;
        }
        let mut panel = af.panel.write();
        if let Some(ref mut state) = *panel {
            state.active_idx = if state.active_idx == 0 {
                count - 1
            } else {
                state.active_idx - 1
            };
        }
    }

    /// Execute the currently active action, or fall back to the default on_select.
    pub fn confirm_action(&mut self) {
        let Some(af) = *self.action_panel_feature.peek() else {
            return;
        };
        let panel_state = af.panel.read().clone();
        let Some(state) = panel_state else { return };
        let items = af.items.read();
        let handler = items.get(state.active_idx).and_then(|reg| reg.on_action);
        let has_action = items.get(state.active_idx).is_some();
        drop(items);
        if let Some(h) = handler {
            h.call(state.item_id.clone());
        } else if !has_action {
            self.confirm_selection();
        }
        self.close_action_panel();
    }
}

/// Access the [`CommandContext`] from within a `CommandRoot` component tree.
///
/// This is the primary hook for building custom components that need to read
/// or mutate palette state (search query, active item, visibility, etc.).
///
/// Must be called inside a component tree where [`CommandRoot`](crate::CommandRoot) is an ancestor.
/// Panics if called outside a `CommandRoot` tree.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_nox_cmdk::use_command_context;
///
/// #[component]
/// fn MyCustomItem(id: String, label: String) -> Element {
///     let ctx = use_command_context();
///     let is_active = ctx.active_item.read().as_deref() == Some(&id);
///     rsx! { div { class: if is_active { "active" } else { "" }, "{label}" } }
/// }
/// ```
pub fn use_command_context() -> CommandContext {
    use_context::<CommandContext>()
}

/// Initialize and provide the command palette context.
/// Must be called in CommandRoot. Returns the context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn init_command_context(
    on_select: Option<EventHandler<String>>,
    custom_filter: Option<CustomFilter>,
    initial_search: Option<String>,
    on_search_change: Option<EventHandler<String>>,
    on_active_change: Option<EventHandler<Option<String>>>,
    scoring_strategy: Option<Rc<dyn ScoringStrategy>>,
    label: Option<String>,
    disable_pointer_selection: bool,
    vim_bindings: bool,
    loop_navigation: bool,
    default_value: Option<String>,
    should_filter: bool,
    search_debounce_ms: u32,
    value: Option<Signal<Option<String>>>,
    on_value_change: Option<EventHandler<String>>,
    page_size: usize,
) -> CommandContext {
    let instance_id = use_hook(crate::helpers::next_instance_id);
    let default_value_ref = use_hook(|| default_value);

    let search = use_signal(|| initial_search.unwrap_or_default());
    let active_item: Signal<Option<String>> = use_signal(|| None);
    let is_open = use_signal(|| false);
    let is_loading = use_signal(|| false);
    // P-051: Store items as Rc<ItemRegistration> for cheap Vec clones
    let items: Signal<Vec<Rc<ItemRegistration>>> = use_signal(Vec::new);
    // P-050: O(1) id → index lookup
    let item_index: Signal<HashMap<String, usize>> = use_signal(HashMap::new);
    let groups: Signal<Vec<GroupRegistration>> = use_signal(Vec::new);
    let status_message = use_signal(String::new);
    let on_select_sig = use_signal(|| on_select);
    let custom_filter_sig: Signal<Option<CustomFilter>> = use_signal(|| custom_filter);
    let input_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let anchor_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    // Lazy feature wrappers — None until the feature is first used.
    let page_feature: Signal<Option<PageFeature>> = use_signal(|| None);
    let mode_feature: Signal<Option<ModeFeature>> = use_signal(|| None);
    let action_panel_feature: Signal<Option<ActionPanelFeature>> = use_signal(|| None);
    let scoring_strategy_sig: Signal<Option<Rc<dyn ScoringStrategy>>> =
        use_signal(|| scoring_strategy);
    let label_sig: Signal<Option<String>> = use_signal(|| label);
    let disable_pointer_selection_sig = use_signal(|| disable_pointer_selection);
    let vim_bindings_sig = use_signal(|| vim_bindings);
    let loop_navigation_sig = use_signal(|| loop_navigation);
    let page_size_sig = use_signal(|| page_size);
    let should_filter_sig = use_signal(|| should_filter);
    let mut should_filter_sig_mut = should_filter_sig;
    should_filter_sig_mut.set(should_filter);

    // P-016: debounced query for search scoring
    let debounced_query: Signal<String> = use_signal(String::new);
    let debounce_task: Rc<RefCell<Option<dioxus_core::Task>>> =
        use_hook(|| Rc::new(RefCell::new(None)));

    // Active page = top of stack (None = root). Reads page_feature wrapper reactively.
    let active_page = use_memo(move || {
        let feat = page_feature.read();
        feat.as_ref()
            .and_then(|pf| pf.page_stack.read().last().cloned())
    });

    // Derive active mode from search prefix. Reads mode_feature wrapper reactively.
    let active_mode = use_memo(move || {
        let query = search.read();
        if query.is_empty() {
            return None;
        }
        let feat = mode_feature.read();
        let mf = feat.as_ref()?;
        let modes_list = mf.modes.read();
        modes_list
            .iter()
            .find(|m| query.starts_with(&m.prefix))
            .cloned()
    });

    // Strip prefix from query for scoring
    let mode_query = use_memo(move || {
        let query = search.read().clone();
        let mode = active_mode.read();
        match &*mode {
            Some(m) => query.strip_prefix(&m.prefix).unwrap_or(&query).to_string(),
            None => query,
        }
    });

    // Update on_select if it changes across renders
    let mut on_select_sig_mut = on_select_sig;
    on_select_sig_mut.set(on_select);

    // on_search_change callback
    let on_search_change_sig: Signal<Option<EventHandler<String>>> =
        use_signal(|| on_search_change);
    let mut on_search_change_sig_mut = on_search_change_sig;
    on_search_change_sig_mut.set(on_search_change);

    // Fire on_search_change when search query changes
    use_effect(move || {
        let query = search.read().clone();
        let handler = on_search_change_sig.peek();
        if let Some(ref h) = *handler {
            h.call(query);
        }
    });

    // P-016: Update debounced_query (feeds scored_items when search_debounce_ms > 0).
    // Uses Dioxus spawn + Task::cancel() to implement debouncing without raw JS interop.
    use_effect(move || {
        let query = mode_query.read().clone();
        // Cancel any in-flight debounce task
        if let Some(old_task) = debounce_task.borrow_mut().take() {
            old_task.cancel();
        }
        if search_debounce_ms > 0 {
            let new_task = spawn(async move {
                #[cfg(target_arch = "wasm32")]
                TimeoutFuture::new(search_debounce_ms).await;
                // Non-wasm: fires immediately (no browser timer available in the
                // desktop/mobile async context; local filtering is instant anyway).
                let mut dq = debounced_query;
                dq.set(query);
            });
            *debounce_task.borrow_mut() = Some(new_task);
        } else {
            let mut dq = debounced_query;
            dq.set(query);
        }
    });

    // on_active_change callback
    let on_active_change_sig: Signal<Option<EventHandler<Option<String>>>> =
        use_signal(|| on_active_change);
    let mut on_active_change_sig_mut = on_active_change_sig;
    on_active_change_sig_mut.set(on_active_change);

    // P-012: on_value_change callback
    let on_value_change_sig: Signal<Option<EventHandler<String>>> = use_signal(|| on_value_change);
    let mut on_value_change_sig_mut = on_value_change_sig;
    on_value_change_sig_mut.set(on_value_change);

    // P-004: Background inert signal — toggled by CommandDialog/CommandSheet open state.
    let inert_background: Signal<bool> = use_signal(|| false);

    // P-023: Screen reader announcer — empty by default, updated on state transitions.
    let announcer: Signal<String> = use_signal(String::new);

    // P-028: DOM id of element focused before the palette opened (for focus restore).
    let focused_before_id: Signal<Option<String>> = use_signal(|| None);

    // Fire on_active_change (and on_value_change) when the highlighted item changes
    use_effect(move || {
        let active_id = active_item.read().clone();
        let active_handler = on_active_change_sig.peek();
        let value_handler = on_value_change_sig.peek();
        if active_handler.is_none() && value_handler.is_none() {
            return;
        }
        let resolved = active_id.map(|id| {
            let items_list = items.peek();
            items_list
                .iter()
                .find(|i| i.id == id)
                .and_then(|it| it.value.clone())
                .unwrap_or(id)
        });
        if let Some(ref h) = *active_handler {
            h.call(resolved.clone());
        }
        if let Some(ref resolved_val) = resolved
            && let Some(ref h) = *value_handler
        {
            h.call(resolved_val.clone());
        }
    });

    // Persist the nucleo Matcher (~135KB) across renders — allocated once.
    let matcher = use_hook(|| Rc::new(RefCell::new(Matcher::new(Config::DEFAULT))));

    // Nucleo filter pipeline: scored_items memo (delegates to pure `score_items`)
    let scored_items = use_memo(move || {
        let active = active_mode.read();
        let mode_id = active.as_ref().map(|m| m.id.clone());
        // P-051: clone is cheap — only Rc pointer clones, not data copies
        let all_items = items.read().clone();

        // Mode-match closure reused in both filter paths
        let mode_matches = |item: &ItemRegistration| -> bool {
            match (&mode_id, &item.mode_id) {
                (Some(active_mode_id), Some(item_mode_id)) => active_mode_id == item_mode_id,
                (Some(_), None) => true, // Items with no mode appear in all modes
                (None, Some(_)) => false, // Mode-specific items hidden at root
                (None, None) => true,    // No mode active, no mode on item
            }
        };

        // P-002: When should_filter is false, bypass scoring entirely
        if !(*should_filter_sig.read()) {
            return all_items
                .into_iter()
                .filter(|i| !i.hidden && mode_matches(i))
                .map(|i| ScoredItem {
                    id: i.id.clone(),
                    score: None,
                    match_indices: None,
                })
                .collect();
        }

        // P-016: Use debounced query when configured, else mode_query directly
        let query = if search_debounce_ms > 0 {
            debounced_query.read().clone()
        } else {
            mode_query.read().clone()
        };

        // Filter items by mode before scoring (Rc clones only)
        let mode_items: Vec<Rc<ItemRegistration>> = all_items
            .into_iter()
            .filter(|item| mode_matches(item))
            .collect();

        let filter_fn = custom_filter_sig.read().clone();
        let strategy = scoring_strategy_sig.read();
        let strategy_ref = strategy.as_ref().map(|s| s.as_ref());
        let mut m = matcher.borrow_mut();
        score_items(&mode_items, &query, filter_fn, strategy_ref, &mut m)
    });

    // P-052: Single merged memo — compute both Vec and HashSet in one pass.
    let visible_items: Memo<(Vec<String>, HashSet<String>)> = use_memo(move || {
        let current_page = active_page.read().clone();
        let all_items = items.read();
        let mut vec: Vec<String> = Vec::new();
        let mut set: HashSet<String> = HashSet::new();
        for si in scored_items.read().iter() {
            let item_page = all_items
                .iter()
                .find(|i| i.id == si.id)
                .and_then(|i| i.page_id.clone());
            if item_page == current_page {
                set.insert(si.id.clone());
                vec.push(si.id.clone());
            }
        }
        (vec, set)
    });

    // Backward-compat derived memos that delegate to the merged memo.
    let visible_item_ids = use_memo(move || visible_items.read().0.clone());
    let visible_item_set = use_memo(move || visible_items.read().1.clone());

    let visible_group_ids = use_memo(move || {
        // Clone the set to avoid holding the memo read guard across items.read()
        let vis = visible_items.read().1.clone();
        let all = items.read();
        let groups_list = groups.read();
        // P-017: force_mount groups are always included
        let mut result: HashSet<String> = groups_list
            .iter()
            .filter_map(|g| {
                if g.force_mount {
                    Some(g.id.clone())
                } else {
                    None
                }
            })
            .collect();
        // Add groups that have at least one visible item
        for item in all.iter() {
            if let Some(ref gid) = item.group_id
                && vis.contains(&item.id)
            {
                result.insert(gid.clone());
            }
        }
        result
    });

    let filtered_count = use_memo(move || visible_items.read().0.len());

    // Auto-select first visible item when filter changes
    use_effect(move || {
        let visible = visible_item_ids.read();
        let current_active = active_item.peek().clone();
        // If the currently active item is no longer visible, reset to first
        let should_reset = match &current_active {
            None => !visible.is_empty(),
            Some(id) => !visible.iter().any(|v| v == id),
        };
        if should_reset {
            let mut active = active_item;
            // Try matching default_value against visible items' value or id
            let default_match = default_value_ref.as_ref().and_then(|dv| {
                let all_items = items.peek();
                visible
                    .iter()
                    .find(|vid| {
                        all_items.iter().any(|item| {
                            item.id == **vid
                                && (item.value.as_deref() == Some(dv.as_str()) || item.id == *dv)
                        })
                    })
                    .cloned()
            });
            active.set(default_match.or_else(|| visible.first().cloned()));
        }
    });

    // P-012: Sync controlled value → active_item.
    // When the `value` signal changes, find the matching item and activate it.
    use_effect(move || {
        let Some(v_sig) = value else { return };
        let target = (v_sig)();
        let items_list = items.peek();
        let new_active = target.and_then(|tv| {
            items_list
                .iter()
                .find(|i| i.value.as_deref() == Some(tv.as_str()) || i.id == tv)
                .map(|i| i.id.clone())
        });
        let mut a = active_item;
        a.set(new_active);
    });

    // Update status message for screen readers
    use_effect(move || {
        let count = filtered_count();
        let query = search.read().clone();
        let mut msg = status_message;
        if query.is_empty() {
            msg.set(String::new());
        } else {
            msg.set(format!(
                "{count} result{}",
                if count == 1 { "" } else { "s" }
            ));
        }
    });

    let ctx = CommandContext {
        search,
        active_item,
        is_open,
        is_loading,
        items,
        item_index,
        groups,
        scored_items,
        filtered_count,
        visible_items,
        visible_item_ids,
        visible_item_set,
        visible_group_ids,
        status_message,
        on_select: on_select_sig,
        custom_filter: custom_filter_sig,
        input_element,
        anchor_element,
        page_feature,
        active_page,
        scoring_strategy: scoring_strategy_sig,
        mode_feature,
        active_mode,
        mode_query,
        label: label_sig,
        disable_pointer_selection: disable_pointer_selection_sig,
        vim_bindings: vim_bindings_sig,
        loop_navigation: loop_navigation_sig,
        page_size: page_size_sig,
        should_filter: should_filter_sig,
        on_value_change: on_value_change_sig,
        inert_background,
        announcer,
        focused_before_id,
        action_panel_feature,
        instance_id,
    };

    use_context_provider(|| ctx);
    ctx
}
