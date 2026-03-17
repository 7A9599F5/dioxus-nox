use std::sync::atomic::{AtomicU32, Ordering};

use dioxus_nox_collection::ListItem;

pub(crate) static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ── AutoComplete ────────────────────────────────────────────────────────────

/// Controls the autocomplete behaviour of an editable combobox.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum AutoComplete {
    /// No autocomplete. Popup shows all items regardless of input.
    #[default]
    None,
    /// Filter the list based on input text.
    List,
    /// Filter the list AND provide inline completion in the input.
    Both,
}

impl AutoComplete {
    /// Value for the `aria-autocomplete` attribute.
    pub fn as_aria_attr(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::List => "list",
            Self::Both => "both",
        }
    }
}

// ── ItemEntry ───────────────────────────────────────────────────────────────

/// Registration entry for a single select option.
#[derive(Clone, PartialEq, Debug)]
pub struct ItemEntry {
    /// Unique value identifying this option.
    pub value: String,
    /// Human-readable label used for display and fuzzy matching.
    pub label: String,
    /// Additional keywords for fuzzy matching (space-separated).
    pub keywords: String,
    /// Whether this option is disabled.
    pub disabled: bool,
    /// Optional group this option belongs to.
    pub group_id: Option<String>,
}

impl ListItem for ItemEntry {
    fn value(&self) -> &str {
        &self.value
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn keywords(&self) -> &str {
        &self.keywords
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn group_id(&self) -> Option<&str> {
        self.group_id.as_deref()
    }
}

// ── GroupEntry ───────────────────────────────────────────────────────────────

/// Registration entry for an option group.
#[derive(Clone, PartialEq, Debug)]
pub struct GroupEntry {
    /// Unique group identifier.
    pub id: String,
    /// Optional heading for the group.
    pub label: Option<String>,
}

// ── ScoredItem & CustomFilter ────────────────────────────────────────────────

// Re-exported from dioxus-nox-collection.
pub use dioxus_nox_collection::{CustomFilter, ScoredItem};
