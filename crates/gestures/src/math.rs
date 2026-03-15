/// Result of the swipe commit/springback decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDecision {
    /// Swipe past threshold — commit the action.
    Commit,
    /// Below threshold — spring back to idle.
    SpringBack,
}

/// Euclidean distance between two points.
#[inline]
pub fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Angle in degrees from origin to (dx, dy). 0 = right, 90 = down.
/// Returns a value in the 0..360 range.
#[inline]
pub fn gesture_angle_degrees(dx: f64, dy: f64) -> f64 {
    let angle = dy.atan2(dx).to_degrees();
    if angle < 0.0 {
        angle + 360.0
    } else {
        angle
    }
}

/// Returns `true` when the gesture vector (dx, dy) is primarily horizontal,
/// meaning the angle is within `tolerance_deg` of 0° (right) or 180° (left).
#[inline]
pub fn is_horizontal_gesture(dx: f64, dy: f64, tolerance_deg: f64) -> bool {
    let angle = gesture_angle_degrees(dx, dy);
    // Near 0° (right)
    if angle <= tolerance_deg || angle >= 360.0 - tolerance_deg {
        return true;
    }
    // Near 180° (left)
    (angle - 180.0).abs() <= tolerance_deg
}

/// Velocity in pixels per millisecond. Returns 0.0 if `elapsed_ms <= 0`.
#[inline]
pub fn velocity(distance_px: f64, elapsed_ms: f64) -> f64 {
    if elapsed_ms <= 0.0 {
        0.0
    } else {
        distance_px.abs() / elapsed_ms
    }
}

/// Determine whether a swipe should commit or spring back.
///
/// Commits if:
/// - `|offset_px| >= item_width * commit_ratio`, OR
/// - `velocity_px_per_ms >= velocity_threshold` and offset is in the commit direction (negative)
#[inline]
pub fn next_swipe_phase(
    offset_px: f64,
    item_width: f64,
    velocity_px_per_ms: f64,
    commit_ratio: f64,
    velocity_threshold: f64,
) -> SwipeDecision {
    let ratio = offset_px.abs() / item_width;
    if ratio >= commit_ratio {
        return SwipeDecision::Commit;
    }
    if velocity_px_per_ms >= velocity_threshold && offset_px < 0.0 {
        return SwipeDecision::Commit;
    }
    SwipeDecision::SpringBack
}
