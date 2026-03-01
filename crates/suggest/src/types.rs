use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

// ── Instance ID ───────────────────────────────────────────────────────────────

static NEXT_INSTANCE_ID: AtomicU32 = AtomicU32::new(0);

pub(crate) fn next_instance_id() -> u32 {
    NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
}

// ── TriggerConfig ─────────────────────────────────────────────────────────────

/// Configures one trigger character and its behavior.
#[derive(Clone, PartialEq)]
pub struct TriggerConfig {
    /// The character that activates this trigger (e.g. `'/'`, `'@'`, `'#'`).
    pub char: char,
    /// Require the trigger char to appear at the start of the current line.
    ///
    /// Default: `true` for `'/'`, `false` for `'@'` / `'#'`.
    pub line_start_only: bool,
    /// Maximum byte length of the filter before the trigger is dismissed.
    ///
    /// Default: `64`.
    pub max_filter_len: usize,
    /// Allow spaces in the filter text (useful for `@Full Name` mentions).
    ///
    /// Default: `false`.
    pub allow_spaces: bool,
}

impl TriggerConfig {
    /// Convenience constructor for a slash-command trigger (`/`).
    ///
    /// `line_start_only = true`, `allow_spaces = false`, `max_filter_len = 64`.
    pub fn slash() -> Self {
        Self {
            char: '/',
            line_start_only: true,
            max_filter_len: 64,
            allow_spaces: false,
        }
    }

    /// Convenience constructor for a mention trigger (`@`).
    ///
    /// `line_start_only = false`, `allow_spaces = false`, `max_filter_len = 64`.
    pub fn mention() -> Self {
        Self {
            char: '@',
            line_start_only: false,
            max_filter_len: 64,
            allow_spaces: false,
        }
    }

    /// Convenience constructor for a hashtag trigger (`#`).
    ///
    /// `line_start_only = false`, `allow_spaces = false`, `max_filter_len = 64`.
    pub fn hashtag() -> Self {
        Self {
            char: '#',
            line_start_only: false,
            max_filter_len: 64,
            allow_spaces: false,
        }
    }
}

// ── TriggerSelectEvent ────────────────────────────────────────────────────────

/// Event emitted when the user selects a suggestion item.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerSelectEvent {
    /// Which trigger char was active (`'/'`, `'@'`, `'#'`, …).
    pub trigger_char: char,
    /// The item value selected.
    pub value: String,
    /// The filter text that was typed after the trigger char.
    pub filter: String,
    /// Byte offset of the trigger char in the input text.
    ///
    /// Replace `text[trigger_offset..trigger_offset + filter.len() + trigger_char.len_utf8()]`
    /// with the selected item text to perform insertion.
    pub trigger_offset: usize,
}

// ── TriggerContext ────────────────────────────────────────────────────────────

/// Internal state shared between all `suggest::*` components via Dioxus context.
///
/// Provided by [`suggest::Root`](crate::suggest::Root). Access from descendants
/// via [`use_suggestion`](crate::use_suggestion).
#[derive(Clone, Copy)]
pub struct TriggerContext {
    /// Which trigger char is active, if any.
    pub active_char: Signal<Option<char>>,
    /// The filter text typed after the trigger char.
    pub filter: Signal<String>,
    /// Byte offset of the trigger char in the input text.
    pub trigger_offset: Signal<usize>,
    /// Keyboard-highlighted item index (`None` = nothing highlighted).
    pub(crate) highlighted_index: Signal<Option<usize>>,
    /// Ordered list of registered item values (push-ordered by `Item` mount).
    pub(crate) items: Signal<Vec<String>>,
    /// `on_select` handler provided by `suggest::Root`.
    pub(crate) on_select: Signal<Option<EventHandler<TriggerSelectEvent>>>,
    /// `TriggerConfig` list provided by `suggest::Root`.
    pub(crate) trigger_configs: Signal<Vec<TriggerConfig>>,
    /// Mounted data of the `Trigger` wrapper div; used by `List` for positioning.
    pub(crate) trigger_element: Signal<Option<std::rc::Rc<MountedData>>>,
    /// Caret-level anchor rect `[left, bottom, width]` for precise popover placement.
    /// Populated by `Trigger` via JS eval after trigger detection.
    /// When `Some`, `List` uses this instead of `trigger_element.get_client_rect()`.
    pub(crate) anchor_rect: Signal<Option<[f64; 3]>>,
    /// Unique ID per `Root` instance (used for `PartialEq`).
    pub(crate) instance_id: u32,
}

impl PartialEq for TriggerContext {
    fn eq(&self, other: &Self) -> bool {
        self.instance_id == other.instance_id
    }
}

impl TriggerContext {
    // ── Public surface ────────────────────────────────────────────────────

    /// Deactivate the current trigger without selecting an item.
    ///
    /// Resets `active_char`, `filter`, `trigger_offset`, and `highlighted_index`.
    pub fn close(&self) {
        let mut ac = self.active_char;
        ac.set(None);
        let mut hi = self.highlighted_index;
        hi.set(None);
        let mut f = self.filter;
        f.set(String::new());
        let mut to = self.trigger_offset;
        to.set(0);
        let mut ar = self.anchor_rect;
        ar.set(None);
    }

    // ── Internal navigation ───────────────────────────────────────────────

    pub(crate) fn select_next(&self) {
        let count = self.items.read().len();
        if count == 0 {
            return;
        }
        let mut hi = self.highlighted_index;
        let new = match *hi.read() {
            None => Some(0),
            Some(i) => Some((i + 1).min(count - 1)),
        };
        hi.set(new);
    }

    pub(crate) fn select_prev(&self) {
        let count = self.items.read().len();
        if count == 0 {
            return;
        }
        let mut hi = self.highlighted_index;
        let new = match *hi.read() {
            None => Some(count.saturating_sub(1)),
            Some(0) => Some(0),
            Some(i) => Some(i - 1),
        };
        hi.set(new);
    }

    pub(crate) fn confirm_selection(&self) {
        let hi = *self.highlighted_index.read();
        let ac = *self.active_char.read();
        let (Some(idx), Some(trigger_char)) = (hi, ac) else {
            return;
        };
        let items = self.items.read();
        let Some(value) = items.get(idx).cloned() else {
            return;
        };
        let filter = self.filter.read().clone();
        let trigger_offset = *self.trigger_offset.read();
        drop(items);
        let event = TriggerSelectEvent {
            trigger_char,
            value,
            filter,
            trigger_offset,
        };
        if let Some(ref h) = *self.on_select.read() {
            h.call(event);
        }
        self.close();
    }

    // ── Item registration (called by suggest::Item on mount / drop) ───────

    pub(crate) fn register_item(&self, value: String) {
        let mut items = self.items;
        items.write().push(value);
    }

    pub(crate) fn unregister_item(&self, value: &str) {
        let mut items = self.items;
        // The read guard must be dropped before we can write; wrap in a block to
        // force the `GenerationalRef` temporary to drop before `items.write()`.
        let pos = {
            let guard = items.read();
            guard.iter().position(|v| v == value)
        };
        if let Some(pos) = pos {
            items.write().remove(pos);
        }
    }
}
