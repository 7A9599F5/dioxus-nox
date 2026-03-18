use std::collections::HashSet;

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::types::{CustomFilter, ListItem, ScoredItem, ScoringConfig};

/// Score and filter items against a query using nucleo fuzzy matching.
///
/// When `query` is empty, returns all (non-hidden) items with `score: None`.
/// When a `custom_filter` is provided it takes precedence over nucleo.
/// When `config` is provided, hidden/force_mount/boost/strategy are applied.
///
/// Results are sorted by descending score.
pub fn score_items<T: ListItem>(
    items: &[T],
    query: &str,
    custom_filter: Option<&CustomFilter>,
    config: Option<&ScoringConfig>,
    matcher: &mut Matcher,
) -> Vec<ScoredItem> {
    let hidden = config.map(|c| &c.hidden_values);
    let force_mount = config.map(|c| &c.force_mount_values);
    let boosts = config.map(|c| &c.boosts);

    // Filter out hidden items
    let active_items: Vec<&T> = items
        .iter()
        .filter(|i| hidden.map(|h| !h.contains(i.value())).unwrap_or(true))
        .collect();

    if query.is_empty() {
        return active_items
            .iter()
            .map(|item| ScoredItem {
                value: item.value().to_string(),
                score: None,
                match_indices: None,
            })
            .collect();
    }

    // Custom filter path
    if let Some(cf) = custom_filter {
        let mut results: Vec<ScoredItem> = active_items
            .iter()
            .filter_map(|item| {
                // Force-mount items always included
                if force_mount.is_some_and(|fm| fm.contains(item.value())) {
                    return Some(ScoredItem {
                        value: item.value().to_string(),
                        score: None,
                        match_indices: None,
                    });
                }
                (cf.0)(query, item.label(), item.keywords()).map(|score| ScoredItem {
                    value: item.value().to_string(),
                    score: Some(score),
                    match_indices: None,
                })
            })
            .collect();
        results.sort_by(|a, b| {
            let sa = a.score.unwrap_or(0);
            let sb = b.score.unwrap_or(0);
            sb.cmp(&sa)
        });
        return results;
    }

    // Nucleo fuzzy matching
    let pattern = Pattern::new(
        query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );

    let mut buf: Vec<char> = Vec::new();

    let mut results: Vec<ScoredItem> = active_items
        .iter()
        .filter_map(|item| {
            // Force-mount items always included
            if force_mount.is_some_and(|fm| fm.contains(item.value())) {
                return Some(ScoredItem {
                    value: item.value().to_string(),
                    score: None,
                    match_indices: None,
                });
            }

            // Score against label with indices
            buf.clear();
            let haystack = Utf32Str::new(item.label(), &mut buf);
            let mut label_indices: Vec<u32> = Vec::new();
            let label_score = pattern.indices(haystack, matcher, &mut label_indices);

            // Score against keywords (indices not needed)
            let kw = item.keywords();
            let kw_score = if kw.is_empty() {
                None
            } else {
                buf.clear();
                let kw_haystack = Utf32Str::new(kw, &mut buf);
                pattern.score(kw_haystack, matcher)
            };

            // Take the best score
            let best = match (label_score, kw_score) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };

            let final_indices = if label_score.is_some() {
                Some(label_indices)
            } else {
                None
            };

            // Apply boost
            let boosted_score = best.map(|score| {
                let boost = boosts
                    .and_then(|b| b.get(item.value()))
                    .copied()
                    .unwrap_or(0);
                (score as i32 + boost).max(0) as u32
            });

            boosted_score.map(|score| ScoredItem {
                value: item.value().to_string(),
                score: Some(score),
                match_indices: final_indices,
            })
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| {
        let sa = a.score.unwrap_or(0);
        let sb = b.score.unwrap_or(0);
        sb.cmp(&sa)
    });

    // Apply scoring strategy if provided
    if let Some(strategy) = config.and_then(|c| c.strategy.as_ref()) {
        results = results
            .into_iter()
            .filter_map(|si| match si.score {
                Some(raw) => strategy
                    .adjust_score(&si.value, raw, query)
                    .map(|adjusted| ScoredItem {
                        value: si.value,
                        score: Some(adjusted),
                        match_indices: si.match_indices,
                    }),
                None => Some(si), // force_mount / empty query — pass through
            })
            .collect();

        // Re-sort after strategy adjustment
        results.sort_by(|a, b| {
            let sa = a.score.unwrap_or(0);
            let sb = b.score.unwrap_or(0);
            sb.cmp(&sa)
        });
    }

    results
}

/// Extract the visible values from scored items (in score order).
pub fn visible_values(scored: &[ScoredItem]) -> Vec<String> {
    scored.iter().map(|si| si.value.clone()).collect()
}

/// Extract the visible values as a HashSet (for O(1) lookups).
pub fn visible_values_set(scored: &[ScoredItem]) -> HashSet<String> {
    scored.iter().map(|si| si.value.clone()).collect()
}
