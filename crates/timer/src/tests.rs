//! Tests for dioxus-nox-timer.

use crate::format::format_duration;
use crate::types::TimerState;

#[test]
fn timer_state_default_is_idle() {
    assert_eq!(TimerState::default(), TimerState::Idle);
}

#[test]
fn timer_state_equality() {
    assert_eq!(TimerState::Running, TimerState::Running);
    assert_ne!(TimerState::Running, TimerState::Paused);
    assert_ne!(TimerState::Idle, TimerState::Complete);
}

#[test]
fn format_duration_zero() {
    assert_eq!(format_duration(0), "0:00");
}

#[test]
fn format_duration_seconds_only() {
    assert_eq!(format_duration(5), "0:05");
    assert_eq!(format_duration(59), "0:59");
}

#[test]
fn format_duration_minutes_and_seconds() {
    assert_eq!(format_duration(60), "1:00");
    assert_eq!(format_duration(65), "1:05");
    assert_eq!(format_duration(125), "2:05");
    assert_eq!(format_duration(599), "9:59");
    assert_eq!(format_duration(3599), "59:59");
}

#[test]
fn format_duration_hours() {
    assert_eq!(format_duration(3600), "1:00:00");
    assert_eq!(format_duration(3661), "1:01:01");
    assert_eq!(format_duration(7200), "2:00:00");
    assert_eq!(format_duration(86399), "23:59:59");
}

#[test]
fn format_duration_negative() {
    assert_eq!(format_duration(-1), "0:00");
    assert_eq!(format_duration(-100), "0:00");
    assert_eq!(format_duration(i64::MIN), "0:00");
}

#[test]
fn format_duration_large_values() {
    // 100 hours
    assert_eq!(format_duration(360000), "100:00:00");
}
