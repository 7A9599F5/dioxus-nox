use crate::context::*;

// ── TimeSegmentKind ────────────────────────────────────────────────

#[test]
fn segment_kind_ranges() {
    assert_eq!(TimeSegmentKind::Hour24.min_value(), 0);
    assert_eq!(TimeSegmentKind::Hour24.max_value(), 23);
    assert_eq!(TimeSegmentKind::Hour12.min_value(), 1);
    assert_eq!(TimeSegmentKind::Hour12.max_value(), 12);
    assert_eq!(TimeSegmentKind::Minute.min_value(), 0);
    assert_eq!(TimeSegmentKind::Minute.max_value(), 59);
    assert_eq!(TimeSegmentKind::Second.min_value(), 0);
    assert_eq!(TimeSegmentKind::Second.max_value(), 59);
}

#[test]
fn segment_kind_max_digits() {
    assert_eq!(TimeSegmentKind::Hour24.max_digits(), 2);
    assert_eq!(TimeSegmentKind::Minute.max_digits(), 2);
    assert_eq!(TimeSegmentKind::Second.max_digits(), 2);
}

#[test]
fn segment_kind_placeholders() {
    assert_eq!(TimeSegmentKind::Hour24.placeholder(), "HH");
    assert_eq!(TimeSegmentKind::Minute.placeholder(), "MM");
    assert_eq!(TimeSegmentKind::Second.placeholder(), "SS");
}

// ── clamp_time_segment ─────────────────────────────────────────────

#[test]
fn clamp_hour24_wraps() {
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour24, -1), 23);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour24, 24), 0);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour24, 15), 15);
}

#[test]
fn clamp_hour12_wraps() {
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour12, 0), 12);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour12, 13), 1);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Hour12, 6), 6);
}

#[test]
fn clamp_minute_wraps() {
    assert_eq!(clamp_time_segment(TimeSegmentKind::Minute, -1), 59);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Minute, 60), 0);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Minute, 30), 30);
}

#[test]
fn clamp_second_wraps() {
    assert_eq!(clamp_time_segment(TimeSegmentKind::Second, -1), 59);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Second, 60), 0);
    assert_eq!(clamp_time_segment(TimeSegmentKind::Second, 45), 45);
}
