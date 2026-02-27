use std::cmp::Ordering;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use dioxus::prelude::*;

use crate::tag::TagLike;

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A group of suggestions sharing a label.
#[derive(Clone, PartialEq, Debug)]
pub struct SuggestionGroup<T: TagLike> {
    /// Group name. Empty string for ungrouped items (tags where `group()` returns `None`).
    pub label: String,
    /// Items visible in this group (may be truncated by `max_items_per_group`).
    pub items: Vec<T>,
    /// Total matching items before `max_items_per_group` truncation.
    pub total_count: usize,
}

/// Configuration for the grouped tag input hook.
///
/// Uses `fn` pointers (not closures) because they are `Copy` and trivially
/// captured by `use_memo`.
pub struct TagInputGroupConfig<T: TagLike> {
    pub available_tags: Vec<T>,
    pub initial_selected: Vec<T>,
    /// Custom filter: receives `(tag, lowercase_query)`. Default: substring match on `name()`.
    pub filter: Option<fn(&T, &str) -> bool>,
    /// Sort items within each group. Default: no sort (insertion order).
    pub sort_items: Option<fn(&T, &T) -> Ordering>,
    /// Sort group headers. Default: no sort (first-seen order).
    pub sort_groups: Option<fn(&str, &str) -> Ordering>,
    /// Max items shown per group. `None` = unlimited. `total_count` still reflects all matches.
    pub max_items_per_group: Option<usize>,
    /// Parent-owned signal for selected tags (controlled mode). `initial_selected` ignored when set.
    pub value: Option<Signal<Vec<T>>>,
    /// Parent-owned signal for search query (controlled mode).
    pub query: Option<Signal<String>>,
    /// Parent-owned signal for dropdown open state (controlled mode).
    pub open: Option<Signal<bool>>,
}

/// Configuration for the simple tag input hook.
pub struct TagInputConfig<T: TagLike> {
    pub available_tags: Vec<T>,
    pub initial_selected: Vec<T>,
    /// Parent-owned signal for selected tags (controlled mode). `initial_selected` ignored when set.
    pub value: Option<Signal<Vec<T>>>,
    /// Parent-owned signal for search query (controlled mode).
    pub query: Option<Signal<String>>,
    /// Parent-owned signal for dropdown open state (controlled mode).
    pub open: Option<Signal<bool>>,
}

impl<T: TagLike> TagInputConfig<T> {
    pub fn new(available_tags: Vec<T>, initial_selected: Vec<T>) -> Self {
        Self {
            available_tags,
            initial_selected,
            value: None,
            query: None,
            open: None,
        }
    }
}

/// Find byte-offset ranges in `text` that match `query` (case-insensitive substring).
///
/// Returns `Vec<(start, end)>` pairs suitable for slicing `text` and wrapping
/// matched portions in highlight markup.
pub fn find_match_ranges(text: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut ranges = Vec::new();
    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&query_lower) {
        let abs_start = start + pos;
        let abs_end = abs_start + query.len();
        ranges.push((abs_start, abs_end));
        start = abs_end;
    }
    ranges
}

/// Headless state for the tag input component.
///
/// All fields are `Signal` or `Memo`, which are `Copy` in Dioxus 0.7,
/// so `TagInputState` manually implements `Clone`, `Copy`, and `PartialEq`
/// without requiring `T: Copy` or `T: PartialEq`.
#[allow(clippy::type_complexity)]
pub struct TagInputState<T: TagLike> {
    /// The current search/filter query text.
    pub search_query: Signal<String>,
    /// The tags currently selected by the user.
    pub selected_tags: Signal<Vec<T>>,
    /// All tags available for selection.
    pub available_tags: Signal<Vec<T>>,
    /// Whether the suggestion dropdown is open.
    pub is_dropdown_open: Signal<bool>,
    /// Index of the currently highlighted suggestion, or `None` when no item is selected.
    pub highlighted_index: Signal<Option<usize>>,
    /// Suggestions filtered by the current search query, excluding already-selected tags.
    pub filtered_suggestions: Memo<Vec<T>>,
    /// Index of the keyboard-selected pill, or `None` when the cursor is in the text input.
    pub active_pill: Signal<Option<usize>>,
    /// Index of the pill whose popover is open, or `None` when no popover is shown.
    pub popover_pill: Signal<Option<usize>>,
    /// Suggestions organized into labelled groups.
    ///
    /// Populated automatically by both `use_tag_input` (single catch-all group)
    /// and `use_tag_input_grouped` (full config). The flat `filtered_suggestions`
    /// memo is always kept in sync — its order matches the concatenation of all
    /// group items.
    pub grouped_suggestions: Memo<Vec<SuggestionGroup<T>>>,
    /// Optional callback for creating new tags on Enter when no suggestion is highlighted.
    ///
    /// When `Some`, pressing Enter with a non-empty query and no highlighted suggestion
    /// will call this callback with the query text. Return `Some(tag)` to accept the
    /// new tag, or `None` to reject it.
    ///
    /// Default: `None` (feature off — Enter just opens the dropdown).
    pub on_create: Signal<Option<Callback<String, Option<T>>>>,
    /// Optional callback fired when a tag is about to be removed.
    ///
    /// Called with the tag being removed, before it is actually removed from `selected_tags`.
    /// Fires from `remove_tag()` and `remove_last_tag()`.
    ///
    /// Default: `None`
    pub on_remove: Signal<Option<Callback<T>>>,
    /// Optional callback fired when a tag is added.
    ///
    /// Called with the newly added tag after it has been pushed to `selected_tags`.
    /// Fires from `add_tag()` (and by extension `create_tag()`).
    ///
    /// Default: `None`
    pub on_add: Signal<Option<Callback<T>>>,
    /// Whether the tag input is disabled (no interaction allowed).
    ///
    /// When `true`, `handle_keydown`, `set_query`, `add_tag`, `remove_tag`, and
    /// `handle_click` become no-ops. Consumers should also apply `disabled` /
    /// `aria-disabled="true"` attributes and visual styling based on this signal.
    pub is_disabled: Signal<bool>,
    /// Screen-reader status message for `aria-live` announcements.
    ///
    /// Updated automatically when tags are added/removed and when the suggestion
    /// count changes. Consumers render this inside a `<div role="status" aria-live="polite">`
    /// element to provide announcements to assistive technology.
    ///
    /// Example messages:
    /// - `"Apple added. 3 tags selected."`
    /// - `"Cherry removed. 2 tags selected."`
    /// - `"5 suggestions available."`
    /// - `"No suggestions found."`
    /// - `"Maximum of 5 tags reached."`
    pub status_message: Signal<String>,
    /// Optional callback fired when text is pasted into the input.
    ///
    /// Called with the raw clipboard text. Return a `Vec<T>` of tags to add.
    /// If the callback returns an empty vec, no tags are added.
    ///
    /// Takes priority over `paste_delimiters` when set.
    ///
    /// Default: `None`
    pub on_paste: Signal<Option<Callback<String, Vec<T>>>>,
    /// Delimiter characters for splitting pasted text into tags.
    ///
    /// When set and `on_paste` is `None`, pasted text is split by these delimiters.
    /// Each non-empty token is passed to `on_create` (if set) to create a tag.
    /// Common delimiters: `[',', '\n', '\t']`.
    ///
    /// Default: `None` (paste behaves normally — text enters the input field)
    pub paste_delimiters: Signal<Option<Vec<char>>>,
    // ── Phase 2: Editing & Reorder ──────────────────────────────────────
    /// Index of the pill currently being edited inline, or `None`.
    ///
    /// When `Some(idx)`, the consumer should render an `<input>` instead of a
    /// `<span>` for that pill. Use `start_editing(idx)` to enter edit mode,
    /// `commit_edit(new_name)` to apply changes, and `cancel_edit()` to discard.
    pub editing_pill: Signal<Option<usize>>,
    /// Optional callback for applying an inline edit to a tag.
    ///
    /// Called with `(current_tag, new_name_string)`. Return `Some(updated_tag)` to
    /// accept the edit, or `None` to reject it. The consumer is responsible for
    /// constructing the updated tag (the library doesn't know your tag's internals).
    ///
    /// Default: `None` (editing disabled)
    pub on_edit: Signal<Option<Callback<(T, String), Option<T>>>>,
    /// Optional callback fired after a tag is reordered via `move_tag`.
    ///
    /// Called with `(from_index, to_index)` after the move completes.
    ///
    /// Default: `None`
    pub on_reorder: Signal<Option<Callback<(usize, usize)>>>,
    // ── Phase 3: Validation & Limits ────────────────────────────────────
    /// Delimiter characters that commit the current query as a tag.
    ///
    /// When set, typing any of these characters commits the current query:
    /// if a suggestion is highlighted, it's selected; otherwise `on_create`
    /// is called (if set). Common delimiters: `[',', ';', '\t']`.
    /// `Enter` is always a commit key and doesn't need to be in this list.
    ///
    /// Default: `None` (only Enter commits)
    pub delimiters: Signal<Option<Vec<char>>>,
    /// Maximum number of tags that can be selected.
    ///
    /// When set and the limit is reached, `add_tag` becomes a no-op and
    /// `status_message` announces "Maximum of N tags reached."
    ///
    /// Default: `None` (unlimited)
    pub max_tags: Signal<Option<usize>>,
    /// Whether the maximum tag limit has been reached.
    ///
    /// Reactive memo derived from `max_tags` and `selected_tags.len()`.
    /// Consumers can use this to disable the input or hide suggestions.
    pub is_at_limit: Memo<bool>,
    /// Optional validation callback called before a tag is committed.
    ///
    /// Called with the tag about to be added. Return `Ok(())` to accept,
    /// or `Err("message")` to reject. The rejection message is stored in
    /// `validation_error` for the consumer to render.
    ///
    /// Default: `None` (no validation — all tags accepted)
    pub validate: Signal<Option<Callback<T, Result<(), String>>>>,
    /// The most recent validation error message, or `None` if valid.
    ///
    /// Set by `add_tag` when `validate` returns `Err(msg)`. Cleared on the
    /// next successful `add_tag` or when `set_query` is called.
    pub validation_error: Signal<Option<String>>,
    // ── Phase 4: Production Guards ─────────────────────────────────────
    /// Whether duplicate tags are allowed.
    ///
    /// When `false` (default), `add_tag` rejects tags whose ID already exists
    /// in `selected_tags`. When `true`, the duplicate check is skipped entirely.
    ///
    /// Default: `false`
    pub allow_duplicates: Signal<bool>,
    /// Optional callback fired when a duplicate tag is rejected.
    ///
    /// Only fires when `allow_duplicates` is `false` and a duplicate is attempted.
    ///
    /// Default: `None`
    pub on_duplicate: Signal<Option<Callback<T>>>,
    /// Whether to restrict tag selection to only items in `available_tags`.
    ///
    /// When `true`, `on_create` is blocked and only tags present in `available_tags`
    /// can be added. Pasted tags not in the allow list are also rejected.
    ///
    /// Default: `false`
    pub enforce_allow_list: Signal<bool>,
    /// List of forbidden tag names (case-insensitive).
    ///
    /// Tags whose `name()` matches any entry (case-insensitive) are rejected by
    /// `add_tag` and filtered out of suggestions.
    ///
    /// Default: `None` (no deny list)
    pub deny_list: Signal<Option<Vec<String>>>,
    /// Minimum number of required tags (informational for form validation).
    ///
    /// Does NOT prevent removal — `is_below_minimum` is a reactive memo that
    /// consumers can use to show validation warnings or disable form submission.
    ///
    /// Default: `None` (no minimum)
    pub min_tags: Signal<Option<usize>>,
    /// Whether the selected tag count is below `min_tags`.
    ///
    /// Reactive memo: `true` when `min_tags` is `Some(n)` and `selected_tags.len() < n`.
    pub is_below_minimum: Memo<bool>,
    /// Whether the tag input is in read-only mode.
    ///
    /// When `true`, tags are displayed but cannot be added, removed, or edited.
    /// Pill navigation (ArrowLeft/Right) and Escape still work.
    ///
    /// Default: `false`
    pub is_readonly: Signal<bool>,
    /// Maximum number of suggestions to show in the dropdown.
    ///
    /// When set, `filtered_suggestions` is truncated after filtering.
    /// `has_no_matches` still reflects the pre-truncation state.
    ///
    /// Default: `None` (unlimited)
    pub max_suggestions: Signal<Option<usize>>,
    /// Whether the current query produces no matching suggestions.
    ///
    /// `true` when `search_query` is non-empty but `filtered_suggestions` is empty
    /// (before `max_suggestions` truncation, if any).
    pub has_no_matches: Memo<bool>,
    // ── Phase 5: Async Data Loading ────────────────────────────────────
    /// Whether suggestions are currently being loaded asynchronously.
    ///
    /// Consumer sets this while fetching. Used for loading spinners and
    /// screen reader announcements.
    ///
    /// Default: `false`
    pub is_loading: Signal<bool>,
    /// Async-fetched suggestions that replace `available_tags` for filtering.
    ///
    /// When `Some`, `filtered_suggestions` filters from this list instead of
    /// `available_tags`. When `None`, the default `available_tags` are used.
    ///
    /// Default: `None`
    pub async_suggestions: Signal<Option<Vec<T>>>,
    /// Optional callback fired when the search query changes.
    ///
    /// Consumer handles the fetch + debounce and sets `async_suggestions` with results.
    ///
    /// Default: `None`
    pub on_search: Signal<Option<Callback<String>>>,
    // ── Phase 6: UX Polish ─────────────────────────────────────────────
    /// Maximum character length for tag names.
    ///
    /// When set, `add_tag` and `create_tag` reject tags whose `name().len()`
    /// exceeds this limit, setting `validation_error`.
    ///
    /// Default: `None` (unlimited)
    pub max_tag_length: Signal<Option<usize>>,
    /// Custom filter function for `use_tag_input()`.
    ///
    /// When `Some`, used instead of the default case-insensitive substring match.
    /// Receives `(tag, lowercase_query)`. Provides parity with the grouped hook's
    /// `TagInputGroupConfig::filter`.
    ///
    /// Default: `None`
    pub filter: Signal<Option<fn(&T, &str) -> bool>>,
    /// Maximum number of tag pills to display before collapsing.
    ///
    /// When set, consumers should render only `visible_tags` and show an
    /// "+N more" badge using `overflow_count`.
    ///
    /// Default: `None` (show all)
    pub max_visible_tags: Signal<Option<usize>>,
    /// Count of tags hidden by `max_visible_tags`.
    ///
    /// `selected_tags.len() - max_visible_tags` when limit is active, else `0`.
    pub overflow_count: Memo<usize>,
    /// The truncated slice of selected tags for rendering.
    ///
    /// When `max_visible_tags` is set, contains only the first N tags.
    /// Otherwise contains all selected tags.
    pub visible_tags: Memo<Vec<T>>,
    /// Optional sort function applied to `selected_tags` after add/remove.
    ///
    /// When `Some`, `selected_tags` is automatically sorted in place after
    /// each mutation.
    ///
    /// Default: `None` (insertion order preserved)
    pub sort_selected: Signal<Option<fn(&T, &T) -> Ordering>>,
    // ── Phase 7: Auto-Complete & Form Helpers ──────────────────────────
    /// The top matching suggestion for auto-complete ghost text.
    ///
    /// First item in `filtered_suggestions` when query is non-empty, else `None`.
    pub auto_complete_suggestion: Memo<Option<T>>,
    /// The completion suffix for ghost text display.
    ///
    /// When `auto_complete_suggestion` is `Some` and the suggestion name starts
    /// with the query (case-insensitive), this contains the remaining characters.
    /// Otherwise empty string.
    pub auto_complete_text: Memo<String>,
    /// JSON-serialized selected tag IDs for hidden form inputs.
    ///
    /// Format: `["id1","id2","id3"]`. Empty array `[]` when no tags selected.
    pub form_value: Memo<String>,
    /// Whether to operate in single-value select mode.
    ///
    /// When `true` and `max_tags` is `Some(1)`, adding a new tag replaces the
    /// existing one instead of rejecting.
    ///
    /// Default: `false`
    pub select_mode: Signal<bool>,
    // ── Phase 8: Enterprise Scale ──────────────────────────────────────
    /// Total count of filtered suggestions before `max_suggestions` truncation.
    ///
    /// Useful for showing "Showing X of Y" UI and for virtual scroller height calculation.
    pub total_filtered_count: Memo<usize>,
    /// Total count of suggestions available (alias for consumer convenience).
    ///
    /// Same as `filtered_suggestions.len()` after truncation. Useful for virtual
    /// scroller item count.
    pub suggestion_count: Memo<usize>,
    /// Unique instance ID for scoping DOM element IDs when multiple tag inputs coexist.
    instance_id: u32,
}

impl<T: TagLike> Clone for TagInputState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: TagLike> Copy for TagInputState<T> {}

impl<T: TagLike> PartialEq for TagInputState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.search_query == other.search_query
            && self.selected_tags == other.selected_tags
            && self.available_tags == other.available_tags
            && self.is_dropdown_open == other.is_dropdown_open
            && self.highlighted_index == other.highlighted_index
            && self.filtered_suggestions == other.filtered_suggestions
            && self.active_pill == other.active_pill
            && self.popover_pill == other.popover_pill
            && self.grouped_suggestions == other.grouped_suggestions
            && self.on_create == other.on_create
            && self.on_remove == other.on_remove
            && self.on_add == other.on_add
            && self.is_disabled == other.is_disabled
            && self.status_message == other.status_message
            && self.on_paste == other.on_paste
            && self.paste_delimiters == other.paste_delimiters
            && self.editing_pill == other.editing_pill
            && self.on_edit == other.on_edit
            && self.on_reorder == other.on_reorder
            && self.delimiters == other.delimiters
            && self.max_tags == other.max_tags
            && self.is_at_limit == other.is_at_limit
            && self.validate == other.validate
            && self.validation_error == other.validation_error
            // Phase 4
            && self.allow_duplicates == other.allow_duplicates
            && self.on_duplicate == other.on_duplicate
            && self.enforce_allow_list == other.enforce_allow_list
            && self.deny_list == other.deny_list
            && self.min_tags == other.min_tags
            && self.is_below_minimum == other.is_below_minimum
            && self.is_readonly == other.is_readonly
            && self.max_suggestions == other.max_suggestions
            && self.has_no_matches == other.has_no_matches
            // Phase 5
            && self.is_loading == other.is_loading
            && self.async_suggestions == other.async_suggestions
            && self.on_search == other.on_search
            // Phase 6
            && self.max_tag_length == other.max_tag_length
            && self.filter == other.filter
            && self.max_visible_tags == other.max_visible_tags
            && self.overflow_count == other.overflow_count
            && self.visible_tags == other.visible_tags
            && self.sort_selected == other.sort_selected
            // Phase 7
            && self.auto_complete_suggestion == other.auto_complete_suggestion
            && self.auto_complete_text == other.auto_complete_text
            && self.form_value == other.form_value
            && self.select_mode == other.select_mode
            // Phase 8
            && self.total_filtered_count == other.total_filtered_count
            && self.suggestion_count == other.suggestion_count
            && self.instance_id == other.instance_id
    }
}

impl<T: TagLike> TagInputState<T> {
    /// Update the search query and open the dropdown.
    ///
    /// Clears any `validation_error` from a previous rejected `add_tag`.
    /// Fires `on_search` callback (if set) for async data loading.
    pub fn set_query(&mut self, query: String) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        self.search_query.set(query.clone());
        self.is_dropdown_open.set(true);
        self.highlighted_index.set(None);
        self.active_pill.set(None);
        self.popover_pill.set(None);
        self.validation_error.set(None);

        // Fire async search callback
        if let Some(cb) = *self.on_search.read() {
            cb.call(query);
        }
    }

    /// Add a tag to the selected list and clear the search query.
    ///
    /// Guards: disabled, readonly, duplicate, allow list, deny list, max_tags limit,
    /// max_tag_length, validation callback, select_mode replacement.
    /// Fires `on_add` callback (if set) after the tag is added.
    /// Updates `status_message` with an announcement like "Apple added. 3 tags selected."
    pub fn add_tag(&mut self, tag: T) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }

        // Allow list enforcement guard
        if *self.enforce_allow_list.read() {
            let in_allow_list = self
                .available_tags
                .read()
                .iter()
                .any(|t| t.id() == tag.id());
            if !in_allow_list {
                self.status_message
                    .set("Only suggestions can be selected.".to_string());
                return;
            }
        }

        // Deny list guard
        if let Some(ref bl) = *self.deny_list.read() {
            let tag_name_lower = tag.name().to_lowercase();
            if bl.iter().any(|b| b.to_lowercase() == tag_name_lower) {
                let name = tag.name().to_string();
                self.status_message.set(format_status_denied(&name));
                self.validation_error.set(Some(format_status_denied(&name)));
                return;
            }
        }

        // Max tag length guard
        if let Some(max_len) = *self.max_tag_length.read() {
            if tag.name().len() > max_len {
                self.validation_error
                    .set(Some(format_error_max_length(max_len)));
                return;
            }
        }

        // Select mode: replace existing tag when max_tags=1
        if *self.select_mode.read() {
            if let Some(1) = *self.max_tags.read() {
                if self.selected_tags.read().len() == 1 {
                    let old_id = self.selected_tags.read()[0].id().to_string();
                    self.selected_tags.write().retain(|t| t.id() != old_id);
                }
            }
        }

        // Max tags guard
        if let Some(max) = *self.max_tags.read() {
            if self.selected_tags.read().len() >= max {
                self.status_message
                    .set(format!("Maximum of {max} tags reached."));
                self.search_query.set(String::new());
                self.is_dropdown_open.set(false);
                return;
            }
        }

        // Duplicate guard
        let already_selected = self.selected_tags.read().iter().any(|t| t.id() == tag.id());
        if already_selected && !*self.allow_duplicates.read() {
            let name = tag.name().to_string();
            self.status_message.set(format_status_duplicate(&name));
            if let Some(cb) = *self.on_duplicate.read() {
                cb.call(tag);
            }
            self.search_query.set(String::new());
            self.highlighted_index.set(None);
            self.is_dropdown_open.set(false);
            self.active_pill.set(None);
            self.popover_pill.set(None);
            return;
        }

        if !already_selected || *self.allow_duplicates.read() {
            // Validation guard
            let validate_cb = *self.validate.read();
            if let Some(cb) = validate_cb {
                if let Err(msg) = cb.call(tag.clone()) {
                    self.validation_error.set(Some(msg));
                    return;
                }
            }
            self.validation_error.set(None);

            let name = tag.name().to_string();
            self.selected_tags.write().push(tag.clone());

            // Auto-sort selected tags if sort function is set
            if let Some(sort_fn) = *self.sort_selected.read() {
                self.selected_tags.write().sort_by(sort_fn);
            }

            let count = self.selected_tags.read().len();
            self.status_message.set(format_status_added(&name, count));

            if let Some(cb) = *self.on_add.read() {
                cb.call(tag);
            }
        }
        self.search_query.set(String::new());
        self.highlighted_index.set(None);
        self.is_dropdown_open.set(false);
        self.active_pill.set(None);
        self.popover_pill.set(None);
    }

    /// Remove a tag from the selected list by its id.
    ///
    /// No-op if the tag is locked (`is_locked() == true`), disabled, or readonly.
    /// Fires `on_remove` callback (if set) before removal.
    /// Updates `status_message` with an announcement like "Cherry removed. 2 tags selected."
    pub fn remove_tag(&mut self, id: &str) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        let is_locked = self
            .selected_tags
            .read()
            .iter()
            .any(|t| t.id() == id && t.is_locked());
        if is_locked {
            return;
        }

        let name = self
            .selected_tags
            .read()
            .iter()
            .find(|t| t.id() == id)
            .map(|t| t.name().to_string());

        if let Some(cb) = *self.on_remove.read() {
            if let Some(tag) = self
                .selected_tags
                .read()
                .iter()
                .find(|t| t.id() == id)
                .cloned()
            {
                cb.call(tag);
            }
        }

        self.selected_tags.write().retain(|t| t.id() != id);
        if let Some(name) = name {
            let count = self.selected_tags.read().len();
            self.status_message.set(format_status_removed(&name, count));
        }
        self.popover_pill.set(None);
    }

    /// Remove the last *unlocked* selected tag (used for Backspace on empty input).
    ///
    /// Walks backwards from the end, skipping locked tags. If all tags are locked, no-op.
    /// Fires `on_remove` callback (if set) before removal.
    /// Updates `status_message` with an announcement.
    pub fn remove_last_tag(&mut self) {
        let tags = self.selected_tags.read();
        if let Some(pos) = tags.iter().rposition(|t| !t.is_locked()) {
            let tag = tags[pos].clone();
            let name = tag.name().to_string();
            drop(tags);

            if let Some(cb) = *self.on_remove.read() {
                cb.call(tag);
            }

            self.selected_tags.write().remove(pos);
            let count = self.selected_tags.read().len();
            self.status_message.set(format_status_removed(&name, count));
        }
    }

    /// Close the dropdown and clear the highlighted selection.
    pub fn close_dropdown(&mut self) {
        self.is_dropdown_open.set(false);
        self.highlighted_index.set(None);
        self.popover_pill.set(None);
    }

    /// Handle click/tap on the input area — clears pill selection and reopens dropdown.
    ///
    /// Attach this to `onclick` on the text `<input>` to fix the mobile bug where
    /// tapping the already-focused input doesn't reopen the dropdown (because
    /// `onfocus` never re-fires).
    pub fn handle_click(&mut self) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        self.active_pill.set(None);
        self.is_dropdown_open.set(true);
        self.highlighted_index.set(None);
        self.popover_pill.set(None);
    }

    /// Toggle the popover for the pill at `index`.
    ///
    /// If the popover is already showing for this pill, it closes.
    /// Opening a popover closes the suggestion dropdown (mutual exclusion).
    pub fn toggle_popover(&mut self, index: usize) {
        let current = *self.popover_pill.read();
        if current == Some(index) {
            self.popover_pill.set(None);
        } else {
            self.popover_pill.set(Some(index));
            self.is_dropdown_open.set(false);
            self.highlighted_index.set(None);
        }
    }

    /// Close any open pill popover.
    pub fn close_popover(&mut self) {
        self.popover_pill.set(None);
    }

    /// Return a stable DOM `id` for the suggestion at `index`.
    ///
    /// Use this as the `id` attribute on each suggestion element so that
    /// keyboard navigation can scroll the highlighted item into view.
    /// The ID is scoped by `instance_id` so multiple tag inputs on the
    /// same page won't collide.
    pub fn suggestion_id(&self, index: usize) -> String {
        format!("dti-{}-s-{}", self.instance_id, index)
    }

    /// Returns the DOM ID for the suggestion listbox container.
    ///
    /// Use this as the `id` on the `<ul>` / `<div role="listbox">` element and as
    /// the value of `aria-controls` / `aria-owns` on the combobox `<input>`.
    pub fn listbox_id(&self) -> String {
        format!("dti-{}-listbox", self.instance_id)
    }

    /// Returns the DOM ID of the currently highlighted suggestion, or an empty string.
    ///
    /// Bind this to `aria-activedescendant` on the combobox `<input>`. An empty
    /// string signals to assistive technology that no option is currently active.
    pub fn active_descendant(&self) -> String {
        match *self.highlighted_index.read() {
            Some(idx) => self.suggestion_id(idx),
            None => String::new(),
        }
    }

    /// Returns `"true"` or `"false"` suitable for `aria-expanded` on the combobox input.
    ///
    /// Reflects whether the suggestion dropdown is currently open.
    pub fn aria_expanded(&self) -> &'static str {
        if *self.is_dropdown_open.read() {
            "true"
        } else {
            "false"
        }
    }

    /// Returns a stable DOM `id` for the selected pill at `index`.
    ///
    /// Use this as the `id` attribute on each pill element for focus management
    /// and ARIA relationships. The ID is scoped by `instance_id` so multiple
    /// tag inputs on the same page won't collide.
    pub fn pill_id(&self, index: usize) -> String {
        format!("dti-{}-p-{}", self.instance_id, index)
    }

    /// Create a tag and add it to both selected and available tags.
    ///
    /// The tag is appended to `available_tags` so it appears in future suggestions
    /// if the user removes and re-types it. Then it is added to `selected_tags`
    /// via `add_tag`.
    ///
    /// Used internally by `handle_keydown`; also available for consumers who want
    /// to trigger creation programmatically.
    pub fn create_tag(&mut self, tag: T) {
        self.available_tags.write().push(tag.clone());
        self.add_tag(tag);
    }

    /// Handle pasted text by splitting it into tags.
    ///
    /// Call this from the consumer's `onpaste` handler after extracting the clipboard
    /// text. The method processes the text according to these rules (in priority order):
    ///
    /// 1. If `on_paste` callback is set: calls it with the raw text. The callback
    ///    returns `Vec<T>` of tags to add.
    /// 2. If `paste_delimiters` is set: splits by delimiters, trims whitespace,
    ///    and passes each non-empty token to `on_create` (if set) to create tags.
    /// 3. Otherwise: no-op (normal paste into the input).
    ///
    /// Updates `status_message` with a summary of how many tags were added.
    pub fn handle_paste(&mut self, text: String) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        if text.is_empty() {
            return;
        }

        // Priority 1: on_paste callback
        let paste_cb = *self.on_paste.read();
        if let Some(cb) = paste_cb {
            let tags = cb.call(text);
            let added = tags.len();
            for tag in tags {
                self.add_tag(tag);
            }
            if added > 0 {
                let count = self.selected_tags.read().len();
                self.status_message.set(format_status_pasted(added, count));
            }
            return;
        }

        // Priority 2: delimiter splitting + on_create
        let delimiters = self.paste_delimiters.read().clone();
        let create_cb = *self.on_create.read();
        if let Some(delimiters) = delimiters {
            if let Some(cb) = create_cb {
                let tokens = split_by_delimiters(&text, &delimiters);
                let mut added = 0;
                for token in tokens {
                    if let Some(tag) = cb.call(token) {
                        self.create_tag(tag);
                        added += 1;
                    }
                }
                if added > 0 {
                    let count = self.selected_tags.read().len();
                    self.status_message.set(format_status_pasted(added, count));
                }
            }
        }

        // Priority 3: no-op, let normal paste happen
    }

    /// Update the status message with the current suggestion count.
    ///
    /// Call this after `filtered_suggestions` changes to announce the available
    /// suggestion count to screen readers. Typically wired via a `use_effect`.
    pub fn announce_suggestions(&mut self, count: usize) {
        if count == 0 {
            self.status_message.set("No suggestions found.".to_string());
        } else {
            self.status_message.set(format_status_suggestions(count));
        }
    }

    // ── Phase 2: Editing methods ────────────────────────────────────────

    /// Enter inline editing mode for the pill at `index`.
    ///
    /// Sets `editing_pill` to `Some(index)` and closes popover/dropdown.
    /// The consumer should render an `<input>` for this pill and call
    /// `commit_edit` or `cancel_edit` when done.
    ///
    /// No-op if `on_edit` callback is not set or if `is_disabled`.
    pub fn start_editing(&mut self, index: usize) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        if self.on_edit.read().is_none() {
            return;
        }
        if index >= self.selected_tags.read().len() {
            return;
        }
        // Don't allow editing locked tags
        if self.selected_tags.read()[index].is_locked() {
            return;
        }
        self.editing_pill.set(Some(index));
        self.popover_pill.set(None);
        self.is_dropdown_open.set(false);
        self.active_pill.set(Some(index));
    }

    /// Commit an inline edit, replacing the tag at the editing index.
    ///
    /// Calls `on_edit` with `(current_tag, new_name)`. If the callback returns
    /// `Some(updated_tag)`, the tag is replaced in `selected_tags`. If it returns
    /// `None`, the edit is rejected and the original tag remains.
    ///
    /// Always exits edit mode afterward.
    pub fn commit_edit(&mut self, new_name: String) {
        let idx = match *self.editing_pill.read() {
            Some(i) => i,
            None => return,
        };
        let edit_cb = *self.on_edit.read();
        if let Some(cb) = edit_cb {
            let current = self.selected_tags.read().get(idx).cloned();
            if let Some(tag) = current {
                if let Some(updated) = cb.call((tag, new_name)) {
                    self.selected_tags.write()[idx] = updated;
                }
            }
        }
        self.editing_pill.set(None);
    }

    /// Cancel inline editing without applying changes.
    pub fn cancel_edit(&mut self) {
        self.editing_pill.set(None);
    }

    // ── Phase 2: Reorder method ─────────────────────────────────────────

    /// Move a tag from one position to another in the selected list.
    ///
    /// Performs `Vec::remove(from)` then `Vec::insert(to, tag)`.
    /// Fires `on_reorder` callback (if set) with `(from, to)` after the move.
    /// Updates `status_message`.
    pub fn move_tag(&mut self, from: usize, to: usize) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        let len = self.selected_tags.read().len();
        if from >= len || to >= len || from == to {
            return;
        }
        let tag = self.selected_tags.write().remove(from);
        let name = tag.name().to_string();
        self.selected_tags.write().insert(to, tag);
        self.status_message
            .set(format!("{name} moved to position {}.", to + 1));

        if let Some(cb) = *self.on_reorder.read() {
            cb.call((from, to));
        }
    }

    // ── Phase 3: Select / Clear all ─────────────────────────────────────

    /// Remove all unlocked tags from the selection.
    ///
    /// Locked tags are preserved. Fires `on_remove` for each removed tag.
    /// Updates `status_message` with a summary.
    pub fn clear_all(&mut self) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        let tags = self.selected_tags.read().clone();
        let to_remove: Vec<T> = tags.into_iter().filter(|t| !t.is_locked()).collect();
        let removed_count = to_remove.len();

        let remove_cb = *self.on_remove.read();
        for tag in &to_remove {
            if let Some(cb) = remove_cb {
                cb.call(tag.clone());
            }
        }

        self.selected_tags.write().retain(|t| t.is_locked());
        self.active_pill.set(None);
        self.popover_pill.set(None);
        self.editing_pill.set(None);

        let locked_count = self.selected_tags.read().len();
        if locked_count > 0 {
            self.status_message.set(format!(
                "All tags cleared. {locked_count} locked tag{} remain{}.",
                if locked_count == 1 { "" } else { "s" },
                if locked_count == 1 { "s" } else { "" }
            ));
        } else {
            self.status_message.set(format!(
                "{removed_count} tag{} cleared.",
                if removed_count == 1 { "" } else { "s" }
            ));
        }
    }

    /// Add all available (unselected) tags to the selection.
    ///
    /// Respects `max_tags` limit — stops adding when the limit is reached.
    /// Updates `status_message` with the count added.
    pub fn select_all(&mut self) {
        if *self.is_disabled.read() || *self.is_readonly.read() {
            return;
        }
        let available = self.filtered_suggestions.read().clone();
        let mut added = 0;
        for tag in available {
            if let Some(max) = *self.max_tags.read() {
                if self.selected_tags.read().len() >= max {
                    break;
                }
            }
            let already = self.selected_tags.read().iter().any(|t| t.id() == tag.id());
            if !already {
                self.selected_tags.write().push(tag.clone());
                added += 1;
                if let Some(cb) = *self.on_add.read() {
                    cb.call(tag);
                }
            }
        }
        if added > 0 {
            let count = self.selected_tags.read().len();
            self.status_message.set(format!(
                "{added} tag{} added. {count} tag{} selected.",
                if added == 1 { "" } else { "s" },
                if count == 1 { "" } else { "s" }
            ));
        }
        self.is_dropdown_open.set(false);
    }

    /// Handle keyboard events for navigating suggestions and pills.
    pub fn handle_keydown(&mut self, event: Event<KeyboardData>) {
        if *self.is_disabled.read() {
            return;
        }
        let pill = *self.active_pill.read();
        if let Some(i) = pill {
            self.handle_pill_keydown(event, i);
        } else {
            self.handle_input_keydown(event);
        }
    }

    /// Handle keyboard events when a pill is keyboard-selected.
    ///
    /// Called by `handle_keydown` when `active_pill` is `Some(i)`, or directly
    /// by compound components that manage their own pill keydown.
    pub fn handle_pill_keydown(&mut self, event: Event<KeyboardData>, pill_index: usize) {
        let key = event.key();
        let readonly = *self.is_readonly.read();

        match key {
            Key::Enter => {
                if readonly {
                    return;
                }
                event.prevent_default();
                self.toggle_popover(pill_index);
            }
            Key::ArrowLeft => {
                event.prevent_default();
                self.popover_pill.set(None);
                if pill_index > 0 {
                    self.active_pill.set(Some(pill_index - 1));
                }
            }
            Key::ArrowRight => {
                event.prevent_default();
                self.popover_pill.set(None);
                let len = self.selected_tags.read().len();
                if pill_index < len - 1 {
                    self.active_pill.set(Some(pill_index + 1));
                } else {
                    self.active_pill.set(None); // back to input
                }
            }
            Key::Backspace | Key::Delete => {
                if readonly {
                    return;
                }
                event.prevent_default();
                if self.popover_pill.read().is_some() {
                    // First press: close popover only (same layered pattern as Escape)
                    self.popover_pill.set(None);
                } else {
                    // Second press (no popover open): delete the pill if not locked
                    let is_locked = self
                        .selected_tags
                        .read()
                        .get(pill_index)
                        .is_some_and(|t| t.is_locked());
                    if !is_locked {
                        let id = self.selected_tags.read()[pill_index].id().to_string();
                        self.remove_tag(&id);
                        let new_len = self.selected_tags.read().len();
                        if new_len == 0 {
                            self.active_pill.set(None);
                        } else if pill_index >= new_len {
                            self.active_pill.set(Some(new_len - 1));
                        }
                        // else: keep same index (now points to the next pill)
                    }
                }
            }
            Key::Home => {
                event.prevent_default();
                self.popover_pill.set(None);
                self.active_pill.set(Some(0));
            }
            Key::End => {
                event.prevent_default();
                self.popover_pill.set(None);
                let len = self.selected_tags.read().len();
                if len > 0 {
                    self.active_pill.set(Some(len - 1));
                }
            }
            Key::Escape => {
                // Layered escape: popover → pill → dropdown
                if self.popover_pill.read().is_some() {
                    self.popover_pill.set(None);
                } else {
                    self.active_pill.set(None);
                    self.close_dropdown();
                }
            }
            _ => {
                if readonly {
                    return;
                }
                // Any typing key exits pill mode so the character goes into the input
                self.active_pill.set(None);
                self.popover_pill.set(None);
            }
        }
    }

    /// Handle keyboard events for the combobox text input.
    ///
    /// Called by `handle_keydown` when no pill is active, or directly by
    /// compound components that manage their own input keydown.
    pub fn handle_input_keydown(&mut self, event: Event<KeyboardData>) {
        let key = event.key();
        let readonly = *self.is_readonly.read();

        // ── Readonly mode: only allow pill entry and escape ─────────────
        if readonly {
            match key {
                Key::ArrowLeft => {
                    if self.search_query.read().is_empty() {
                        let len = self.selected_tags.read().len();
                        if len > 0 {
                            event.prevent_default();
                            self.active_pill.set(Some(len - 1));
                        }
                    }
                }
                Key::Escape => {
                    self.close_dropdown();
                }
                _ => {}
            }
            return;
        }

        // ── Input mode (normal) ─────────────────────────────────────────
        match key {
            Key::ArrowDown => {
                event.prevent_default();
                if !*self.is_dropdown_open.read() {
                    self.is_dropdown_open.set(true);
                }
                let len = self.filtered_suggestions.read().len();
                if len > 0 {
                    let next = match *self.highlighted_index.read() {
                        None => 0,
                        Some(i) => (i + 1) % len,
                    };
                    self.highlighted_index.set(Some(next));
                    scroll_into_view_by_id(&self.suggestion_id(next));
                }
            }
            Key::ArrowUp => {
                event.prevent_default();
                if !*self.is_dropdown_open.read() {
                    self.is_dropdown_open.set(true);
                }
                let len = self.filtered_suggestions.read().len();
                if len > 0 {
                    let next = match *self.highlighted_index.read() {
                        None | Some(0) => len - 1,
                        Some(i) => i - 1,
                    };
                    self.highlighted_index.set(Some(next));
                    scroll_into_view_by_id(&self.suggestion_id(next));
                }
            }
            Key::ArrowLeft => {
                // Enter pill mode from the right when query is empty
                if self.search_query.read().is_empty() {
                    let len = self.selected_tags.read().len();
                    if len > 0 {
                        event.prevent_default();
                        self.active_pill.set(Some(len - 1));
                    }
                }
            }
            Key::Enter => {
                event.prevent_default();
                let highlight = *self.highlighted_index.read();
                if let Some(idx) = highlight {
                    if *self.is_dropdown_open.read() {
                        let suggestions = self.filtered_suggestions.read();
                        if let Some(tag) = suggestions.get(idx).cloned() {
                            drop(suggestions);
                            self.add_tag(tag);
                        }
                    }
                } else {
                    let query = self.search_query.read().clone();
                    let callback = *self.on_create.read();
                    if !query.is_empty() {
                        // enforce_allow_list blocks on_create
                        if *self.enforce_allow_list.read() {
                            // Do nothing — only suggestions can be selected
                            self.is_dropdown_open.set(true);
                        } else if let Some(cb) = callback {
                            if let Some(tag) = cb.call(query) {
                                self.create_tag(tag);
                            }
                        } else {
                            self.is_dropdown_open.set(true);
                        }
                    } else {
                        self.is_dropdown_open.set(true);
                    }
                }
            }
            Key::Tab => {
                // Auto-complete: Tab accepts top suggestion when available
                let ac = self.auto_complete_suggestion.read().clone();
                if ac.is_some() && !self.search_query.read().is_empty() {
                    event.prevent_default();
                    if let Some(tag) = ac {
                        self.add_tag(tag);
                    }
                }
            }
            Key::Backspace => {
                // On empty input, select last *unlocked* pill instead of immediately deleting
                if self.search_query.read().is_empty() {
                    let tags = self.selected_tags.read();
                    if let Some(pos) = tags.iter().rposition(|t| !t.is_locked()) {
                        drop(tags);
                        self.active_pill.set(Some(pos));
                    }
                }
            }
            Key::Escape => {
                self.close_dropdown();
            }
            Key::Character(ref c) => {
                // Custom delimiter: commit query when a delimiter char is typed
                let delims = self.delimiters.read().clone();
                if let Some(delimiters) = delims {
                    if let Some(ch) = c.chars().next() {
                        if delimiters.contains(&ch) {
                            event.prevent_default();
                            let highlight = *self.highlighted_index.read();
                            if let Some(idx) = highlight {
                                let suggestions = self.filtered_suggestions.read();
                                if let Some(tag) = suggestions.get(idx).cloned() {
                                    drop(suggestions);
                                    self.add_tag(tag);
                                }
                            } else {
                                // enforce_allow_list blocks on_create via delimiter too
                                if !*self.enforce_allow_list.read() {
                                    let query = self.search_query.read().clone();
                                    let callback = *self.on_create.read();
                                    if !query.is_empty() {
                                        if let Some(cb) = callback {
                                            if let Some(tag) = cb.call(query) {
                                                self.create_tag(tag);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Create a headless tag input state.
///
/// `available_tags` is the full set of tags the user can choose from.
/// `initial_selected` is the set of tags already selected on mount.
///
/// Returns a `TagInputState<T>` with reactive signals and a memo for filtered suggestions.
#[allow(clippy::type_complexity)]
pub fn use_tag_input<T: TagLike>(
    available_tags: Vec<T>,
    initial_selected: Vec<T>,
) -> TagInputState<T> {
    use_tag_input_with(TagInputConfig::new(available_tags, initial_selected))
}

/// Create a headless tag input state with optional controlled signals.
///
/// This is the configurable version of `use_tag_input`. When `value`, `query`, or `open`
/// signals are provided in the config, the hook uses those directly instead of creating
/// internal ones. All mutations (`add_tag`, `remove_tag`, etc.) write to the provided
/// signal automatically — no callbacks needed.
#[allow(clippy::type_complexity)]
pub fn use_tag_input_with<T: TagLike>(config: TagInputConfig<T>) -> TagInputState<T> {
    let instance_id = use_hook(|| INSTANCE_COUNTER.fetch_add(1, AtomicOrdering::Relaxed));

    // Always create internal signals unconditionally (hook ordering rules).
    // Use parent signal if provided, otherwise internal.
    let internal_query = use_signal(String::new);
    let internal_selected = use_signal(|| config.initial_selected);
    let internal_available = use_signal(|| config.available_tags);
    let internal_open = use_signal(|| false);

    let search_query = config.query.unwrap_or(internal_query);
    let selected_tags = config.value.unwrap_or(internal_selected);
    let available_tags = internal_available;
    let is_dropdown_open = config.open.unwrap_or(internal_open);
    let highlighted_index = use_signal(|| None);

    // Phase 4
    let deny_list: Signal<Option<Vec<String>>> = use_signal(|| None);
    let max_suggestions: Signal<Option<usize>> = use_signal(|| None);
    // Phase 5
    let async_suggestions: Signal<Option<Vec<T>>> = use_signal(|| None);
    // Phase 6
    let filter: Signal<Option<fn(&T, &str) -> bool>> = use_signal(|| None);

    let filtered_suggestions = use_memo(move || {
        let query = search_query.read().to_lowercase();
        let selected = selected_tags.read();

        // Use async_suggestions if available, otherwise available_tags
        let async_sugg = async_suggestions.read();
        let avail;
        let source: &[T] = if let Some(ref items) = *async_sugg {
            items
        } else {
            avail = available_tags.read();
            &avail
        };

        let bl = deny_list.read();

        let mut results: Vec<T> = source
            .iter()
            .filter(|tag| {
                // Exclude already-selected tags
                !selected.iter().any(|s| s.id() == tag.id())
            })
            .filter(|tag| {
                // Deny list filter
                if let Some(ref bl_list) = *bl {
                    let name_lower = tag.name().to_lowercase();
                    if bl_list.iter().any(|b| b.to_lowercase() == name_lower) {
                        return false;
                    }
                }
                true
            })
            .filter(|tag| {
                if query.is_empty() {
                    return true;
                }
                // Custom filter or default substring match
                match *filter.read() {
                    Some(f) => f(tag, &query),
                    None => tag.name().to_lowercase().contains(&query),
                }
            })
            .cloned()
            .collect();

        // Apply max_suggestions truncation
        if let Some(max) = *max_suggestions.read() {
            results.truncate(max);
        }

        results
    });

    let grouped_suggestions =
        use_memo(move || build_groups(&filtered_suggestions.read(), None, None, None));

    let active_pill = use_signal(|| None);
    let popover_pill = use_signal(|| None);
    let on_create = use_signal(|| None);
    let on_remove = use_signal(|| None);
    let on_add = use_signal(|| None);
    let is_disabled = use_signal(|| false);
    let status_message = use_signal(String::new);
    let on_paste = use_signal(|| None);
    let paste_delimiters = use_signal(|| None);
    // Phase 2
    let editing_pill = use_signal(|| None);
    let on_edit = use_signal(|| None);
    let on_reorder = use_signal(|| None);
    // Phase 3
    let delimiters = use_signal(|| None);
    let max_tags: Signal<Option<usize>> = use_signal(|| None);
    let is_at_limit = use_memo(move || match *max_tags.read() {
        Some(max) => selected_tags.read().len() >= max,
        None => false,
    });
    let validate = use_signal(|| None);
    let validation_error = use_signal(|| None);
    // Phase 4
    let allow_duplicates = use_signal(|| false);
    let on_duplicate = use_signal(|| None);
    let enforce_allow_list = use_signal(|| false);
    let min_tags: Signal<Option<usize>> = use_signal(|| None);
    let is_below_minimum = use_memo(move || match *min_tags.read() {
        Some(min) => selected_tags.read().len() < min,
        None => false,
    });
    let is_readonly = use_signal(|| false);
    let has_no_matches = use_memo(move || {
        let query = search_query.read();
        !query.is_empty() && filtered_suggestions.read().is_empty()
    });
    // Phase 5
    let is_loading = use_signal(|| false);
    let on_search = use_signal(|| None);
    // Phase 6
    let max_tag_length = use_signal(|| None);
    let max_visible_tags: Signal<Option<usize>> = use_signal(|| None);
    let overflow_count = use_memo(move || match *max_visible_tags.read() {
        Some(max) => {
            let len = selected_tags.read().len();
            len.saturating_sub(max)
        }
        None => 0,
    });
    let visible_tags = use_memo(move || {
        let tags = selected_tags.read().clone();
        match *max_visible_tags.read() {
            Some(max) => tags.into_iter().take(max).collect(),
            None => tags,
        }
    });
    let sort_selected: Signal<Option<fn(&T, &T) -> Ordering>> = use_signal(|| None);
    // Phase 7
    let auto_complete_suggestion = use_memo(move || {
        let query = search_query.read();
        if query.is_empty() {
            return None;
        }
        filtered_suggestions.read().first().cloned()
    });
    let auto_complete_text = use_memo(move || {
        let query = search_query.read();
        if query.is_empty() {
            return String::new();
        }
        match auto_complete_suggestion.read().as_ref() {
            Some(tag) => {
                let name = tag.name();
                if name.to_lowercase().starts_with(&query.to_lowercase()) {
                    name[query.len()..].to_string()
                } else {
                    String::new()
                }
            }
            None => String::new(),
        }
    });
    let form_value = use_memo(move || {
        let tags = selected_tags.read();
        let ids: Vec<String> = tags.iter().map(|t| format!("\"{}\"", t.id())).collect();
        format!("[{}]", ids.join(","))
    });
    let select_mode = use_signal(|| false);
    // Phase 8
    let total_filtered_count = use_memo(move || {
        // This counts the filtered results before max_suggestions truncation.
        // Since the memo already truncates, we re-compute the pre-truncation count.
        let query = search_query.read().to_lowercase();
        let selected = selected_tags.read();
        let async_sugg = async_suggestions.read();
        let avail;
        let source: &[T] = if let Some(ref items) = *async_sugg {
            items
        } else {
            avail = available_tags.read();
            &avail
        };
        let bl = deny_list.read();
        source
            .iter()
            .filter(|tag| !selected.iter().any(|s| s.id() == tag.id()))
            .filter(|tag| {
                if let Some(ref bl_list) = *bl {
                    let name_lower = tag.name().to_lowercase();
                    if bl_list.iter().any(|b| b.to_lowercase() == name_lower) {
                        return false;
                    }
                }
                true
            })
            .filter(|tag| {
                if query.is_empty() {
                    return true;
                }
                match *filter.read() {
                    Some(f) => f(tag, &query),
                    None => tag.name().to_lowercase().contains(&query),
                }
            })
            .count()
    });
    let suggestion_count = use_memo(move || filtered_suggestions.read().len());

    TagInputState {
        search_query,
        selected_tags,
        available_tags,
        is_dropdown_open,
        highlighted_index,
        filtered_suggestions,
        grouped_suggestions,
        active_pill,
        popover_pill,
        on_create,
        on_remove,
        on_add,
        is_disabled,
        status_message,
        on_paste,
        paste_delimiters,
        editing_pill,
        on_edit,
        on_reorder,
        delimiters,
        max_tags,
        is_at_limit,
        validate,
        validation_error,
        // Phase 4
        allow_duplicates,
        on_duplicate,
        enforce_allow_list,
        deny_list,
        min_tags,
        is_below_minimum,
        is_readonly,
        max_suggestions,
        has_no_matches,
        // Phase 5
        is_loading,
        async_suggestions,
        on_search,
        // Phase 6
        max_tag_length,
        filter,
        max_visible_tags,
        overflow_count,
        visible_tags,
        sort_selected,
        // Phase 7
        auto_complete_suggestion,
        auto_complete_text,
        form_value,
        select_mode,
        // Phase 8
        total_filtered_count,
        suggestion_count,
        instance_id,
    }
}

/// Create a headless tag input state with grouped suggestions, custom filtering, and sorting.
///
/// This is the full-featured version of `use_tag_input`. It uses the `TagLike::group()`
/// method to organize suggestions into labelled sections and supports custom filter/sort
/// functions and per-group item limits.
#[allow(clippy::type_complexity)]
pub fn use_tag_input_grouped<T: TagLike>(config: TagInputGroupConfig<T>) -> TagInputState<T> {
    let instance_id = use_hook(|| INSTANCE_COUNTER.fetch_add(1, AtomicOrdering::Relaxed));

    // Always create internal signals unconditionally (hook ordering rules).
    // Use parent signal if provided, otherwise internal.
    let internal_query = use_signal(String::new);
    let internal_selected = use_signal(|| config.initial_selected);
    let internal_available = use_signal(|| config.available_tags);
    let internal_open = use_signal(|| false);

    let search_query = config.query.unwrap_or(internal_query);
    let selected_tags = config.value.unwrap_or(internal_selected);
    let available_tags = internal_available;
    let is_dropdown_open = config.open.unwrap_or(internal_open);
    let highlighted_index = use_signal(|| None);

    let filter_fn = config.filter;
    let sort_items_fn = config.sort_items;
    let sort_groups_fn = config.sort_groups;
    let max_items = config.max_items_per_group;

    // Phase 4
    let deny_list: Signal<Option<Vec<String>>> = use_signal(|| None);
    let max_suggestions: Signal<Option<usize>> = use_signal(|| None);
    // Phase 5
    let async_suggestions: Signal<Option<Vec<T>>> = use_signal(|| None);

    let filtered_suggestions = use_memo(move || {
        let query = search_query.read().to_lowercase();
        let selected = selected_tags.read();

        // Use async_suggestions if available, otherwise available_tags
        let async_sugg = async_suggestions.read();
        let avail;
        let source: &[T] = if let Some(ref items) = *async_sugg {
            items
        } else {
            avail = available_tags.read();
            &avail
        };

        let bl = deny_list.read();

        let filtered: Vec<T> = source
            .iter()
            .filter(|tag| !selected.iter().any(|s| s.id() == tag.id()))
            .filter(|tag| {
                // Deny list filter
                if let Some(ref bl_list) = *bl {
                    let name_lower = tag.name().to_lowercase();
                    if bl_list.iter().any(|b| b.to_lowercase() == name_lower) {
                        return false;
                    }
                }
                true
            })
            .filter(|tag| {
                if query.is_empty() {
                    return true;
                }
                match filter_fn {
                    Some(f) => f(tag, &query),
                    None => tag.name().to_lowercase().contains(&query),
                }
            })
            .cloned()
            .collect();

        // Build groups, apply sort/limit, then flatten back to the canonical flat order
        let groups = build_groups(&filtered, sort_items_fn, sort_groups_fn, max_items);
        let mut results: Vec<T> = groups.into_iter().flat_map(|g| g.items).collect();

        // Apply max_suggestions truncation
        if let Some(max) = *max_suggestions.read() {
            results.truncate(max);
        }

        results
    });

    let grouped_suggestions = use_memo(move || {
        // Re-group from the already-filtered+sorted flat list (preserves order from above)
        build_groups(
            &filtered_suggestions.read(),
            sort_items_fn,
            sort_groups_fn,
            max_items,
        )
    });

    let active_pill = use_signal(|| None);
    let popover_pill = use_signal(|| None);
    let on_create = use_signal(|| None);
    let on_remove = use_signal(|| None);
    let on_add = use_signal(|| None);
    let is_disabled = use_signal(|| false);
    let status_message = use_signal(String::new);
    let on_paste = use_signal(|| None);
    let paste_delimiters = use_signal(|| None);
    // Phase 2
    let editing_pill = use_signal(|| None);
    let on_edit = use_signal(|| None);
    let on_reorder = use_signal(|| None);
    // Phase 3
    let delimiters = use_signal(|| None);
    let max_tags: Signal<Option<usize>> = use_signal(|| None);
    let is_at_limit = use_memo(move || match *max_tags.read() {
        Some(max) => selected_tags.read().len() >= max,
        None => false,
    });
    let validate = use_signal(|| None);
    let validation_error = use_signal(|| None);
    // Phase 4
    let allow_duplicates = use_signal(|| false);
    let on_duplicate = use_signal(|| None);
    let enforce_allow_list = use_signal(|| false);
    let min_tags: Signal<Option<usize>> = use_signal(|| None);
    let is_below_minimum = use_memo(move || match *min_tags.read() {
        Some(min) => selected_tags.read().len() < min,
        None => false,
    });
    let is_readonly = use_signal(|| false);
    let has_no_matches = use_memo(move || {
        let query = search_query.read();
        !query.is_empty() && filtered_suggestions.read().is_empty()
    });
    // Phase 5
    let is_loading = use_signal(|| false);
    let on_search = use_signal(|| None);
    // Phase 6
    let max_tag_length = use_signal(|| None);
    let filter: Signal<Option<fn(&T, &str) -> bool>> = use_signal(|| None);
    let max_visible_tags: Signal<Option<usize>> = use_signal(|| None);
    let overflow_count = use_memo(move || match *max_visible_tags.read() {
        Some(max) => {
            let len = selected_tags.read().len();
            len.saturating_sub(max)
        }
        None => 0,
    });
    let visible_tags = use_memo(move || {
        let tags = selected_tags.read().clone();
        match *max_visible_tags.read() {
            Some(max) => tags.into_iter().take(max).collect(),
            None => tags,
        }
    });
    let sort_selected: Signal<Option<fn(&T, &T) -> Ordering>> = use_signal(|| None);
    // Phase 7
    let auto_complete_suggestion = use_memo(move || {
        let query = search_query.read();
        if query.is_empty() {
            return None;
        }
        filtered_suggestions.read().first().cloned()
    });
    let auto_complete_text = use_memo(move || {
        let query = search_query.read();
        if query.is_empty() {
            return String::new();
        }
        match auto_complete_suggestion.read().as_ref() {
            Some(tag) => {
                let name = tag.name();
                if name.to_lowercase().starts_with(&query.to_lowercase()) {
                    name[query.len()..].to_string()
                } else {
                    String::new()
                }
            }
            None => String::new(),
        }
    });
    let form_value = use_memo(move || {
        let tags = selected_tags.read();
        let ids: Vec<String> = tags.iter().map(|t| format!("\"{}\"", t.id())).collect();
        format!("[{}]", ids.join(","))
    });
    let select_mode = use_signal(|| false);
    // Phase 8
    let total_filtered_count = use_memo(move || {
        let query = search_query.read().to_lowercase();
        let selected = selected_tags.read();
        let async_sugg = async_suggestions.read();
        let avail;
        let source: &[T] = if let Some(ref items) = *async_sugg {
            items
        } else {
            avail = available_tags.read();
            &avail
        };
        let bl = deny_list.read();
        source
            .iter()
            .filter(|tag| !selected.iter().any(|s| s.id() == tag.id()))
            .filter(|tag| {
                if let Some(ref bl_list) = *bl {
                    let name_lower = tag.name().to_lowercase();
                    if bl_list.iter().any(|b| b.to_lowercase() == name_lower) {
                        return false;
                    }
                }
                true
            })
            .filter(|tag| {
                if query.is_empty() {
                    return true;
                }
                match filter_fn {
                    Some(f) => f(tag, &query),
                    None => tag.name().to_lowercase().contains(&query),
                }
            })
            .count()
    });
    let suggestion_count = use_memo(move || filtered_suggestions.read().len());

    TagInputState {
        search_query,
        selected_tags,
        available_tags,
        is_dropdown_open,
        highlighted_index,
        filtered_suggestions,
        grouped_suggestions,
        active_pill,
        popover_pill,
        on_create,
        on_remove,
        on_add,
        is_disabled,
        status_message,
        on_paste,
        paste_delimiters,
        editing_pill,
        on_edit,
        on_reorder,
        delimiters,
        max_tags,
        is_at_limit,
        validate,
        validation_error,
        // Phase 4
        allow_duplicates,
        on_duplicate,
        enforce_allow_list,
        deny_list,
        min_tags,
        is_below_minimum,
        is_readonly,
        max_suggestions,
        has_no_matches,
        // Phase 5
        is_loading,
        async_suggestions,
        on_search,
        // Phase 6
        max_tag_length,
        filter,
        max_visible_tags,
        overflow_count,
        visible_tags,
        sort_selected,
        // Phase 7
        auto_complete_suggestion,
        auto_complete_text,
        form_value,
        select_mode,
        // Phase 8
        total_filtered_count,
        suggestion_count,
        instance_id,
    }
}

// ---------------------------------------------------------------------------
// Status message formatting helpers (pure functions, testable)
// ---------------------------------------------------------------------------

pub(crate) fn format_status_added(name: &str, total: usize) -> String {
    format!(
        "{name} added. {total} tag{} selected.",
        if total == 1 { "" } else { "s" }
    )
}

pub(crate) fn format_status_removed(name: &str, total: usize) -> String {
    format!(
        "{name} removed. {total} tag{} selected.",
        if total == 1 { "" } else { "s" }
    )
}

pub(crate) fn format_status_pasted(added: usize, total: usize) -> String {
    format!(
        "{added} tag{} pasted. {total} tag{} selected.",
        if added == 1 { "" } else { "s" },
        if total == 1 { "" } else { "s" }
    )
}

pub(crate) fn format_status_suggestions(count: usize) -> String {
    format!(
        "{count} suggestion{} available.",
        if count == 1 { "" } else { "s" }
    )
}

/// Split a string by delimiter characters, trim whitespace, and return non-empty tokens.
pub(crate) fn split_by_delimiters(text: &str, delimiters: &[char]) -> Vec<String> {
    text.split(|c: char| delimiters.contains(&c))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Pure helper functions used in production code
// ---------------------------------------------------------------------------

/// Format the status message for duplicate rejection.
pub(crate) fn format_status_duplicate(name: &str) -> String {
    format!("{name} already exists.")
}

/// Format the status message for deny list rejection.
pub(crate) fn format_status_denied(name: &str) -> String {
    format!("{name} is not allowed.")
}

/// Format the validation error for max tag length.
pub(crate) fn format_error_max_length(max_len: usize) -> String {
    format!("Tag must be {max_len} characters or fewer.")
}

// ---------------------------------------------------------------------------
// Pure helper functions (test-only — production memos inline equivalent logic)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) fn is_denied(name: &str, deny_list: &[String]) -> bool {
    let name_lower = name.to_lowercase();
    deny_list.iter().any(|b| b.to_lowercase() == name_lower)
}

#[cfg(test)]
pub(crate) fn is_in_allow_list<T: TagLike>(id: &str, available: &[T]) -> bool {
    available.iter().any(|t| t.id() == id)
}

#[cfg(test)]
pub(crate) fn filter_denied<T: TagLike>(items: &[T], deny_list: &[String]) -> Vec<T> {
    items
        .iter()
        .filter(|tag| !is_denied(tag.name(), deny_list))
        .cloned()
        .collect()
}

#[cfg(test)]
pub(crate) fn compute_auto_complete_text(query: &str, suggestion_name: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    if suggestion_name
        .to_lowercase()
        .starts_with(&query.to_lowercase())
    {
        suggestion_name[query.len()..].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
pub(crate) fn format_form_value(ids: &[&str]) -> String {
    let quoted: Vec<String> = ids.iter().map(|id| format!("\"{}\"", id)).collect();
    format!("[{}]", quoted.join(","))
}

#[cfg(test)]
pub(crate) fn compute_overflow(total: usize, max_visible: Option<usize>) -> usize {
    match max_visible {
        Some(max) => total.saturating_sub(max),
        None => 0,
    }
}

#[cfg(test)]
pub(crate) fn is_below_min(count: usize, min_tags: Option<usize>) -> bool {
    match min_tags {
        Some(min) => count < min,
        None => false,
    }
}

#[cfg(test)]
pub(crate) fn format_status_truncated(shown: usize, total: usize) -> String {
    format!("Showing {shown} of {total} suggestions. Type to refine.")
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Scroll the element with the given `id` into view using `ScrollLogicalPosition::Nearest`.
///
/// No-op on non-WASM targets.
#[cfg(target_arch = "wasm32")]
fn scroll_into_view_by_id(element_id: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    if let Some(el) = document.get_element_by_id(element_id) {
        let opts = web_sys::ScrollIntoViewOptions::new();
        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_into_view_by_id(_element_id: &str) {}

/// Extract the clipboard text from a paste `Event<ClipboardData>` on WASM targets.
///
/// Uses `web-sys` to cast the underlying event to a `ClipboardEvent` and read
/// `clipboardData.getData("text/plain")`. Returns `None` on non-WASM targets
/// or if the clipboard data is unavailable.
///
/// Typical usage in consumer RSX:
/// ```ignore
/// onpaste: move |evt: Event<ClipboardData>| {
///     if let Some(text) = extract_clipboard_text(&evt) {
///         evt.prevent_default();
///         state.handle_paste(text);
///     }
/// }
/// ```
#[cfg(target_arch = "wasm32")]
pub fn extract_clipboard_text(
    event: &dioxus::prelude::Event<dioxus::prelude::ClipboardData>,
) -> Option<String> {
    use wasm_bindgen::JsCast;
    let clip: &dioxus::prelude::ClipboardData = &event.data();
    let web_event: web_sys::Event = clip.downcast::<web_sys::Event>()?.clone();
    let clipboard_event: web_sys::ClipboardEvent = web_event.dyn_into().ok()?;
    let data_transfer = clipboard_event.clipboard_data()?;
    data_transfer.get_data("text/plain").ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn extract_clipboard_text(
    _event: &dioxus::prelude::Event<dioxus::prelude::ClipboardData>,
) -> Option<String> {
    None
}

/// Build `SuggestionGroup`s from a flat list of tags, preserving first-seen group order.
pub(crate) fn build_groups<T: TagLike>(
    items: &[T],
    sort_items: Option<fn(&T, &T) -> Ordering>,
    sort_groups: Option<fn(&str, &str) -> Ordering>,
    max_items_per_group: Option<usize>,
) -> Vec<SuggestionGroup<T>> {
    // Collect items into groups, preserving first-seen order via Vec of (label, items).
    let mut group_order: Vec<String> = Vec::new();
    let mut group_map: Vec<(String, Vec<T>)> = Vec::new();

    for item in items {
        let label = item.group().unwrap_or("").to_string();
        if let Some(pos) = group_order.iter().position(|l| l == &label) {
            group_map[pos].1.push(item.clone());
        } else {
            group_order.push(label.clone());
            group_map.push((label, vec![item.clone()]));
        }
    }

    // Sort groups if requested
    if let Some(cmp) = sort_groups {
        group_map.sort_by(|(a, _), (b, _)| cmp(a, b));
    }

    // Sort items within each group and apply max_items truncation
    group_map
        .into_iter()
        .map(|(label, mut items)| {
            if let Some(cmp) = sort_items {
                items.sort_by(cmp);
            }
            let total_count = items.len();
            if let Some(max) = max_items_per_group {
                items.truncate(max);
            }
            SuggestionGroup {
                label,
                items,
                total_count,
            }
        })
        .collect()
}
