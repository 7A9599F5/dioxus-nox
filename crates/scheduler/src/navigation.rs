use chrono::{Datelike, Duration, NaiveDate};

/// Navigate to the next period based on view type.
pub fn next_day(date: NaiveDate) -> NaiveDate {
    date + Duration::days(1)
}

/// Navigate to the previous day.
pub fn prev_day(date: NaiveDate) -> NaiveDate {
    date - Duration::days(1)
}

/// Navigate to the next week.
pub fn next_week(date: NaiveDate) -> NaiveDate {
    date + Duration::weeks(1)
}

/// Navigate to the previous week.
pub fn prev_week(date: NaiveDate) -> NaiveDate {
    date - Duration::weeks(1)
}

/// Get the start of the week (Monday) for a given date.
pub fn week_start(date: NaiveDate) -> NaiveDate {
    let days_since_monday = date.weekday().num_days_from_monday();
    date - Duration::days(days_since_monday as i64)
}

/// Get all 7 dates in the week containing the given date.
pub fn week_dates(date: NaiveDate) -> [NaiveDate; 7] {
    let start = week_start(date);
    let mut dates = [start; 7];
    for (i, date) in dates.iter_mut().enumerate().skip(1) {
        *date = start + Duration::days(i as i64);
    }
    dates
}

/// Navigate between time slots in a day view.
///
/// Returns the new (hour, minute) after moving up or down.
/// `slot_minutes` is the slot granularity (e.g., 30 or 60).
pub fn navigate_slot(
    current_hour: u32,
    current_minute: u32,
    direction: SlotDirection,
    slot_minutes: u32,
    day_start_hour: u32,
    day_end_hour: u32,
) -> (u32, u32) {
    let current_total = current_hour * 60 + current_minute;
    let min_total = day_start_hour * 60;
    let max_total = day_end_hour * 60 - slot_minutes;

    let new_total = match direction {
        SlotDirection::Up => current_total.saturating_sub(slot_minutes),
        SlotDirection::Down => current_total.saturating_add(slot_minutes),
    };

    let clamped = new_total.clamp(min_total, max_total);
    (clamped / 60, clamped % 60)
}

/// Direction for slot navigation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlotDirection {
    Up,
    Down,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 4, day).unwrap()
    }

    #[test]
    fn next_prev_day() {
        assert_eq!(next_day(date(4)), date(5));
        assert_eq!(prev_day(date(4)), date(3));
    }

    #[test]
    fn next_prev_week() {
        assert_eq!(next_week(date(4)), date(11));
        assert_eq!(prev_week(date(11)), date(4));
    }

    #[test]
    fn week_start_monday() {
        // April 4, 2026 is a Saturday
        let start = week_start(date(4));
        assert_eq!(start.weekday(), Weekday::Mon);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 3, 30).unwrap());
    }

    #[test]
    fn week_dates_returns_seven_days() {
        let dates = week_dates(date(4));
        assert_eq!(dates.len(), 7);
        assert_eq!(dates[0].weekday(), Weekday::Mon);
        assert_eq!(dates[6].weekday(), Weekday::Sun);
    }

    #[test]
    fn slot_navigation_down() {
        let (h, m) = navigate_slot(9, 0, SlotDirection::Down, 30, 6, 22);
        assert_eq!((h, m), (9, 30));
    }

    #[test]
    fn slot_navigation_up() {
        let (h, m) = navigate_slot(9, 30, SlotDirection::Up, 30, 6, 22);
        assert_eq!((h, m), (9, 0));
    }

    #[test]
    fn slot_navigation_clamps_at_start() {
        let (h, m) = navigate_slot(6, 0, SlotDirection::Up, 30, 6, 22);
        assert_eq!((h, m), (6, 0));
    }

    #[test]
    fn slot_navigation_clamps_at_end() {
        let (h, m) = navigate_slot(21, 30, SlotDirection::Down, 30, 6, 22);
        assert_eq!((h, m), (21, 30));
    }

    #[test]
    fn slot_navigation_hourly() {
        let (h, m) = navigate_slot(10, 0, SlotDirection::Down, 60, 6, 22);
        assert_eq!((h, m), (11, 0));
    }
}
