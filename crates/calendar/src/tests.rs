use time::{Date, Month, Weekday, macros::date};

use crate::math::*;
use crate::types::*;

// ── WeekdaySet ──────────────────────────────────────────────────────

#[test]
fn weekday_set_single_and_contains() {
    let set = WeekdaySet::single(Weekday::Friday);
    assert!(set.contains(Weekday::Friday));
    assert!(!set.contains(Weekday::Monday));
    assert!(!set.contains(Weekday::Sunday));
}

#[test]
fn weekday_set_first() {
    assert_eq!(WeekdaySet::EMPTY.first(), None);
    assert_eq!(
        WeekdaySet::single(Weekday::Friday).first(),
        Some(Weekday::Friday)
    );
    assert_eq!(WeekdaySet::ALL.first(), Some(Weekday::Monday));
}

#[test]
fn weekday_set_remove() {
    let mut set = WeekdaySet::ALL;
    assert!(set.remove(Weekday::Monday));
    assert!(!set.contains(Weekday::Monday));
    assert!(!set.remove(Weekday::Monday)); // already removed
    assert!(set.contains(Weekday::Tuesday)); // others intact
}

#[test]
fn weekday_set_iter_from_sunday() {
    let days: Vec<_> = WeekdaySet::ALL.iter(Weekday::Sunday).collect();
    assert_eq!(days.len(), 7);
    assert_eq!(days[0], Weekday::Sunday);
    assert_eq!(days[1], Weekday::Monday);
    assert_eq!(days[6], Weekday::Saturday);
}

#[test]
fn weekday_set_iter_from_wednesday() {
    let days: Vec<_> = WeekdaySet::ALL.iter(Weekday::Wednesday).collect();
    assert_eq!(days[0], Weekday::Wednesday);
    assert_eq!(days[1], Weekday::Thursday);
    assert_eq!(days[6], Weekday::Tuesday);
}

// ── DateRange ───────────────────────────────────────────────────────

#[test]
fn date_range_normalizes_order() {
    let a = date!(2026 - 01 - 15);
    let b = date!(2026 - 01 - 10);
    let range = DateRange::new(a, b);
    assert_eq!(range.start(), b);
    assert_eq!(range.end(), a);
}

#[test]
fn date_range_contains() {
    let range = DateRange::new(date!(2026 - 03 - 01), date!(2026 - 03 - 31));
    assert!(range.contains(date!(2026 - 03 - 01)));
    assert!(range.contains(date!(2026 - 03 - 15)));
    assert!(range.contains(date!(2026 - 03 - 31)));
    assert!(!range.contains(date!(2026 - 02 - 28)));
    assert!(!range.contains(date!(2026 - 04 - 01)));
}

#[test]
fn date_range_contains_exclusive() {
    let range = DateRange::new(date!(2026 - 03 - 01), date!(2026 - 03 - 31));
    assert!(!range.contains_exclusive(date!(2026 - 03 - 01))); // boundary
    assert!(range.contains_exclusive(date!(2026 - 03 - 15)));
    assert!(!range.contains_exclusive(date!(2026 - 03 - 31))); // boundary
}

#[test]
fn date_range_clamp() {
    let range = DateRange::new(date!(2026 - 03 - 10), date!(2026 - 03 - 20));
    assert_eq!(range.clamp(date!(2026 - 03 - 05)), date!(2026 - 03 - 10));
    assert_eq!(range.clamp(date!(2026 - 03 - 15)), date!(2026 - 03 - 15));
    assert_eq!(range.clamp(date!(2026 - 03 - 25)), date!(2026 - 03 - 20));
}

// ── Month navigation ────────────────────────────────────────────────

#[test]
fn next_month_basic() {
    let jan = date!(2026 - 01 - 15);
    let feb = next_month(jan).unwrap();
    assert_eq!(feb.month(), Month::February);
    assert_eq!(feb.year(), 2026);
    assert_eq!(feb.day(), 15);
}

#[test]
fn next_month_year_wrap() {
    let dec = date!(2026 - 12 - 10);
    let jan = next_month(dec).unwrap();
    assert_eq!(jan.month(), Month::January);
    assert_eq!(jan.year(), 2027);
}

#[test]
fn next_month_clamps_day() {
    // Jan 31 -> Feb should clamp to Feb 28 (non-leap year)
    let jan31 = date!(2026 - 01 - 31);
    let feb = next_month(jan31).unwrap();
    assert_eq!(feb, date!(2026 - 02 - 28));
}

#[test]
fn next_month_clamps_leap() {
    // Jan 31 -> Feb in leap year should clamp to Feb 29
    let jan31 = date!(2024 - 01 - 31);
    let feb = next_month(jan31).unwrap();
    assert_eq!(feb, date!(2024 - 02 - 29));
}

#[test]
fn previous_month_basic() {
    let mar = date!(2026 - 03 - 15);
    let feb = previous_month(mar).unwrap();
    assert_eq!(feb.month(), Month::February);
    assert_eq!(feb.day(), 15);
}

#[test]
fn previous_month_year_wrap() {
    let jan = date!(2026 - 01 - 10);
    let dec = previous_month(jan).unwrap();
    assert_eq!(dec.month(), Month::December);
    assert_eq!(dec.year(), 2025);
}

#[test]
fn previous_month_clamps_day() {
    // Mar 31 -> Feb should clamp
    let mar31 = date!(2026 - 03 - 31);
    let feb = previous_month(mar31).unwrap();
    assert_eq!(feb, date!(2026 - 02 - 28));
}

#[test]
fn nth_month_next_basic() {
    let jan = date!(2026 - 01 - 15);
    assert_eq!(nth_month_next(jan, 0), Some(jan));
    assert_eq!(nth_month_next(jan, 3).unwrap().month(), Month::April);
}

#[test]
fn nth_month_previous_basic() {
    let apr = date!(2026 - 04 - 15);
    assert_eq!(nth_month_previous(apr, 0), Some(apr));
    assert_eq!(nth_month_previous(apr, 3).unwrap().month(), Month::January);
}

// ── Day helpers ─────────────────────────────────────────────────────

#[test]
fn days_in_month_values() {
    assert_eq!(days_in_month(Month::February, 2024), 29); // leap
    assert_eq!(days_in_month(Month::February, 2026), 28); // non-leap
    assert_eq!(days_in_month(Month::January, 2026), 31);
    assert_eq!(days_in_month(Month::April, 2026), 30);
}

#[test]
fn first_and_last_of_month() {
    let mid = date!(2026 - 04 - 15);
    assert_eq!(first_of_month(mid), date!(2026 - 04 - 01));
    assert_eq!(last_of_month(mid), date!(2026 - 04 - 30));
}

#[test]
fn leading_days_sunday_start() {
    // April 2026: 1st is Wednesday. With Sunday start: Sun=0, Mon=1, Tue=2, Wed=3 -> 3 leading days.
    let apr = date!(2026 - 04 - 01);
    assert_eq!(leading_days(apr, Weekday::Sunday), 3);
}

#[test]
fn leading_days_monday_start() {
    // April 2026: 1st is Wednesday. With Monday start: Mon=0, Tue=1, Wed=2 -> 2 leading days.
    let apr = date!(2026 - 04 - 01);
    assert_eq!(leading_days(apr, Weekday::Monday), 2);
}

#[test]
fn leading_days_starts_on_first_day() {
    // February 2021: 1st is Monday. With Monday start: 0 leading days.
    let feb = date!(2021 - 02 - 01);
    assert_eq!(leading_days(feb, Weekday::Monday), 0);
}

// ── Grid generation ─────────────────────────────────────────────────

#[test]
fn month_grid_april_2026_sunday_start() {
    let grid = month_grid(2026, Month::April, Weekday::Sunday);
    // April 2026: 30 days, 1st is Wednesday
    // Sunday start: 3 leading days (Mar 29, 30, 31)
    // 3 + 30 = 33 days, remainder 33 % 7 = 5, trailing = 2
    // Total: 35 dates = 5 rows
    assert_eq!(grid.len(), 35);
    assert_eq!(grid.len() % 7, 0);

    // First visible date should be Sunday Mar 29
    assert_eq!(grid[0], date!(2026 - 03 - 29));
    assert_eq!(grid[0].weekday(), Weekday::Sunday);

    // April 1 should be at index 3
    assert_eq!(grid[3], date!(2026 - 04 - 01));

    // Last April day (30th) at index 3 + 29 = 32
    assert_eq!(grid[32], date!(2026 - 04 - 30));

    // Trailing: May 1, May 2
    assert_eq!(grid[33], date!(2026 - 05 - 01));
    assert_eq!(grid[34], date!(2026 - 05 - 02));
}

#[test]
fn month_grid_february_2021_monday_start() {
    // Feb 2021: 28 days, 1st is Monday. With Monday start: 0 leading days.
    // 28 days, 28 % 7 = 0, no trailing. Exactly 4 rows.
    let grid = month_grid(2021, Month::February, Weekday::Monday);
    assert_eq!(grid.len(), 28);
    assert_eq!(grid[0], date!(2021 - 02 - 01));
    assert_eq!(grid[27], date!(2021 - 02 - 28));
}

#[test]
fn month_grid_may_2024_sunday_start() {
    // May 2024: 31 days, 1st is Wednesday.
    // Sunday start: 3 leading days.
    // 3 + 31 = 34, remainder 34 % 7 = 6, trailing = 1. Total = 35.
    let grid = month_grid(2024, Month::May, Weekday::Sunday);
    assert_eq!(grid.len(), 35);
    assert_eq!(grid.len() % 7, 0);

    let may_days: Vec<_> = grid
        .iter()
        .filter(|d| d.month() == Month::May && d.year() == 2024)
        .collect();
    assert_eq!(may_days.len(), 31);
}

#[test]
fn month_grid_december_2018_sunday_start() {
    // Dec 2018: 31 days, 1st is Saturday.
    // Sunday start: 6 leading days.
    // 6 + 31 = 37, remainder 37 % 7 = 2, trailing = 5. Total = 42.
    let grid = month_grid(2018, Month::December, Weekday::Sunday);
    assert_eq!(grid.len(), 42);

    // All rows should have 7 days
    let rows = grid_rows(&grid);
    assert_eq!(rows.len(), 6);
    for row in &rows {
        assert_eq!(row.len(), 7);
    }
}

#[test]
fn month_grid_every_row_is_7_days() {
    // Test several months to ensure all grids have complete rows.
    for year in [2020, 2024, 2026] {
        for month_num in 1..=12u8 {
            let month = Month::try_from(month_num).unwrap();
            for &fdow in &[Weekday::Sunday, Weekday::Monday] {
                let grid = month_grid(year, month, fdow);
                assert_eq!(
                    grid.len() % 7,
                    0,
                    "Incomplete row for {year}-{month_num:02} starting on {fdow:?}"
                );
                assert!(
                    grid.len() >= 28 && grid.len() <= 42,
                    "Grid size {} out of range for {year}-{month_num:02}",
                    grid.len()
                );
            }
        }
    }
}

// ── Weekday headers ─────────────────────────────────────────────────

#[test]
fn weekday_headers_sunday_start() {
    let headers = weekday_headers(Weekday::Sunday);
    assert_eq!(headers.len(), 7);
    assert_eq!(headers[0], Weekday::Sunday);
    assert_eq!(headers[6], Weekday::Saturday);
}

#[test]
fn weekday_headers_monday_start() {
    let headers = weekday_headers(Weekday::Monday);
    assert_eq!(headers[0], Weekday::Monday);
    assert_eq!(headers[6], Weekday::Sunday);
}

#[test]
fn weekday_short_labels() {
    assert_eq!(weekday_short(Weekday::Monday), "Mo");
    assert_eq!(weekday_short(Weekday::Sunday), "Su");
}

// ── Navigation ──────────────────────────────────────────────────────

#[test]
fn navigate_left_right() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    let focused = date!(2026 - 04 - 15);

    assert_eq!(
        navigate(focused, NavigationKey::Left, range),
        Some(date!(2026 - 04 - 14))
    );
    assert_eq!(
        navigate(focused, NavigationKey::Right, range),
        Some(date!(2026 - 04 - 16))
    );
}

#[test]
fn navigate_up_down_by_week() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    let focused = date!(2026 - 04 - 15);

    assert_eq!(
        navigate(focused, NavigationKey::Up, range),
        Some(date!(2026 - 04 - 08))
    );
    assert_eq!(
        navigate(focused, NavigationKey::Down, range),
        Some(date!(2026 - 04 - 22))
    );
}

#[test]
fn navigate_shift_up_down_by_month() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    let focused = date!(2026 - 04 - 15);

    assert_eq!(
        navigate(focused, NavigationKey::ShiftUp, range),
        Some(date!(2026 - 03 - 15))
    );
    assert_eq!(
        navigate(focused, NavigationKey::ShiftDown, range),
        Some(date!(2026 - 05 - 15))
    );
}

#[test]
fn navigate_home_end() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    let focused = date!(2026 - 04 - 15);

    assert_eq!(
        navigate(focused, NavigationKey::Home, range),
        Some(date!(2026 - 04 - 01))
    );
    assert_eq!(
        navigate(focused, NavigationKey::End, range),
        Some(date!(2026 - 04 - 30))
    );
}

#[test]
fn navigate_clamped_to_enabled_range() {
    let range = DateRange::new(date!(2026 - 04 - 10), date!(2026 - 04 - 20));
    let focused = date!(2026 - 04 - 10);

    // Left would go to Apr 9, which is outside the enabled range
    assert_eq!(navigate(focused, NavigationKey::Left, range), None);

    // Up would go to Apr 3, also outside
    assert_eq!(navigate(focused, NavigationKey::Up, range), None);

    let end = date!(2026 - 04 - 20);
    assert_eq!(navigate(end, NavigationKey::Right, range), None);
}

// ── ARIA ────────────────────────────────────────────────────────────

#[test]
fn aria_label_format() {
    let date = date!(2026 - 04 - 04);
    let label = aria_date_label(&date);
    assert!(label.contains("Saturday"));
    assert!(label.contains("April"));
    assert!(label.contains("4"));
    assert!(label.contains("2026"));
}

// ── RelativeMonth ───────────────────────────────────────────────────

#[test]
fn relative_month_current() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    assert_eq!(
        relative_month(date!(2026 - 04 - 15), Month::April, range),
        RelativeMonth::Current
    );
}

#[test]
fn relative_month_previous_and_next() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    assert_eq!(
        relative_month(date!(2026 - 03 - 31), Month::April, range),
        RelativeMonth::Previous
    );
    assert_eq!(
        relative_month(date!(2026 - 05 - 01), Month::April, range),
        RelativeMonth::Next
    );
}

#[test]
fn relative_month_data_attr() {
    assert_eq!(RelativeMonth::Previous.as_data_attr(), "previous");
    assert_eq!(RelativeMonth::Current.as_data_attr(), "current");
    assert_eq!(RelativeMonth::Next.as_data_attr(), "next");
}

// ── navigate_with (disabled-date skip) ──────────────────────────────

#[test]
fn navigate_with_skips_disabled_right() {
    let range = DateRange::new(date!(2026 - 04 - 01), date!(2026 - 04 - 30));
    // Apr 16 and 17 are disabled; pressing Right from 15 should land on 18
    let disabled = |d: Date| d == date!(2026 - 04 - 16) || d == date!(2026 - 04 - 17);
    assert_eq!(
        navigate_with(date!(2026 - 04 - 15), NavigationKey::Right, range, disabled),
        Some(date!(2026 - 04 - 18))
    );
}

#[test]
fn navigate_with_skips_disabled_left() {
    let range = DateRange::new(date!(2026 - 04 - 01), date!(2026 - 04 - 30));
    // Apr 13 and 14 disabled; pressing Left from 15 should land on 12
    let disabled = |d: Date| d == date!(2026 - 04 - 13) || d == date!(2026 - 04 - 14);
    assert_eq!(
        navigate_with(date!(2026 - 04 - 15), NavigationKey::Left, range, disabled),
        Some(date!(2026 - 04 - 12))
    );
}

#[test]
fn navigate_with_skips_disabled_down() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    // Apr 22 disabled; pressing Down from 15 should skip to Apr 29
    let disabled = |d: Date| d == date!(2026 - 04 - 22);
    assert_eq!(
        navigate_with(date!(2026 - 04 - 15), NavigationKey::Down, range, disabled),
        Some(date!(2026 - 04 - 29))
    );
}

#[test]
fn navigate_with_all_disabled_returns_none() {
    let range = DateRange::new(date!(2026 - 04 - 10), date!(2026 - 04 - 20));
    // Everything after 15 is disabled; pressing Right should return None
    let disabled = |d: Date| d > date!(2026 - 04 - 15);
    assert_eq!(
        navigate_with(date!(2026 - 04 - 15), NavigationKey::Right, range, disabled),
        None
    );
}

#[test]
fn navigate_with_jump_key_doesnt_search() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    // Home target (Apr 1) is disabled — should return None, not search further
    let disabled = |d: Date| d == date!(2026 - 04 - 01);
    assert_eq!(
        navigate_with(date!(2026 - 04 - 15), NavigationKey::Home, range, disabled),
        None
    );
}

#[test]
fn navigate_with_no_disabled_behaves_like_navigate() {
    let range = DateRange::new(date!(2026 - 01 - 01), date!(2026 - 12 - 31));
    let focused = date!(2026 - 04 - 15);
    let no_disabled = |_: Date| false;
    assert_eq!(
        navigate_with(focused, NavigationKey::Right, range, no_disabled),
        navigate(focused, NavigationKey::Right, range)
    );
    assert_eq!(
        navigate_with(focused, NavigationKey::Up, range, no_disabled),
        navigate(focused, NavigationKey::Up, range)
    );
}

// ── ISO week numbers ───────────────────────────────────────────────

#[test]
fn iso_week_number_known_dates() {
    // 2026-01-01 is Thursday → ISO week 1
    assert_eq!(iso_week_number(date!(2026 - 01 - 01)), 1);
    // 2025-12-29 is Monday → ISO week 1 of 2026
    assert_eq!(iso_week_number(date!(2025 - 12 - 29)), 1);
    // 2026-06-15 is Monday → ISO week 25
    assert_eq!(iso_week_number(date!(2026 - 06 - 15)), 25);
}

// ── contiguous_range ───────────────────────────────────────────────

#[test]
fn contiguous_range_no_disabled() {
    let range = DateRange::new(date!(2026 - 04 - 01), date!(2026 - 04 - 30));
    let result = contiguous_range(date!(2026 - 04 - 15), range, |_| false);
    assert_eq!(result.start(), date!(2026 - 04 - 01));
    assert_eq!(result.end(), date!(2026 - 04 - 30));
}

#[test]
fn contiguous_range_with_gap() {
    let range = DateRange::new(date!(2026 - 04 - 01), date!(2026 - 04 - 30));
    // Disabled dates on Apr 10 and Apr 20 create a contiguous zone of Apr 11-19
    let disabled = |d: Date| d == date!(2026 - 04 - 10) || d == date!(2026 - 04 - 20);
    let result = contiguous_range(date!(2026 - 04 - 15), range, disabled);
    assert_eq!(result.start(), date!(2026 - 04 - 11));
    assert_eq!(result.end(), date!(2026 - 04 - 19));
}

#[test]
fn contiguous_range_anchor_at_boundary() {
    let range = DateRange::new(date!(2026 - 04 - 01), date!(2026 - 04 - 30));
    let disabled = |d: Date| d == date!(2026 - 04 - 05);
    // Anchor at start of range — contiguous zone is Apr 1-4
    let result = contiguous_range(date!(2026 - 04 - 01), range, disabled);
    assert_eq!(result.start(), date!(2026 - 04 - 01));
    assert_eq!(result.end(), date!(2026 - 04 - 04));
}

// ── Year/Decade grid helpers ───────────────────────────────────────

#[test]
fn year_grid_has_12_months() {
    let months = year_grid();
    assert_eq!(months.len(), 12);
    assert_eq!(months[0], Month::January);
    assert_eq!(months[11], Month::December);
}

#[test]
fn decade_grid_has_12_years() {
    let years = decade_grid(2026);
    assert_eq!(years.len(), 12);
    assert_eq!(years[0], 2020);
    assert_eq!(years[11], 2031);
}

#[test]
fn decade_range_values() {
    assert_eq!(decade_range(2026), (2020, 2029));
    assert_eq!(decade_range(2020), (2020, 2029));
    assert_eq!(decade_range(2000), (2000, 2009));
}

// ── ViewMode ───────────────────────────────────────────────────────

#[test]
fn view_mode_data_attrs() {
    use crate::types::ViewMode;
    assert_eq!(ViewMode::Month.as_data_attr(), "month");
    assert_eq!(ViewMode::Year.as_data_attr(), "year");
    assert_eq!(ViewMode::Decade.as_data_attr(), "decade");
}

// ── CellRenderData construction ────────────────────────────────────

#[test]
fn cell_render_data_fields() {
    use crate::types::CellRenderData;
    let data = CellRenderData {
        date: date!(2026 - 04 - 15),
        day: 15,
        is_today: true,
        is_selected: false,
        is_disabled: false,
        is_unavailable: false,
        is_focused: true,
        is_outside_month: false,
        relative_month: RelativeMonth::Current,
        range_position: Some("start"),
    };
    assert_eq!(data.day, 15);
    assert!(data.is_today);
    assert!(!data.is_selected);
    assert_eq!(data.range_position, Some("start"));
}
