use nucleo_matcher::Matcher;

use crate::types::{CustomFilter, ItemEntry, ScoredItem};

/// Score and filter items against a query using nucleo fuzzy matching.
///
/// Delegates to `dioxus_nox_collection::score_items`.
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
    dioxus_nox_collection::score_items(items, query, custom_filter, None, matcher)
}

/// Extract the visible values from scored items (in score order).
pub fn visible_values(scored: &[ScoredItem]) -> Vec<String> {
    dioxus_nox_collection::visible_values(scored)
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
        let cf = CustomFilter::from_label_only(|query, label| {
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
