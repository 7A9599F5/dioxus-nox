use time::{Weekday, macros::date};

use crate::presets::*;
use crate::segment::*;

// ── Segment clamping ───────────────────────────────────────────────

#[test]
fn clamp_month_wraps() {
    assert_eq!(clamp_segment(SegmentKind::Month, 0, 2026, 1), 12);
    assert_eq!(clamp_segment(SegmentKind::Month, 13, 2026, 1), 1);
    assert_eq!(clamp_segment(SegmentKind::Month, 6, 2026, 1), 6);
}

#[test]
fn clamp_day_wraps() {
    // February non-leap: 28 days
    assert_eq!(clamp_segment(SegmentKind::Day, 0, 2026, 2), 28);
    assert_eq!(clamp_segment(SegmentKind::Day, 29, 2026, 2), 1);
    // February leap: 29 days
    assert_eq!(clamp_segment(SegmentKind::Day, 30, 2024, 2), 1);
    assert_eq!(clamp_segment(SegmentKind::Day, 0, 2024, 2), 29);
}

#[test]
fn clamp_year() {
    assert_eq!(clamp_segment(SegmentKind::Year, 0, 2026, 1), 1);
    assert_eq!(clamp_segment(SegmentKind::Year, 10000, 2026, 1), 9999);
    assert_eq!(clamp_segment(SegmentKind::Year, 2026, 2026, 1), 2026);
}

// ── Segment formatting ─────────────────────────────────────────────

#[test]
fn format_segment_padding() {
    assert_eq!(format_segment(SegmentKind::Year, 5), "0005");
    assert_eq!(format_segment(SegmentKind::Year, 2026), "2026");
    assert_eq!(format_segment(SegmentKind::Month, 3), "03");
    assert_eq!(format_segment(SegmentKind::Day, 9), "09");
    assert_eq!(format_segment(SegmentKind::Day, 15), "15");
}

// ── Segment kind ───────────────────────────────────────────────────

#[test]
fn segment_kind_max_digits() {
    assert_eq!(SegmentKind::Year.max_digits(), 4);
    assert_eq!(SegmentKind::Month.max_digits(), 2);
    assert_eq!(SegmentKind::Day.max_digits(), 2);
}

#[test]
fn segment_kind_placeholders() {
    assert_eq!(SegmentKind::Year.placeholder(), "YYYY");
    assert_eq!(SegmentKind::Month.placeholder(), "MM");
    assert_eq!(SegmentKind::Day.placeholder(), "DD");
}

// ── Presets ─────────────────────────────────────────────────────────

#[test]
fn last_n_days_range() {
    let today = date!(2026 - 04 - 05);
    let range = last_n_days(7, today);
    assert_eq!(range.start(), date!(2026 - 03 - 29));
    assert_eq!(range.end(), today);
}

#[test]
fn this_week_sunday_start() {
    // Apr 5, 2026 is Sunday
    let today = date!(2026 - 04 - 05);
    let range = this_week(today, Weekday::Sunday);
    assert_eq!(range.start(), date!(2026 - 04 - 05)); // Sunday
    assert_eq!(range.end(), date!(2026 - 04 - 11)); // Saturday
}

#[test]
fn this_week_monday_start() {
    // Apr 5, 2026 is Sunday
    let today = date!(2026 - 04 - 05);
    let range = this_week(today, Weekday::Monday);
    assert_eq!(range.start(), date!(2026 - 03 - 30)); // Monday
    assert_eq!(range.end(), date!(2026 - 04 - 05)); // Sunday
}

#[test]
fn this_month_range() {
    let today = date!(2026 - 04 - 15);
    let range = this_month(today);
    assert_eq!(range.start(), date!(2026 - 04 - 01));
    assert_eq!(range.end(), date!(2026 - 04 - 30));
}

#[test]
fn this_year_range() {
    let today = date!(2026 - 06 - 15);
    let range = this_year(today);
    assert_eq!(range.start(), date!(2026 - 01 - 01));
    assert_eq!(range.end(), date!(2026 - 12 - 31));
}

#[test]
fn last_month_range() {
    let today = date!(2026 - 04 - 05);
    let range = last_month(today);
    assert_eq!(range.start(), date!(2026 - 03 - 01));
    assert_eq!(range.end(), date!(2026 - 03 - 31));
}

#[test]
fn last_month_january_wraps_to_december() {
    let today = date!(2026 - 01 - 15);
    let range = last_month(today);
    assert_eq!(range.start(), date!(2025 - 12 - 01));
    assert_eq!(range.end(), date!(2025 - 12 - 31));
}

#[test]
fn last_year_range() {
    let today = date!(2026 - 04 - 05);
    let range = last_year(today);
    assert_eq!(range.start(), date!(2025 - 01 - 01));
    assert_eq!(range.end(), date!(2025 - 12 - 31));
}
