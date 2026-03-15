use crate::math::*;
use crate::types::*;

// ── distance ─────────────────────────────────────────────────────────────────

#[test]
fn distance_zero() {
    assert_eq!(distance(0.0, 0.0, 0.0, 0.0), 0.0);
}

#[test]
fn distance_horizontal() {
    assert!((distance(0.0, 0.0, 100.0, 0.0) - 100.0).abs() < f64::EPSILON);
}

#[test]
fn distance_vertical() {
    assert!((distance(0.0, 0.0, 0.0, 50.0) - 50.0).abs() < f64::EPSILON);
}

#[test]
fn distance_diagonal_3_4_5() {
    assert!((distance(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < f64::EPSILON);
}

// ── gesture_angle_degrees ────────────────────────────────────────────────────

#[test]
fn angle_right() {
    assert!((gesture_angle_degrees(100.0, 0.0) - 0.0).abs() < 1.0);
}

#[test]
fn angle_down() {
    assert!((gesture_angle_degrees(0.0, 100.0) - 90.0).abs() < 1.0);
}

#[test]
fn angle_left() {
    assert!((gesture_angle_degrees(-100.0, 0.0) - 180.0).abs() < 1.0);
}

#[test]
fn angle_up() {
    assert!((gesture_angle_degrees(0.0, -100.0) - 270.0).abs() < 1.0);
}

// ── is_horizontal_gesture ────────────────────────────────────────────────────

#[test]
fn horizontal_pure_right() {
    assert!(is_horizontal_gesture(100.0, 0.0, 30.0));
}

#[test]
fn horizontal_pure_left() {
    assert!(is_horizontal_gesture(-100.0, 0.0, 30.0));
}

#[test]
fn vertical_not_horizontal() {
    assert!(!is_horizontal_gesture(0.0, 100.0, 30.0));
}

#[test]
fn diagonal_within_tolerance() {
    // ~20° from horizontal
    assert!(is_horizontal_gesture(100.0, 36.0, 30.0));
}

#[test]
fn diagonal_outside_tolerance() {
    // 45°
    assert!(!is_horizontal_gesture(100.0, 100.0, 30.0));
}

#[test]
fn left_diagonal_within_tolerance() {
    // ~20° from 180°
    assert!(is_horizontal_gesture(-100.0, 36.0, 30.0));
}

// ── velocity ─────────────────────────────────────────────────────────────────

#[test]
fn velocity_zero_time() {
    assert_eq!(velocity(100.0, 0.0), 0.0);
}

#[test]
fn velocity_negative_time() {
    assert_eq!(velocity(100.0, -5.0), 0.0);
}

#[test]
fn velocity_normal() {
    assert!((velocity(100.0, 200.0) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn velocity_negative_distance() {
    // Negative distance (leftward) still returns positive velocity
    assert!((velocity(-100.0, 200.0) - 0.5).abs() < f64::EPSILON);
}

// ── next_swipe_phase ─────────────────────────────────────────────────────────

#[test]
fn commit_by_distance() {
    // offset 50px of 100px action width = 50% > 40% threshold
    assert_eq!(
        next_swipe_phase(-50.0, 100.0, 0.0, 0.40, 0.5),
        SwipeDecision::Commit
    );
}

#[test]
fn springback_below_threshold() {
    // offset 30px of 100px = 30% < 40%
    assert_eq!(
        next_swipe_phase(-30.0, 100.0, 0.0, 0.40, 0.5),
        SwipeDecision::SpringBack
    );
}

#[test]
fn commit_by_velocity() {
    // Small offset but high velocity
    assert_eq!(
        next_swipe_phase(-10.0, 100.0, 0.6, 0.40, 0.5),
        SwipeDecision::Commit
    );
}

#[test]
fn velocity_wrong_direction_no_commit() {
    // Positive offset (swiping right) — velocity shouldn't commit
    assert_eq!(
        next_swipe_phase(10.0, 100.0, 0.6, 0.40, 0.5),
        SwipeDecision::SpringBack
    );
}

#[test]
fn commit_at_exact_threshold() {
    // Exactly at 40%
    assert_eq!(
        next_swipe_phase(-40.0, 100.0, 0.0, 0.40, 0.5),
        SwipeDecision::Commit
    );
}

// ── data attributes ──────────────────────────────────────────────────────────

#[test]
fn swipe_phase_data_attrs() {
    assert_eq!(SwipePhase::Idle.as_data_attr(), "idle");
    assert_eq!(SwipePhase::Dragging.as_data_attr(), "dragging");
    assert_eq!(SwipePhase::Open.as_data_attr(), "open");
    assert_eq!(SwipePhase::Closing.as_data_attr(), "closing");
}

#[test]
fn long_press_phase_data_attrs() {
    assert_eq!(LongPressPhase::Idle.as_data_attr(), "idle");
    assert_eq!(LongPressPhase::Pending.as_data_attr(), "pending");
    assert_eq!(LongPressPhase::Fired.as_data_attr(), "fired");
}
