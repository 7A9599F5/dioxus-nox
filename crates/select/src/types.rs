use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

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

// ── GroupEntry ───────────────────────────────────────────────────────────────

/// Registration entry for an option group.
#[derive(Clone, PartialEq, Debug)]
pub struct GroupEntry {
    /// Unique group identifier.
    pub id: String,
    /// Optional heading for the group.
    pub label: Option<String>,
}

// ── ScoredItem ──────────────────────────────────────────────────────────────

/// An item paired with its fuzzy match score and highlight indices.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoredItem {
    /// The item's value.
    pub value: String,
    /// Fuzzy match score (`None` when query is empty = show all).
    pub score: Option<u32>,
    /// Byte-offset indices into the label where the match occurred.
    pub match_indices: Option<Vec<u32>>,
}

// ── CustomFilter ────────────────────────────────────────────────────────────

/// Wrapper for a custom filter function.
///
/// Receives `(query, item_label)` → `Option<u32>` where `None` means no match
/// and `Some(score)` is the relevance score.
/// Inner type alias for the custom filter function.
type FilterFn = dyn Fn(&str, &str) -> Option<u32>;

#[derive(Clone)]
pub struct CustomFilter(pub Rc<FilterFn>);

impl CustomFilter {
    pub fn new(f: impl Fn(&str, &str) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(f))
    }
}

impl PartialEq for CustomFilter {
    fn eq(&self, _other: &Self) -> bool {
        // Always not-equal to ensure reactive updates on filter swap.
        false
    }
}

impl std::fmt::Debug for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustomFilter(..)")
    }
}
