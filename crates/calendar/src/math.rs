//! Pure date math for calendar grid generation and navigation.
//!
//! All functions are pure Rust — no Dioxus, no signals, no web-sys.
//! Testable without any framework runtime.

use time::{Date, Month, Weekday, ext::NumericalDuration};

use crate::types::{DateRange, RelativeMonth, WeekdaySet};

// ── Month navigation ────────────────────────────────────────────────

/// Advance to the same day in the next month, clamping the day if needed.
/// Returns `None` only for dates at the `Date` max boundary.
pub fn next_month(date: Date) -> Option<Date> {
    let next = date.month().next();
    let year = date.year() + if next == Month::January { 1 } else { 0 };
    let max_day = days_in_month(next, year);
    Date::from_calendar_date(year, next, date.day().min(max_day)).ok()
}

/// Go back to the same day in the previous month, clamping the day if needed.
/// Returns `None` only for dates at the `Date` min boundary.
pub fn previous_month(date: Date) -> Option<Date> {
    let prev = date.month().previous();
    let year = date.year() + if prev == Month::December { -1 } else { 0 };
    let max_day = days_in_month(prev, year);
    Date::from_calendar_date(year, prev, date.day().min(max_day)).ok()
}

/// Move forward `n` months from the given date, clamping the day.
pub fn nth_month_next(date: Date, n: u8) -> Option<Date> {
    match n {
        0 => Some(date),
        _ => {
            let month = date.month();
            let target = month.nth_next(n);
            let year = date.year() + if month > target { 1 } else { 0 };
            let max_day = days_in_month(target, year);
            Date::from_calendar_date(year, target, date.day().min(max_day)).ok()
        }
    }
}

/// Move backward `n` months from the given date, clamping the day.
pub fn nth_month_previous(date: Date, n: u8) -> Option<Date> {
    match n {
        0 => Some(date),
        _ => {
            let month = date.month();
            let target = month.nth_prev(n);
            let year = date.year() - if month < target { 1 } else { 0 };
            let max_day = days_in_month(target, year);
            Date::from_calendar_date(year, target, date.day().min(max_day)).ok()
        }
    }
}

/// Replace the month of a date, clamping the day.
pub fn replace_month(date: Date, month: Month) -> Date {
    let max_day = days_in_month(month, date.year());
    Date::from_calendar_date(date.year(), month, date.day().min(max_day))
        .expect("valid year + valid month + clamped day")
}

// ── Day helpers ─────────────────────────────────────────────────────

/// Number of days in the given month and year.
pub fn days_in_month(month: Month, year: i32) -> u8 {
    month.length(year)
}

/// First day of the month containing `date`.
pub fn first_of_month(date: Date) -> Date {
    date.replace_day(1).expect("day 1 is always valid")
}

/// Last day of the month containing `date`.
pub fn last_of_month(date: Date) -> Date {
    let max = days_in_month(date.month(), date.year());
    date.replace_day(max).expect("days_in_month is valid")
}

/// Number of days from the first-day-of-week to the weekday of the 1st of the month.
/// Used to compute how many leading cells from the previous month to show.
pub fn leading_days(date: Date, first_day_of_week: Weekday) -> u8 {
    let first = first_of_month(date).weekday();
    let fdow = first_day_of_week.number_days_from_monday() as i8;
    let fom = first.number_days_from_monday() as i8;
    let diff = fom - fdow;
    if diff < 0 {
        (diff + 7) as u8
    } else {
        diff as u8
    }
}

/// Which relative month a date belongs to, given the displayed month.
pub fn relative_month(
    date: Date,
    displayed_month: Month,
    enabled_range: DateRange,
) -> RelativeMonth {
    if date < enabled_range.start() {
        RelativeMonth::Previous
    } else if date > enabled_range.end() {
        RelativeMonth::Next
    } else {
        match date.month().cmp(&displayed_month) {
            std::cmp::Ordering::Less => RelativeMonth::Previous,
            std::cmp::Ordering::Equal => RelativeMonth::Current,
            std::cmp::Ordering::Greater => RelativeMonth::Next,
        }
    }
}

// ── Grid generation ─────────────────────────────────────────────────

/// Generate a flat list of dates for a calendar month grid.
///
/// Returns 28-42 dates covering the visible grid:
/// - Leading days from the previous month (to fill the first row)
/// - All days of the current month
/// - Trailing days from the next month (to complete the last row)
///
/// The grid always has complete 7-day rows.
pub fn month_grid(year: i32, month: Month, first_day_of_week: Weekday) -> Vec<Date> {
    let view_date = Date::from_calendar_date(year, month, 1).expect("valid date");
    let lead = leading_days(view_date, first_day_of_week) as i64;

    let mut grid = Vec::with_capacity(42);

    // Start from the first visible date (may be in previous month)
    let mut date = view_date.saturating_sub(lead.days());

    // Fill leading days from previous month
    for _ in 0..lead {
        grid.push(date);
        date = date.next_day().expect("not at Date::MAX");
    }

    // Fill current month
    let num_days = days_in_month(month, year);
    for day in 1..=num_days {
        date = Date::from_calendar_date(year, month, day).expect("valid");
        grid.push(date);
    }

    // Fill trailing days to complete the final row
    date = date.next_day().unwrap_or(date);
    let remainder = grid.len() % 7;
    if remainder > 0 {
        for _ in 0..(7 - remainder) {
            grid.push(date);
            date = date.next_day().unwrap_or(date);
        }
    }

    grid
}

/// Split a flat grid into rows of 7 days each.
pub fn grid_rows(grid: &[Date]) -> Vec<&[Date]> {
    grid.chunks(7).collect()
}

/// Generate the weekday header labels starting from the configured first day of week.
pub fn weekday_headers(first_day_of_week: Weekday) -> Vec<Weekday> {
    WeekdaySet::ALL.iter(first_day_of_week).collect()
}

// ── Weekday formatting ──────────────────────────────────────────────

/// Two-letter weekday abbreviation.
pub fn weekday_short(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Mo",
        Weekday::Tuesday => "Tu",
        Weekday::Wednesday => "We",
        Weekday::Thursday => "Th",
        Weekday::Friday => "Fr",
        Weekday::Saturday => "Sa",
        Weekday::Sunday => "Su",
    }
}

// ── Navigation ──────────────────────────────────────────────────────

/// Move focus by arrow keys within a calendar grid.
/// Returns the new focused date, or `None` if movement is not possible.
pub fn navigate(focused: Date, key: NavigationKey, enabled_range: DateRange) -> Option<Date> {
    navigate_with(focused, key, enabled_range, |_| false)
}

/// Move focus by arrow keys, skipping dates where `is_disabled` returns true.
///
/// For single-step keys (Left, Right, Up, Down), continues stepping in the
/// same direction until a non-disabled date is found or the range boundary
/// is hit (max 31 steps to cross a full month). For jump keys (ShiftUp,
/// ShiftDown, Home, End), lands on the target — if disabled, does not move.
pub fn navigate_with(
    focused: Date,
    key: NavigationKey,
    enabled_range: DateRange,
    is_disabled: impl Fn(Date) -> bool,
) -> Option<Date> {
    let candidate = step(focused, key);
    let candidate = candidate.filter(|d| enabled_range.contains(*d))?;

    if !is_disabled(candidate) {
        return Some(candidate);
    }

    // For jump keys, don't search further — land or stay
    if matches!(
        key,
        NavigationKey::ShiftUp
            | NavigationKey::ShiftDown
            | NavigationKey::Home
            | NavigationKey::End
    ) {
        return None;
    }

    // For step keys, continue in same direction (max 31 steps)
    let mut current = candidate;
    for _ in 0..31 {
        let next = step(current, key);
        match next {
            Some(d) if enabled_range.contains(d) => {
                if !is_disabled(d) {
                    return Some(d);
                }
                current = d;
            }
            _ => return None,
        }
    }
    None
}

/// Apply a single navigation step.
fn step(focused: Date, key: NavigationKey) -> Option<Date> {
    match key {
        NavigationKey::Left => focused.previous_day(),
        NavigationKey::Right => focused.next_day(),
        NavigationKey::Up => Some(focused.saturating_sub(7.days())),
        NavigationKey::Down => Some(focused.saturating_add(7.days())),
        NavigationKey::ShiftUp => previous_month(focused),
        NavigationKey::ShiftDown => next_month(focused),
        NavigationKey::Home => Some(first_of_month(focused)),
        NavigationKey::End => Some(last_of_month(focused)),
    }
}

/// Keyboard navigation directions for calendar grid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NavigationKey {
    Left,
    Right,
    Up,
    Down,
    ShiftUp,
    ShiftDown,
    Home,
    End,
}

// ── Week numbers ───────────────────────────────────────────────────

/// ISO 8601 week number (1-53) for the given date.
pub fn iso_week_number(date: Date) -> u8 {
    date.iso_week()
}

// ── Range clamping ─────────────────────────────────────────────────

/// Find the contiguous non-disabled range around `anchor`.
///
/// Walks outward from anchor until hitting a disabled date or the
/// `enabled_range` boundary. Used during range selection to prevent
/// keyboard navigation from crossing disabled gaps.
pub fn contiguous_range(
    anchor: Date,
    enabled_range: DateRange,
    is_disabled: impl Fn(Date) -> bool,
) -> DateRange {
    // Walk backward from anchor to find start
    let mut start = anchor;
    while let Some(prev) = start.previous_day() {
        if !enabled_range.contains(prev) || is_disabled(prev) {
            break;
        }
        start = prev;
    }

    // Walk forward from anchor to find end
    let mut end = anchor;
    while let Some(next) = end.next_day() {
        if !enabled_range.contains(next) || is_disabled(next) {
            break;
        }
        end = next;
    }

    DateRange::new(start, end)
}

// ── Year/Decade grid helpers ───────────────────────────────────────

/// All 12 months of the year, in order.
pub fn year_grid() -> Vec<Month> {
    (1..=12u8).map(|m| Month::try_from(m).unwrap()).collect()
}

/// 12 years centered on the current decade (e.g., 2020-2031 for 2026).
pub fn decade_grid(year: i32) -> Vec<i32> {
    let (start, _) = decade_range(year);
    (start..start + 12).collect()
}

/// Decade range: (start, end) where start is the decade floor (e.g., 2020 for 2026).
pub fn decade_range(year: i32) -> (i32, i32) {
    let start = year - year.rem_euclid(10);
    (start, start + 9)
}

// ── ARIA helpers ────────────────────────────────────────────────────

/// Human-readable ARIA label for a date (e.g., "Friday, April 4, 2026").
pub fn aria_date_label(date: &Date) -> String {
    format!(
        "{}, {} {}, {}",
        date.weekday(),
        date.month(),
        date.day(),
        date.year()
    )
}
