use std::collections::{HashMap, HashSet};

use nucleo_matcher::{Config, Matcher};

use crate::navigation::{first, last, navigate, navigate_by, type_ahead};
use crate::scoring::{score_items, visible_values, visible_values_set};
use crate::types::{CustomFilter, Direction, ListItem, ScoredItem, ScoringConfig, ScoringStrategy};

// ── Test item type ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct TestItem {
    value: String,
    label: String,
    keywords: String,
    disabled: bool,
    group_id: Option<String>,
}

impl ListItem for TestItem {
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

fn ti(value: &str, label: &str) -> TestItem {
    TestItem {
        value: value.to_string(),
        label: label.to_string(),
        keywords: String::new(),
        disabled: false,
        group_id: None,
    }
}

fn ti_kw(value: &str, label: &str, keywords: &str) -> TestItem {
    TestItem {
        value: value.to_string(),
        label: label.to_string(),
        keywords: keywords.to_string(),
        disabled: false,
        group_id: None,
    }
}

fn ti_disabled(value: &str, label: &str) -> TestItem {
    TestItem {
        value: value.to_string(),
        label: label.to_string(),
        keywords: String::new(),
        disabled: true,
        group_id: None,
    }
}

#[allow(dead_code)]
fn ti_group(value: &str, label: &str, group: &str) -> TestItem {
    TestItem {
        value: value.to_string(),
        label: label.to_string(),
        keywords: String::new(),
        disabled: false,
        group_id: Some(group.to_string()),
    }
}

fn matcher() -> Matcher {
    Matcher::new(Config::DEFAULT)
}

fn vals(specs: &[&str]) -> Vec<String> {
    specs.iter().map(|s| s.to_string()).collect()
}

// =========================================================================
// Scoring tests (ported from select filter.rs + cmdk scoring.rs)
// =========================================================================

#[test]
fn empty_query_returns_all() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana")];
    let results = score_items(&items, "", None, None, &mut matcher());
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.score.is_none()));
}

#[test]
fn fuzzy_match_filters() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana"), ti("c", "Cherry")];
    let results = score_items(&items, "ban", None, None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "b");
    assert!(results[0].score.is_some());
}

#[test]
fn fuzzy_match_returns_indices() {
    let items = vec![ti("a", "Apple")];
    let results = score_items(&items, "apl", None, None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert!(results[0].match_indices.is_some());
}

#[test]
fn keyword_match() {
    let items = vec![ti_kw("a", "Red Fruit", "apple crimson")];
    let results = score_items(&items, "apple", None, None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "a");
}

#[test]
fn no_match_returns_empty() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana")];
    let results = score_items(&items, "zzz", None, None, &mut matcher());
    assert!(results.is_empty());
}

#[test]
fn empty_items_returns_empty() {
    let items: Vec<TestItem> = vec![];
    let results = score_items(&items, "test", None, None, &mut matcher());
    assert!(results.is_empty());
}

#[test]
fn custom_filter_used_when_provided() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana")];
    let cf = CustomFilter::from_label_only(|query, label| {
        if label.to_lowercase().starts_with(&query.to_lowercase()) {
            Some(100)
        } else {
            None
        }
    });
    let results = score_items(&items, "ban", Some(&cf), None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "b");
}

#[test]
fn custom_filter_3arg() {
    let items = vec![
        ti_kw("a", "Open File", ""),
        ti_kw("b", "Settings", "config preferences"),
    ];
    let cf = CustomFilter::new(|query, _label, keywords| {
        if keywords.contains(query) {
            Some(100)
        } else {
            None
        }
    });
    let results = score_items(&items, "config", Some(&cf), None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "b");
}

#[test]
fn results_sorted_by_score_descending() {
    let items = vec![ti("a", "abcdef"), ti("b", "abc"), ti("c", "ab")];
    let results = score_items(&items, "abc", None, None, &mut matcher());
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

#[test]
fn visible_values_set_works() {
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
    let set = visible_values_set(&scored);
    assert!(set.contains("a"));
    assert!(set.contains("b"));
    assert_eq!(set.len(), 2);
}

// ── ScoringConfig tests (hidden, force_mount, boost, strategy) ─────────

fn config_hidden(values: &[&str]) -> ScoringConfig {
    ScoringConfig {
        hidden_values: values.iter().map(|s| s.to_string()).collect(),
        force_mount_values: HashSet::new(),
        boosts: HashMap::new(),
        strategy: None,
    }
}

fn config_force_mount(values: &[&str]) -> ScoringConfig {
    ScoringConfig {
        hidden_values: HashSet::new(),
        force_mount_values: values.iter().map(|s| s.to_string()).collect(),
        boosts: HashMap::new(),
        strategy: None,
    }
}

fn config_boost(boosts: &[(&str, i32)]) -> ScoringConfig {
    ScoringConfig {
        hidden_values: HashSet::new(),
        force_mount_values: HashSet::new(),
        boosts: boosts.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        strategy: None,
    }
}

#[test]
fn hidden_items_excluded() {
    let items = vec![ti("a", "Alpha"), ti("h", "Hidden"), ti("b", "Beta")];
    let cfg = config_hidden(&["h"]);
    let results = score_items(&items, "", None, Some(&cfg), &mut matcher());
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.value != "h"));
}

#[test]
fn force_mount_always_included() {
    let items = vec![ti("a", "Alpha"), ti("fm", "ForceMounted")];
    let cfg = config_force_mount(&["fm"]);
    let results = score_items(&items, "zzzznothing", None, Some(&cfg), &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "fm");
    assert!(results[0].score.is_none());
}

#[test]
fn custom_filter_respects_force_mount() {
    let items = vec![ti("a", "Alpha"), ti("fm", "ForceMounted")];
    let cfg = config_force_mount(&["fm"]);
    let cf = CustomFilter::new(|_q, _l, _kw| None); // reject everything
    let results = score_items(&items, "anything", Some(&cf), Some(&cfg), &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "fm");
}

#[test]
fn boost_adjusts_score() {
    let items = vec![ti("a", "Alpha"), ti("b", "Alpha Beta")];
    let cfg = config_boost(&[("a", 100)]);
    let results = score_items(&items, "alpha", None, Some(&cfg), &mut matcher());
    assert_eq!(results[0].value, "a");
}

#[test]
fn negative_boost_dampens_score() {
    let items = vec![ti("a", "abc"), ti("b", "abcdef")];
    let cfg = config_boost(&[("a", -1000)]);
    let results = score_items(&items, "abc", None, Some(&cfg), &mut matcher());
    assert_eq!(results.last().unwrap().value, "a");
}

#[test]
fn boost_cannot_go_negative() {
    let items = vec![ti("a", "Alpha")];
    let cfg = config_boost(&[("a", -99999)]);
    let results = score_items(&items, "alpha", None, Some(&cfg), &mut matcher());
    assert_eq!(results[0].score, Some(0));
}

struct DoubleScore;
impl ScoringStrategy for DoubleScore {
    fn adjust_score(&self, _value: &str, raw: u32, _query: &str) -> Option<u32> {
        Some(raw * 2)
    }
}

struct FilterLowScores;
impl ScoringStrategy for FilterLowScores {
    fn adjust_score(&self, _value: &str, raw: u32, _query: &str) -> Option<u32> {
        if raw > 50 { Some(raw) } else { None }
    }
}

#[test]
fn scoring_strategy_adjusts_scores() {
    let items = vec![ti("a", "Alpha")];
    let cfg = ScoringConfig {
        hidden_values: HashSet::new(),
        force_mount_values: HashSet::new(),
        boosts: HashMap::new(),
        strategy: Some(std::rc::Rc::new(DoubleScore)),
    };
    let results = score_items(&items, "alpha", None, Some(&cfg), &mut matcher());
    let without = score_items(&items, "alpha", None, None, &mut matcher());
    assert_eq!(results[0].score.unwrap(), without[0].score.unwrap() * 2);
}

#[test]
fn scoring_strategy_can_filter() {
    let items = vec![ti("a", "Alpha"), ti("b", "Alphanumeric")];
    let cfg = ScoringConfig {
        hidden_values: HashSet::new(),
        force_mount_values: HashSet::new(),
        boosts: HashMap::new(),
        strategy: Some(std::rc::Rc::new(FilterLowScores)),
    };
    let results = score_items(&items, "alp", None, Some(&cfg), &mut matcher());
    for r in &results {
        assert!(r.score.unwrap() > 50);
    }
}

#[test]
fn scoring_strategy_skips_force_mount() {
    let items = vec![ti("fm", "Force")];
    let cfg = ScoringConfig {
        hidden_values: HashSet::new(),
        force_mount_values: ["fm".to_string()].into(),
        boosts: HashMap::new(),
        strategy: Some(std::rc::Rc::new(DoubleScore)),
    };
    let results = score_items(&items, "zzz", None, Some(&cfg), &mut matcher());
    assert_eq!(results[0].score, None);
}

// =========================================================================
// Navigation tests (ported from select navigation.rs)
// =========================================================================

#[test]
fn navigate_forward_wraps() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("c"), Direction::Forward, true),
        Some("a".into())
    );
}

#[test]
fn navigate_backward_wraps() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Backward, true),
        Some("c".into())
    );
}

#[test]
fn navigate_skips_disabled() {
    let items = vec![ti("a", "A"), ti_disabled("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Forward, true),
        Some("c".into())
    );
}

#[test]
fn navigate_all_disabled_returns_none() {
    let items = vec![ti_disabled("a", "A"), ti_disabled("b", "B")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Forward, true),
        None
    );
}

#[test]
fn navigate_empty_list_returns_none() {
    let items: Vec<TestItem> = vec![];
    let filtered: Vec<String> = vec![];
    assert_eq!(
        navigate(&items, &filtered, None, Direction::Forward, true),
        None
    );
}

#[test]
fn navigate_no_current_forward_selects_first() {
    let items = vec![ti("a", "A"), ti("b", "B")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(
        navigate(&items, &filtered, None, Direction::Forward, true),
        Some("a".into())
    );
}

#[test]
fn navigate_no_current_backward_selects_last() {
    let items = vec![ti("a", "A"), ti("b", "B")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(
        navigate(&items, &filtered, None, Direction::Backward, true),
        Some("b".into())
    );
}

#[test]
fn navigate_single_element() {
    let items = vec![ti("a", "A")];
    let filtered = vals(&["a"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Forward, true),
        Some("a".into())
    );
}

// ── No-loop navigation (ported from cmdk) ──────────────────────────────

#[test]
fn navigate_no_loop_stops_at_end() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("c"), Direction::Forward, false),
        None
    );
}

#[test]
fn navigate_no_loop_stops_at_start() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Backward, false),
        None
    );
}

#[test]
fn navigate_no_loop_skips_disabled_stops_at_end() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti_disabled("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("b"), Direction::Forward, false),
        None
    );
}

#[test]
fn navigate_no_loop_from_middle() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate(&items, &filtered, Some("a"), Direction::Forward, false),
        Some("b".into())
    );
    assert_eq!(
        navigate(&items, &filtered, Some("c"), Direction::Backward, false),
        Some("b".into())
    );
}

// ── first / last ───────────────────────────────────────────────────────

#[test]
fn first_returns_first_non_disabled() {
    let items = vec![ti_disabled("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(first(&items, &filtered), Some("b".into()));
}

#[test]
fn last_returns_last_non_disabled() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti_disabled("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(last(&items, &filtered), Some("b".into()));
}

#[test]
fn first_empty_returns_none() {
    let items: Vec<TestItem> = vec![];
    let filtered: Vec<String> = vec![];
    assert_eq!(first(&items, &filtered), None);
}

// ── type_ahead ─────────────────────────────────────────────────────────

#[test]
fn type_ahead_finds_match() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana"), ti("c", "Cherry")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(type_ahead(&items, &filtered, None, "b"), Some("b".into()));
}

#[test]
fn type_ahead_case_insensitive() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(type_ahead(&items, &filtered, None, "BAN"), Some("b".into()));
}

#[test]
fn type_ahead_wraps_from_current() {
    let items = vec![ti("a", "Apple"), ti("b", "Avocado")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(
        type_ahead(&items, &filtered, Some("a"), "a"),
        Some("b".into())
    );
}

#[test]
fn type_ahead_no_match_returns_none() {
    let items = vec![ti("a", "Apple"), ti("b", "Banana")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(type_ahead(&items, &filtered, None, "z"), None);
}

#[test]
fn type_ahead_skips_disabled() {
    let items = vec![ti_disabled("a", "Apple"), ti("b", "Apricot")];
    let filtered = vals(&["a", "b"]);
    assert_eq!(type_ahead(&items, &filtered, None, "a"), Some("b".into()));
}

#[test]
fn type_ahead_empty_prefix_returns_none() {
    let items = vec![ti("a", "Apple")];
    let filtered = vals(&["a"]);
    assert_eq!(type_ahead(&items, &filtered, None, ""), None);
}

// ── navigate_by ────────────────────────────────────────────────────────

#[test]
fn navigate_by_multiple_steps() {
    let items = vec![
        ti("a", "A"),
        ti("b", "B"),
        ti("c", "C"),
        ti("d", "D"),
        ti("e", "E"),
    ];
    let filtered = vals(&["a", "b", "c", "d", "e"]);
    assert_eq!(
        navigate_by(&items, &filtered, Some("a"), 3, Direction::Forward, true),
        Some("d".into())
    );
}

#[test]
fn navigate_by_stops_at_boundary_no_loop() {
    let items = vec![ti("a", "A"), ti("b", "B"), ti("c", "C")];
    let filtered = vals(&["a", "b", "c"]);
    assert_eq!(
        navigate_by(&items, &filtered, Some("b"), 5, Direction::Forward, false),
        Some("c".into())
    );
}

#[test]
fn navigate_by_zero_steps_returns_current() {
    let items = vec![ti("a", "A")];
    let filtered = vals(&["a"]);
    assert_eq!(
        navigate_by(&items, &filtered, Some("a"), 0, Direction::Forward, true),
        Some("a".into())
    );
}

// ── ListItem trait for references ──────────────────────────────────────

#[test]
fn list_item_ref_works() {
    let item = ti("a", "Apple");
    let r: &TestItem = &item;
    assert_eq!(r.value(), "a");
    assert_eq!(r.label(), "Apple");

    // Can use with score_items via &T
    let items = [ti("a", "Apple"), ti("b", "Banana")];
    let refs: Vec<&TestItem> = items.iter().collect();
    let results = score_items(&refs, "app", None, None, &mut matcher());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, "a");
}

// ── CustomFilter constructors ──────────────────────────────────────────

#[test]
fn custom_filter_never_equal() {
    let a = CustomFilter::new(|_, _, _| Some(1));
    let b = CustomFilter::new(|_, _, _| Some(1));
    assert_ne!(a, b);
}

#[test]
fn custom_filter_from_label_only_works() {
    let cf = CustomFilter::from_label_only(
        |query, label| {
            if label.contains(query) { Some(1) } else { None }
        },
    );
    assert_eq!((cf.0)("hel", "hello", "ignored keywords"), Some(1));
    assert_eq!((cf.0)("xyz", "hello", ""), None);
}
