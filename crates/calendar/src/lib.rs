pub mod context;
pub mod math;
pub mod types;

mod components;

/// Compound component namespace for single-date calendar.
///
/// ```text
/// use dioxus_nox_calendar::calendar;
///
/// rsx! {
///     calendar::Root {
///         calendar::Header {
///             calendar::PrevButton { "<" }
///             calendar::Title {}
///             calendar::NextButton { ">" }
///         }
///         calendar::Grid {}
///     }
/// }
/// ```
pub mod calendar {
    pub use crate::components::{
        DecadeView, Grid, Header, MonthView, NextButton, PrevButton, Root, SelectMonth,
        SelectYear, Title, YearView,
    };
}

/// Compound component namespace for range-select calendar.
///
/// Uses all the same sub-components as `calendar::`, but with
/// `RangeRoot` instead of `Root`.
///
/// ```text
/// use dioxus_nox_calendar::range_calendar;
///
/// rsx! {
///     range_calendar::Root {
///         range_calendar::Header {
///             range_calendar::PrevButton { "<" }
///             range_calendar::Title {}
///             range_calendar::NextButton { ">" }
///         }
///         range_calendar::Grid {}
///     }
/// }
/// ```
pub mod range_calendar {
    pub use crate::components::{
        DecadeView, Grid, Header, MonthView, NextButton, PrevButton, RangeRoot as Root,
        SelectMonth, SelectYear, Title, YearView,
    };
}

// Re-export key types at crate root for convenience
pub use context::{
    BaseCalendarContext, CalendarFocusContext, GridOptionsContext, MonthViewContext, RangeContext,
    SelectionContext, SingleContext,
};
pub use types::{CellRenderData, DateRange, DateStatus, RelativeMonth, ViewMode, WeekdaySet};

#[cfg(test)]
mod tests;
