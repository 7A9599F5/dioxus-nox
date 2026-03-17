use std::rc::Rc;

/// Trait for items that participate in collection scoring and navigation.
///
/// Implemented by select's `ItemEntry`, cmdk's `ItemRegistration`,
/// extensions' `PluginCommand`, and any consumer type.
pub trait ListItem {
    /// Unique value identifying this item.
    fn value(&self) -> &str;
    /// Human-readable label used for display and fuzzy matching.
    fn label(&self) -> &str;
    /// Additional keywords for fuzzy matching (space-separated).
    fn keywords(&self) -> &str {
        ""
    }
    /// Whether this item is disabled (skipped by navigation).
    fn disabled(&self) -> bool {
        false
    }
    /// Optional group this item belongs to.
    fn group_id(&self) -> Option<&str> {
        None
    }
}

// Blanket impl for references
impl<T: ListItem> ListItem for &T {
    fn value(&self) -> &str {
        (*self).value()
    }
    fn label(&self) -> &str {
        (*self).label()
    }
    fn keywords(&self) -> &str {
        (*self).keywords()
    }
    fn disabled(&self) -> bool {
        (*self).disabled()
    }
    fn group_id(&self) -> Option<&str> {
        (*self).group_id()
    }
}

/// An item paired with its fuzzy match score and highlight indices.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoredItem {
    /// The item's value (from `ListItem::value()`).
    pub value: String,
    /// Fuzzy match score (`None` when query is empty = show all).
    pub score: Option<u32>,
    /// Character-offset indices into the label where the match occurred.
    /// `None` when query is empty, matched via keywords only, or custom filter.
    pub match_indices: Option<Vec<u32>>,
}

/// Navigation direction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Direction {
    Forward,
    Backward,
}

/// Optional configuration for advanced scoring (cmdk features).
///
/// When `None` is passed to `score_items`, behavior is identical to
/// select's simple scoring path.
pub struct ScoringConfig {
    /// Items to exclude from scoring entirely.
    pub hidden_values: std::collections::HashSet<String>,
    /// Items that always appear in results regardless of query match.
    pub force_mount_values: std::collections::HashSet<String>,
    /// Additive score modifiers per item value. Applied after nucleo scoring.
    pub boosts: std::collections::HashMap<String, i32>,
    /// Post-scoring strategy for adjusting/filtering results.
    pub strategy: Option<Rc<dyn ScoringStrategy>>,
}

/// Trait for pluggable score adjustment after nucleo matching.
///
/// Return `None` to remove the item from results.
/// Return `Some(score)` to set the final score.
pub trait ScoringStrategy: 'static {
    fn adjust_score(&self, value: &str, raw_score: u32, query: &str) -> Option<u32>;
}

impl std::fmt::Debug for dyn ScoringStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScoringStrategy")
    }
}

/// Wrapper for a custom filter function.
///
/// Receives `(query, label, keywords)` and returns `Option<u32>` where
/// `None` means no match and `Some(score)` is relevance.
///
/// The 3-arg signature is the superset (cmdk uses keywords in custom filters).
/// Select wraps its 2-arg filters to ignore the third argument.
type FilterFn = dyn Fn(&str, &str, &str) -> Option<u32>;

#[derive(Clone)]
pub struct CustomFilter(pub Rc<FilterFn>);

impl CustomFilter {
    /// Create from a 3-arg function `(query, label, keywords) -> Option<score>`.
    pub fn new(f: impl Fn(&str, &str, &str) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(f))
    }

    /// Create from a 2-arg function `(query, label) -> Option<score>`.
    /// The keywords argument is ignored.
    pub fn from_label_only(f: impl Fn(&str, &str) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(move |q, l, _kw| f(q, l)))
    }
}

impl PartialEq for CustomFilter {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustomFilter(..)")
    }
}
