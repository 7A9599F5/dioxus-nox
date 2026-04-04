use chrono::NaiveDateTime;

use crate::types::EventPosition;

/// Minimal event interface for layout computation.
/// This avoids depending on the full `ScheduleEvent` trait.
#[derive(Clone, Debug)]
pub struct LayoutEvent {
    pub id: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

/// Compute visual layout positions for events within a single day.
///
/// Uses column-packing to handle overlapping events:
/// 1. Sort events by start time, then by duration (longest first).
/// 2. Assign each event to the leftmost available column.
/// 3. Track column end-times to detect overlaps.
/// 4. Output `EventPosition` with percentage offsets and column info.
///
/// `day_start` and `day_end` define the visible time range.
///
/// This is a **pure function** — no Dioxus dependency — fully testable.
pub fn compute_event_layout(
    events: &[LayoutEvent],
    day_start: NaiveDateTime,
    day_end: NaiveDateTime,
) -> Vec<(String, EventPosition)> {
    if events.is_empty() {
        return Vec::new();
    }

    let total_minutes = (day_end - day_start).num_minutes() as f64;
    if total_minutes <= 0.0 {
        return Vec::new();
    }

    // Sort by start time, then longest duration first for tie-breaking.
    let mut sorted: Vec<_> = events.iter().collect();
    sorted.sort_by(|a, b| {
        a.start.cmp(&b.start).then_with(|| {
            let dur_a = (a.end - a.start).num_minutes();
            let dur_b = (b.end - b.start).num_minutes();
            dur_b.cmp(&dur_a) // longest first
        })
    });

    // Column end-times: track when each column becomes free.
    let mut column_ends: Vec<NaiveDateTime> = Vec::new();
    // Store (event_id, column_index) for each event.
    let mut assignments: Vec<(String, usize)> = Vec::new();

    for event in &sorted {
        // Clamp event to visible range.
        let ev_start = event.start.max(day_start);
        let ev_end = event.end.min(day_end);
        if ev_start >= ev_end {
            continue;
        }

        // Find the leftmost column where this event fits.
        let col = column_ends
            .iter()
            .position(|&end| end <= ev_start)
            .unwrap_or_else(|| {
                column_ends.push(day_start);
                column_ends.len() - 1
            });

        column_ends[col] = ev_end;
        assignments.push((event.id.clone(), col));
    }

    let total_columns = column_ends.len().max(1);

    // Build result with position calculations.
    let mut result = Vec::with_capacity(assignments.len());
    for (id, col) in assignments {
        let event = sorted.iter().find(|e| e.id == id).unwrap();
        let ev_start = event.start.max(day_start);
        let ev_end = event.end.min(day_end);

        let start_offset = (ev_start - day_start).num_minutes() as f64;
        let duration = (ev_end - ev_start).num_minutes() as f64;

        result.push((
            id,
            EventPosition {
                top_percent: (start_offset / total_minutes) * 100.0,
                height_percent: (duration / total_minutes) * 100.0,
                column: col,
                total_columns,
            },
        ));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 4, 4)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    fn day_start() -> NaiveDateTime {
        dt(6, 0)
    }

    fn day_end() -> NaiveDateTime {
        dt(22, 0)
    }

    #[test]
    fn empty_events() {
        let result = compute_event_layout(&[], day_start(), day_end());
        assert!(result.is_empty());
    }

    #[test]
    fn single_event() {
        let events = vec![LayoutEvent {
            id: "e1".into(),
            start: dt(9, 0),
            end: dt(10, 0),
        }];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "e1");
        assert_eq!(result[0].1.column, 0);
        assert_eq!(result[0].1.total_columns, 1);

        // 9:00 is 3 hours after 6:00 = 180 minutes out of 960 total
        let expected_top = (180.0 / 960.0) * 100.0;
        assert!((result[0].1.top_percent - expected_top).abs() < 0.01);

        // 1 hour = 60 minutes out of 960
        let expected_height = (60.0 / 960.0) * 100.0;
        assert!((result[0].1.height_percent - expected_height).abs() < 0.01);
    }

    #[test]
    fn non_overlapping_events() {
        let events = vec![
            LayoutEvent {
                id: "e1".into(),
                start: dt(9, 0),
                end: dt(10, 0),
            },
            LayoutEvent {
                id: "e2".into(),
                start: dt(11, 0),
                end: dt(12, 0),
            },
        ];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 2);
        // Both should be in column 0 since they don't overlap.
        assert_eq!(result[0].1.column, 0);
        assert_eq!(result[1].1.column, 0);
        assert_eq!(result[0].1.total_columns, 1);
    }

    #[test]
    fn overlapping_events() {
        let events = vec![
            LayoutEvent {
                id: "e1".into(),
                start: dt(9, 0),
                end: dt(11, 0),
            },
            LayoutEvent {
                id: "e2".into(),
                start: dt(10, 0),
                end: dt(12, 0),
            },
        ];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 2);
        // They overlap, so they should be in different columns.
        assert_eq!(result[0].1.column, 0);
        assert_eq!(result[1].1.column, 1);
        assert_eq!(result[0].1.total_columns, 2);
        assert_eq!(result[1].1.total_columns, 2);
    }

    #[test]
    fn three_way_overlap() {
        let events = vec![
            LayoutEvent {
                id: "e1".into(),
                start: dt(9, 0),
                end: dt(12, 0),
            },
            LayoutEvent {
                id: "e2".into(),
                start: dt(10, 0),
                end: dt(11, 0),
            },
            LayoutEvent {
                id: "e3".into(),
                start: dt(10, 30),
                end: dt(11, 30),
            },
        ];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].1.total_columns, 3);
        // All should be in different columns.
        let cols: Vec<usize> = result.iter().map(|r| r.1.column).collect();
        assert_eq!(cols[0], 0);
        assert_eq!(cols[1], 1);
        assert_eq!(cols[2], 2);
    }

    #[test]
    fn event_outside_visible_range_is_skipped() {
        let events = vec![LayoutEvent {
            id: "e1".into(),
            start: dt(4, 0),
            end: dt(5, 0),
        }];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert!(result.is_empty());
    }

    #[test]
    fn event_partially_visible_is_clamped() {
        let events = vec![LayoutEvent {
            id: "e1".into(),
            start: dt(5, 0), // before day_start (6:00)
            end: dt(8, 0),
        }];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 1);
        // Should start at 0% (clamped to day_start)
        assert!((result[0].1.top_percent - 0.0).abs() < 0.01);
        // Duration should be 2 hours (6:00 to 8:00)
        let expected_height = (120.0 / 960.0) * 100.0;
        assert!((result[0].1.height_percent - expected_height).abs() < 0.01);
    }

    #[test]
    fn column_reuse_after_gap() {
        let events = vec![
            LayoutEvent {
                id: "e1".into(),
                start: dt(9, 0),
                end: dt(10, 0),
            },
            LayoutEvent {
                id: "e2".into(),
                start: dt(9, 30),
                end: dt(10, 30),
            },
            LayoutEvent {
                id: "e3".into(),
                start: dt(10, 0),
                end: dt(11, 0),
            },
        ];
        let result = compute_event_layout(&events, day_start(), day_end());
        assert_eq!(result.len(), 3);
        // e3 starts at 10:00, e1 ends at 10:00 → e3 can reuse column 0
        assert_eq!(result[0].1.column, 0); // e1
        assert_eq!(result[1].1.column, 1); // e2
        assert_eq!(result[2].1.column, 0); // e3 reuses column 0
    }
}
