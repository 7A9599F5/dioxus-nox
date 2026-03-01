use dioxus::prelude::*;

use crate::types::TriggerContext;

/// Access suggestion state from any descendant of `suggest::Root`.
///
/// Must be called inside a component tree where
/// [`suggest::Root`](crate::suggest::Root) is an ancestor.
/// Panics if called outside a `suggest::Root` tree.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_nox_suggest::use_suggestion;
///
/// #[component]
/// fn MyInput() -> Element {
///     let sg = use_suggestion();
///     // Feed the active filter into your item list:
///     let filter = sg.filter();
///     // …
/// }
/// ```
pub fn use_suggestion() -> SuggestionHandle {
    SuggestionHandle {
        ctx: use_context::<TriggerContext>(),
    }
}

/// Read-only handle providing access to the current suggestion state.
///
/// Returned by [`use_suggestion`].
pub struct SuggestionHandle {
    ctx: TriggerContext,
}

impl SuggestionHandle {
    /// The currently active trigger character, or `None` if no trigger is active.
    pub fn active_char(&self) -> Option<char> {
        *self.ctx.active_char.read()
    }

    /// The filter text typed after the trigger char (empty string when trigger just fired).
    pub fn filter(&self) -> String {
        self.ctx.filter.read().clone()
    }

    /// Byte offset of the trigger char in the input text.
    ///
    /// Useful for computing the replacement range when inserting a selected value.
    pub fn trigger_offset(&self) -> usize {
        *self.ctx.trigger_offset.read()
    }

    /// Whether a trigger is currently active.
    pub fn is_open(&self) -> bool {
        self.ctx.active_char.read().is_some()
    }

    /// Deactivate the current trigger without selecting an item.
    pub fn close(&self) {
        self.ctx.close();
    }

    /// Handle a keydown event when the suggestion popover is open.
    ///
    /// Returns `true` if the key was consumed (caller should `prevent_default`
    /// and `stop_propagation`), `false` if the key should pass through.
    pub fn handle_keydown(&self, key: &str) -> bool {
        self.ctx.handle_keydown(key)
    }
}
