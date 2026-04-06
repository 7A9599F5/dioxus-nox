use std::fmt;
use time::{Date, Weekday};

/// Calendar date range (inclusive on both ends).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    start: Date,
    end: Date,
}

impl DateRange {
    /// Create a new date range. If `start > end`, they are swapped.
    pub fn new(start: Date, end: Date) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn start(&self) -> Date {
        self.start
    }

    pub fn end(&self) -> Date {
        self.end
    }

    /// Returns true if the range contains the given date (inclusive).
    pub fn contains(&self, date: Date) -> bool {
        self.start <= date && date <= self.end
    }

    /// Returns true if the date is strictly between start and end (exclusive).
    pub fn contains_exclusive(&self, date: Date) -> bool {
        self.start < date && date < self.end
    }

    /// Clamp a date to within this range.
    pub fn clamp(&self, date: Date) -> Date {
        date.clamp(self.start, self.end)
    }
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} – {}", self.start, self.end)
    }
}

/// Disabled/unavailable status for a single date.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DateStatus {
    /// Non-interactive: keyboard navigation skips, not clickable.
    pub disabled: bool,
    /// Visually marked but still focusable/navigable (e.g. "booked").
    pub unavailable: bool,
}

/// Which month a date belongs to, relative to the currently displayed month.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RelativeMonth {
    Previous,
    Current,
    Next,
}

impl RelativeMonth {
    pub fn is_current(self) -> bool {
        self == Self::Current
    }

    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Previous => "previous",
            Self::Current => "current",
            Self::Next => "next",
        }
    }
}

/// View mode for calendar navigation (month → year → decade drill-down).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Month,
    /// 12-month grid for the current year.
    Year,
    /// ~12-year grid (e.g., 2020-2031).
    Decade,
}

impl ViewMode {
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Year => "year",
            Self::Decade => "decade",
        }
    }
}

/// Computed state for a single calendar cell, passed to custom `render_cell` callbacks.
///
/// All data attributes, ARIA, event handlers, and focus management remain
/// framework-controlled — the callback only replaces the `<button>` children.
#[derive(Clone, Debug)]
pub struct CellRenderData {
    pub date: Date,
    pub day: u8,
    pub is_today: bool,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub is_unavailable: bool,
    pub is_focused: bool,
    pub is_outside_month: bool,
    pub relative_month: RelativeMonth,
    /// `"start"`, `"middle"`, or `"end"` for range mode; `None` for single.
    pub range_position: Option<&'static str>,
}

/// A bitmask collection of weekdays stored as a single byte.
/// Bits 0-6 correspond to Monday-Sunday.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    /// All seven days.
    pub const ALL: Self = Self(0b0111_1111);

    /// Empty set.
    pub const EMPTY: Self = Self(0);

    /// Create a set containing a single weekday.
    pub const fn single(weekday: Weekday) -> Self {
        Self(1 << weekday.number_days_from_monday())
    }

    /// Returns true if the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns true if the set contains the given day.
    pub const fn contains(self, day: Weekday) -> bool {
        self.0 & Self::single(day).0 != 0
    }

    /// Remove a day from the set. Returns true if it was present.
    pub fn remove(&mut self, day: Weekday) -> bool {
        if self.contains(day) {
            self.0 &= !Self::single(day).0;
            true
        } else {
            false
        }
    }

    /// Get the first day in the set (starting from Monday).
    pub const fn first(self) -> Option<Weekday> {
        if self.is_empty() {
            return None;
        }
        Some(Weekday::Monday.nth_next(self.0.trailing_zeros() as u8))
    }

    /// Split at the given day: `(before, at_and_after)`.
    const fn split_at(self, weekday: Weekday) -> (Self, Self) {
        let at_and_after = 0b1000_0000 - Self::single(weekday).0;
        let before = at_and_after ^ 0b0111_1111;
        (Self(self.0 & before), Self(self.0 & at_and_after))
    }

    /// Iterate weekdays in this set, starting from the given day.
    pub const fn iter(self, start: Weekday) -> WeekdaySetIter {
        WeekdaySetIter { days: self, start }
    }
}

/// Iterator over weekdays in a `WeekdaySet`, starting from a given day.
pub struct WeekdaySetIter {
    days: WeekdaySet,
    start: Weekday,
}

impl Iterator for WeekdaySetIter {
    type Item = Weekday;

    fn next(&mut self) -> Option<Self::Item> {
        if self.days.is_empty() {
            return None;
        }
        let (before, after) = self.days.split_at(self.start);
        let bucket = if after.is_empty() { before } else { after };
        let next = bucket.first().expect("set is not empty");
        self.days.remove(next);
        Some(next)
    }
}
