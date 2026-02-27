// Pure unit tests — no Dioxus runtime required.

use crate::placement::compute_float_style;
use crate::trigger::{detect_trigger, extract_filter, utf16_to_byte_index};

// ── utf16_to_byte_index ───────────────────────────────────────────────────────

#[test]
fn utf16_index_zero_returns_zero() {
    assert_eq!(utf16_to_byte_index("hello", 0), Some(0));
}

#[test]
fn utf16_index_ascii() {
    assert_eq!(utf16_to_byte_index("hello", 3), Some(3));
}

#[test]
fn utf16_index_end_of_string() {
    assert_eq!(utf16_to_byte_index("hello", 5), Some(5));
}

#[test]
fn utf16_index_out_of_range() {
    assert_eq!(utf16_to_byte_index("hello", 10), None);
}

#[test]
fn utf16_index_two_byte_char() {
    // "é" is U+00E9, 2 UTF-8 bytes, 1 UTF-16 code unit.
    // "éx" → utf16[1] = byte 2 (start of 'x')
    assert_eq!(utf16_to_byte_index("éx", 1), Some(2));
}

#[test]
fn utf16_index_cjk_char() {
    // "中" is U+4E2D: 3 UTF-8 bytes, 1 UTF-16 code unit.
    // "中x" → cursor at utf16[1] = byte 3 (start of 'x')
    assert_eq!(utf16_to_byte_index("中x", 1), Some(3));
}

#[test]
fn utf16_index_emoji_surrogate_pair() {
    // "🙂" is U+1F642: 4 UTF-8 bytes, 2 UTF-16 code units.
    // "🙂x" → cursor at utf16[2] = byte 4 (start of 'x')
    assert_eq!(utf16_to_byte_index("🙂x", 2), Some(4));
}

// ── detect_trigger (line_start_only = true, i.e. slash behaviour) ────────────

#[test]
fn slash_trigger_at_start_of_text() {
    assert_eq!(detect_trigger("/", 1, '/', true), Some(0));
}

#[test]
fn slash_trigger_after_newline() {
    // "hello\n/" — slash at byte 6, cursor at UTF-16 7
    assert_eq!(detect_trigger("hello\n/", 7, '/', true), Some(6));
}

#[test]
fn slash_no_trigger_mid_word() {
    assert_eq!(detect_trigger("hello/world", 6, '/', true), None);
}

#[test]
fn slash_no_trigger_not_at_line_start() {
    assert_eq!(detect_trigger("abc/", 4, '/', true), None);
}

#[test]
fn slash_no_trigger_cursor_zero() {
    assert_eq!(detect_trigger("/", 0, '/', true), None);
}

#[test]
fn slash_trigger_with_filter_text() {
    assert_eq!(detect_trigger("/head", 5, '/', true), Some(0));
}

#[test]
fn slash_trigger_after_newline_with_filter() {
    assert_eq!(detect_trigger("hello\n/hea", 10, '/', true), Some(6));
}

#[test]
fn slash_no_trigger_empty_text() {
    assert_eq!(detect_trigger("", 0, '/', true), None);
}

#[test]
fn slash_trigger_second_line_start() {
    assert_eq!(detect_trigger("a\nb\n/", 5, '/', true), Some(4));
}

#[test]
fn slash_trigger_with_emoji_prefix() {
    // "🙂\n/cmd" — emoji=2 UTF-16 units, newline=1, slash=1 → cursor_utf16=4
    let text = "🙂\n/cmd";
    assert!(detect_trigger(text, 4, '/', true).is_some());
}

#[test]
fn slash_no_trigger_emoji_no_newline() {
    // "🙂/cmd" — slash not at line start
    let text = "🙂/cmd";
    let cursor_utf16 = 3; // emoji(2) + slash(1)
    assert!(detect_trigger(text, cursor_utf16, '/', true).is_none());
}

// ── detect_trigger (line_start_only = false, i.e. mention/@/# behaviour) ─────

#[test]
fn mention_trigger_at_start_of_text() {
    assert_eq!(detect_trigger("@alice", 1, '@', false), Some(0));
}

#[test]
fn mention_trigger_after_space() {
    // "hello @alice" — '@' at byte 6, cursor at UTF-16 7
    assert_eq!(detect_trigger("hello @alice", 7, '@', false), Some(6));
}

#[test]
fn mention_no_trigger_no_at_sign() {
    assert_eq!(detect_trigger("hello world", 11, '@', false), None);
}

#[test]
fn mention_trigger_after_newline() {
    // "hello\n@alice" — '@' at byte 6
    assert_eq!(detect_trigger("hello\n@alice", 7, '@', false), Some(6));
}

#[test]
fn hashtag_trigger_at_start() {
    assert_eq!(detect_trigger("#tag", 1, '#', false), Some(0));
}

#[test]
fn hashtag_trigger_after_space() {
    assert_eq!(detect_trigger("hello #tag", 7, '#', false), Some(6));
}

// ── extract_filter ────────────────────────────────────────────────────────────

#[test]
fn filter_empty_after_slash() {
    assert_eq!(extract_filter("/", 1, '/', true, false, 64), Some(String::new()));
}

#[test]
fn filter_text() {
    assert_eq!(extract_filter("/head", 5, '/', true, false, 64), Some("head".to_string()));
}

#[test]
fn filter_after_newline() {
    assert_eq!(
        extract_filter("hello\n/hea", 10, '/', true, false, 64),
        Some("hea".to_string())
    );
}

#[test]
fn filter_none_mid_word() {
    assert_eq!(extract_filter("abc/def", 4, '/', true, false, 64), None);
}

#[test]
fn filter_none_space_disallowed() {
    assert_eq!(extract_filter("/hello world", 12, '/', true, false, 64), None);
}

#[test]
fn filter_space_allowed() {
    // With allow_spaces = true
    assert_eq!(
        extract_filter("@Full Name", 10, '@', false, true, 64),
        Some("Full Name".to_string())
    );
}

#[test]
fn filter_partial_word() {
    assert_eq!(extract_filter("/he", 3, '/', true, false, 64), Some("he".to_string()));
}

#[test]
fn filter_none_cursor_zero() {
    assert_eq!(extract_filter("/", 0, '/', true, false, 64), None);
}

#[test]
fn filter_exceeds_max_len() {
    // filter "abcde" (5 bytes) with max_filter_len=3 → None
    assert_eq!(extract_filter("/abcde", 6, '/', true, false, 3), None);
}

#[test]
fn filter_exactly_at_max_len() {
    // filter "abc" (3 bytes) with max_filter_len=3 → Some
    assert_eq!(
        extract_filter("/abc", 4, '/', true, false, 3),
        Some("abc".to_string())
    );
}

#[test]
fn filter_mention_after_space() {
    assert_eq!(
        extract_filter("hello @bob", 10, '@', false, false, 64),
        Some("bob".to_string())
    );
}

// ── compute_float_style ───────────────────────────────────────────────────────

#[test]
fn float_style_contains_position_fixed() {
    let s = compute_float_style(10.0, 200.0, 300.0, 4.0, 800.0);
    assert!(s.contains("position:fixed"), "expected position:fixed in {s}");
}

#[test]
fn float_style_top_is_bottom_plus_offset() {
    // anchor_bottom=200, side_offset=4 → top=204
    let s = compute_float_style(10.0, 200.0, 300.0, 4.0, 800.0);
    assert!(s.contains("top:204px"), "expected top:204px in {s}");
}

#[test]
fn float_style_left_equals_anchor_left() {
    let s = compute_float_style(50.0, 200.0, 300.0, 4.0, 800.0);
    assert!(s.contains("left:50px"), "expected left:50px in {s}");
}

#[test]
fn float_style_min_width_equals_anchor_width() {
    let s = compute_float_style(50.0, 200.0, 300.0, 4.0, 800.0);
    assert!(s.contains("min-width:300px"), "expected min-width:300px in {s}");
}

#[test]
fn float_style_zero_offset() {
    // anchor_bottom=100, side_offset=0 → top=100
    let s = compute_float_style(0.0, 100.0, 200.0, 0.0, 600.0);
    assert!(s.contains("top:100px"), "expected top:100px in {s}");
}
