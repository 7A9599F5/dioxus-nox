use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::types::{CustomFilter, ItemEntry, ScoredItem};

/// Score and filter items against a query using nucleo fuzzy matching.
///
/// When `query` is empty, returns all items with `score: None` (no filtering).
/// When a `custom_filter` is provided it takes precedence over nucleo.
///
/// Results are sorted by descending score.
pub fn score_items(
    items: &[ItemEntry],
    query: &str,
    custom_filter: Option<&CustomFilter>,
    matcher: &mut Matcher,
) -> Vec<ScoredItem> {
    if query.is_empty() {
        return items
            .iter()
            .map(|item| ScoredItem {
                value: item.value.clone(),
                score: None,
                match_indices: None,
            })
            .collect();
    }

    // Custom filter path
    if let Some(cf) = custom_filter {
        let mut results: Vec<ScoredItem> = items
            .iter()
            .filter_map(|item| {
                (cf.0)(query, &item.label).map(|score| ScoredItem {
                    value: item.value.clone(),
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

    let mut results: Vec<ScoredItem> = items
        .iter()
        .filter_map(|item| {
            // Score against label with indices
            buf.clear();
            let haystack = Utf32Str::new(&item.label, &mut buf);
            let mut label_indices: Vec<u32> = Vec::new();
            let label_score = pattern.indices(haystack, matcher, &mut label_indices);

            // Score against keywords (indices not needed — we only highlight the label)
            let kw_score = if item.keywords.is_empty() {
                None
            } else {
                buf.clear();
                let kw_haystack = Utf32Str::new(&item.keywords, &mut buf);
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
                None // Matched via keywords, no label highlights
            };

            best.map(|score| ScoredItem {
                value: item.value.clone(),
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

    results
}

/// Extract the visible values from scored items (in score order).
pub fn visible_values(scored: &[ScoredItem]) -> Vec<String> {
    scored.iter().map(|si| si.value.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleo_matcher::Config;

    fn make_items(specs: &[(&str, &str, &str)]) -> Vec<ItemEntry> {
        specs
            .iter()
            .map(|(v, l, kw)| ItemEntry {
                value: v.to_string(),
                label: l.to_string(),
                keywords: kw.to_string(),
                disabled: false,
                group_id: None,
            })
            .collect()
    }

    fn matcher() -> Matcher {
        Matcher::new(Config::DEFAULT)
    }

    #[test]
    fn empty_query_returns_all() {
        let items = make_items(&[("a", "Apple", ""), ("b", "Banana", "")]);
        let results = score_items(&items, "", None, &mut matcher());
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.score.is_none()));
    }

    #[test]
    fn fuzzy_match_filters() {
        let items = make_items(&[("a", "Apple", ""), ("b", "Banana", ""), ("c", "Cherry", "")]);
        let results = score_items(&items, "ban", None, &mut matcher());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "b");
        assert!(results[0].score.is_some());
    }

    #[test]
    fn fuzzy_match_returns_indices() {
        let items = make_items(&[("a", "Apple", "")]);
        let results = score_items(&items, "apl", None, &mut matcher());
        assert_eq!(results.len(), 1);
        assert!(results[0].match_indices.is_some());
    }

    #[test]
    fn keyword_match() {
        let items = make_items(&[("a", "Red Fruit", "apple crimson")]);
        let results = score_items(&items, "apple", None, &mut matcher());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "a");
    }

    #[test]
    fn no_match_returns_empty() {
        let items = make_items(&[("a", "Apple", ""), ("b", "Banana", "")]);
        let results = score_items(&items, "zzz", None, &mut matcher());
        assert!(results.is_empty());
    }

    #[test]
    fn custom_filter_used_when_provided() {
        let items = make_items(&[("a", "Apple", ""), ("b", "Banana", "")]);
        let cf = CustomFilter::new(|query, label| {
            if label.to_lowercase().starts_with(&query.to_lowercase()) {
                Some(100)
            } else {
                None
            }
        });
        let results = score_items(&items, "ban", Some(&cf), &mut matcher());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "b");
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let items = make_items(&[("a", "abcdef", ""), ("b", "abc", ""), ("c", "ab", "")]);
        let results = score_items(&items, "abc", None, &mut matcher());
        // Higher score should come first
        for w in results.windows(2) {
            assert!(w[0].score.unwrap_or(0) >= w[1].score.unwrap_or(0));
        }
    }

    #[test]
    fn visible_values_preserves_order() {
        let scored = vec![
            ScoredItem {
                value: "b".into(),
                score: Some(100),
                match_indices: None,
            },
            ScoredItem {
                value: "a".into(),
                score: Some(50),
                match_indices: None,
            },
        ];
        assert_eq!(
            visible_values(&scored),
            vec!["b".to_string(), "a".to_string()]
        );
    }
}
