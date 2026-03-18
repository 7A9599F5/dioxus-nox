use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use nucleo_matcher::Matcher;

use crate::types::{CustomFilter, ItemRegistration, ScoredItem, ScoringStrategy};

/// Score command items against a query.
///
/// Constructs a `ScoringConfig` from cmdk-specific item fields (hidden, force_mount,
/// boost) and delegates to `dioxus_nox_collection::score_items`.
pub fn score_items(
    items: &[Rc<ItemRegistration>],
    query: &str,
    custom_filter: Option<CustomFilter>,
    scoring_strategy: Option<&dyn ScoringStrategy>,
    matcher: &mut Matcher,
) -> Vec<ScoredItem> {
    // Build ScoringConfig from per-item flags
    let hidden_values: HashSet<String> = items
        .iter()
        .filter(|i| i.hidden)
        .map(|i| i.id.clone())
        .collect();
    let force_mount_values: HashSet<String> = items
        .iter()
        .filter(|i| i.force_mount)
        .map(|i| i.id.clone())
        .collect();
    let boosts: HashMap<String, i32> = items
        .iter()
        .filter(|i| i.boost != 0)
        .map(|i| (i.id.clone(), i.boost))
        .collect();

    let has_config = !hidden_values.is_empty()
        || !force_mount_values.is_empty()
        || !boosts.is_empty()
        || scoring_strategy.is_some();

    // Wrap the strategy reference in Rc for config (safe: lives for this call)
    let strategy_rc: Option<Rc<dyn ScoringStrategy>> = scoring_strategy.map(|s| {
        // Create a wrapper that holds a raw pointer — only valid for this call scope
        struct StrategyRef(*const dyn ScoringStrategy);
        impl ScoringStrategy for StrategyRef {
            fn adjust_score(&self, value: &str, raw_score: u32, query: &str) -> Option<u32> {
                unsafe { (*self.0).adjust_score(value, raw_score, query) }
            }
        }
        Rc::new(StrategyRef(s as *const dyn ScoringStrategy)) as Rc<dyn ScoringStrategy>
    });

    let config = if has_config {
        Some(dioxus_nox_collection::ScoringConfig {
            hidden_values,
            force_mount_values,
            boosts,
            strategy: strategy_rc,
        })
    } else {
        None
    };

    // Bridge custom filter
    let cf_bridge = custom_filter.map(|cf| {
        dioxus_nox_collection::CustomFilter::new(move |q: &str, l: &str, kw: &str| (cf.0)(q, l, kw))
    });

    // Deref Rc for ListItem trait access
    let items_ref: Vec<&ItemRegistration> = items.iter().map(|i| i.as_ref()).collect();

    let collection_results = dioxus_nox_collection::score_items(
        &items_ref,
        query,
        cf_bridge.as_ref(),
        config.as_ref(),
        matcher,
    );

    // Map collection ScoredItem (value) → cmdk ScoredItem (id)
    collection_results
        .into_iter()
        .map(|si| ScoredItem {
            id: si.value,
            score: si.score,
            match_indices: si.match_indices,
        })
        .collect()
}
