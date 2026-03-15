use std::rc::Rc;

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::types::{CustomFilter, ItemRegistration, ScoredItem, ScoringStrategy};

/// Pure scoring function. Scores all items against the query, returning
/// visible items sorted by descending score. Re-uses caller-owned `Matcher`
/// and internal `Vec<char>` buffers to avoid per-call allocation.
pub fn score_items(
    items: &[Rc<ItemRegistration>],
    query: &str,
    custom_filter: Option<CustomFilter>,
    scoring_strategy: Option<&dyn ScoringStrategy>,
    matcher: &mut Matcher,
) -> Vec<ScoredItem> {
    // Exclude hidden items before any scoring
    let active_items: Vec<&ItemRegistration> = items
        .iter()
        .map(|i| i.as_ref())
        .filter(|i| !i.hidden)
        .collect();

    if query.is_empty() {
        return active_items
            .iter()
            .map(|item| ScoredItem {
                id: item.id.clone(),
                score: None,
                match_indices: None,
            })
            .collect();
    }

    // Custom filter path
    if let Some(cf) = custom_filter {
        return active_items
            .iter()
            .filter_map(|item| {
                if item.force_mount {
                    return Some(ScoredItem {
                        id: item.id.clone(),
                        score: None,
                        match_indices: None,
                    });
                }
                (cf.0)(query, &item.label, &item.keywords_cached).map(|score| ScoredItem {
                    id: item.id.clone(),
                    score: Some(score),
                    match_indices: None,
                })
            })
            .collect();
    }

    // Nucleo fuzzy matching with reused buffers
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
            if item.force_mount {
                return Some(ScoredItem {
                    id: item.id.clone(),
                    score: None,
                    match_indices: None,
                });
            }
            // Score against label with indices
            buf.clear();
            let haystack = Utf32Str::new(&item.label, &mut buf);
            let mut label_indices: Vec<u32> = Vec::new();
            let label_score = pattern.indices(haystack, matcher, &mut label_indices);

            // Score against cached keywords (indices not needed — we only highlight the label)
            buf.clear();
            let kw_haystack = Utf32Str::new(&item.keywords_cached, &mut buf);
            let kw_score = pattern.score(kw_haystack, matcher);

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
                None // Matched via keywords, no label highlights
            };

            best.map(|score| ScoredItem {
                id: item.id.clone(),
                score: Some((score as i32 + item.boost).max(0) as u32),
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
    if let Some(strategy) = scoring_strategy {
        results = results
            .into_iter()
            .filter_map(|si| match si.score {
                Some(raw) => strategy
                    .adjust_score(&si.id, raw, query)
                    .map(|adjusted| ScoredItem {
                        id: si.id,
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
