use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;
use time::{Date, Month, Weekday};

use crate::math;
use crate::types::{DateRange, DateStatus};

// ── Instance ID ─────────────────────────────────────────────────────

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ── Base context (config + view state — changes on month nav) ───────

/// Shared calendar configuration and view state.
///
/// Provided by both `calendar::Root` and `calendar::RangeRoot`.
/// Header, Nav, and Title components consume this; cells also read it
/// for `data-today`, `data-disabled`, `data-outside-month`.
#[derive(Clone, Copy)]
pub struct BaseCalendarContext {
    /// Currently displayed month (internal uncontrolled state).
    pub(crate) view_date: Signal<Date>,
    /// Consumer-controlled view date signal (if provided).
    pub(crate) controlled_view: Option<Signal<Date>>,
    /// Whether the entire calendar is disabled.
    pub(crate) disabled: bool,
    /// Which weekday starts the grid.
    pub(crate) first_day_of_week: Weekday,
    /// Lower bound of selectable dates.
    pub(crate) min_date: Date,
    /// Upper bound of selectable dates.
    pub(crate) max_date: Date,
    /// Number of months visible at once.
    pub(crate) month_count: u8,
    /// Today's date (for `data-today`).
    pub(crate) today: Date,
    /// Unique ID for ARIA element ID generation.
    pub(crate) instance_id: u32,
    /// Cached disabled/unavailable status per visible date.
    pub(crate) date_status_cache: Memo<HashMap<Date, DateStatus>>,
    /// Callback when the view month changes.
    pub(crate) on_view_change: Option<EventHandler<Date>>,
    /// Whether the calendar is read-only (visible selections, no interaction).
    pub(crate) read_only: bool,
    /// Custom weekday formatter for i18n. Default: "Mo", "Tu", ...
    pub(crate) format_weekday: Option<Callback<Weekday, String>>,
    /// Custom month formatter for i18n. Default: "January", "February", ...
    pub(crate) format_month: Option<Callback<Month, String>>,
    /// Custom date aria-label formatter for i18n. Default: "Friday, April 4, 2026"
    pub(crate) format_date_label: Option<Callback<Date, String>>,
    /// Current view mode (month/year/decade).
    pub(crate) view_mode: Signal<crate::types::ViewMode>,
}

impl BaseCalendarContext {
    /// The current view date (respects controlled mode).
    pub fn current_view(&self) -> Date {
        match self.controlled_view {
            Some(sig) => (sig)(),
            None => (self.view_date)(),
        }
    }

    /// Set the view date (navigates to a new month).
    pub fn set_view(&mut self, date: Date) {
        let clamped = date.clamp(self.min_date, self.max_date);
        if let Some(mut controlled) = self.controlled_view {
            controlled.set(clamped);
        } else {
            self.view_date.set(clamped);
        }
        if let Some(handler) = &self.on_view_change {
            handler.call(clamped);
        }
    }

    /// Navigate to the previous month.
    pub fn go_prev_month(&mut self) {
        let current = self.current_view();
        if let Some(prev) = math::previous_month(current) {
            self.set_view(prev);
        }
    }

    /// Navigate to the next month.
    pub fn go_next_month(&mut self) {
        let current = self.current_view();
        if let Some(next) = math::next_month(current) {
            self.set_view(next);
        }
    }

    /// Whether the previous-month button should be disabled.
    pub fn is_prev_disabled(&self) -> bool {
        self.disabled || {
            let view = self.current_view();
            math::previous_month(view)
                .is_none_or(|d| d < math::first_of_month(self.min_date))
        }
    }

    /// Whether the next-month button should be disabled.
    pub fn is_next_disabled(&self) -> bool {
        self.disabled || {
            let view = self.current_view();
            math::next_month(view)
                .is_none_or(|d| d > self.max_date)
        }
    }

    /// The enabled date range.
    pub fn enabled_range(&self) -> DateRange {
        DateRange::new(self.min_date, self.max_date)
    }

    /// Check if a date is disabled (non-interactive).
    pub fn is_date_disabled(&self, date: Date) -> bool {
        self.disabled
            || !self.enabled_range().contains(date)
            || self
                .date_status_cache
                .read()
                .get(&date)
                .is_some_and(|s| s.disabled)
    }

    /// Check if a date is unavailable (marked but focusable).
    pub fn is_date_unavailable(&self, date: Date) -> bool {
        self.date_status_cache
            .read()
            .get(&date)
            .is_some_and(|s| s.unavailable)
    }

    /// Whether the calendar is in read-only mode.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Current view mode.
    pub fn view_mode(&self) -> crate::types::ViewMode {
        (self.view_mode)()
    }

    /// Set the view mode (month/year/decade).
    pub fn set_view_mode(&self, mode: crate::types::ViewMode) {
        let mut vm = self.view_mode;
        vm.set(mode);
    }

    /// Format a weekday for display (grid headers, aria). Falls back to English abbreviation.
    pub fn weekday_label(&self, day: Weekday) -> String {
        match &self.format_weekday {
            Some(cb) => cb.call(day),
            None => math::weekday_short(day).to_string(),
        }
    }

    /// Format a month for display (title, select). Falls back to English full name.
    pub fn month_label(&self, month: Month) -> String {
        match &self.format_month {
            Some(cb) => cb.call(month),
            None => format!("{month}"),
        }
    }

    /// Format a date for ARIA label. Falls back to English "Friday, April 4, 2026".
    pub fn date_aria_label(&self, date: &Date) -> String {
        match &self.format_date_label {
            Some(cb) => cb.call(*date),
            None => math::aria_date_label(date),
        }
    }

    /// Instance-scoped element ID.
    pub fn element_id(&self, suffix: &str) -> String {
        format!("nox-cal-{}-{}", self.instance_id, suffix)
    }

    /// Cell-specific element ID.
    pub fn cell_id(&self, date: Date) -> String {
        format!("nox-cal-{}-cell-{}", self.instance_id, date)
    }
}

// ── Month view offset (for multi-month layouts) ────────────────────

/// Per-pane offset context for multi-month views.
///
/// Provided by `calendar::MonthView`. When absent, components default
/// to offset 0 (single-month mode).
#[derive(Clone, Copy)]
pub struct MonthViewContext {
    pub(crate) offset: u8,
}

impl MonthViewContext {
    /// Compute the view date for this pane (base view + offset months).
    pub fn view_date(&self, base: &BaseCalendarContext) -> Date {
        crate::math::nth_month_next(base.current_view(), self.offset)
            .unwrap_or_else(|| base.current_view())
    }
}

// ── Grid options context (render_cell, week numbers) ──────────────

/// Per-grid options passed down to cells via context (avoids prop drilling
/// through `<table>`/`<tr>`).
#[derive(Clone, Copy)]
pub struct GridOptionsContext {
    /// Custom render callback for cell content (replaces button children only).
    pub(crate) render_cell: Option<Callback<crate::types::CellRenderData, Element>>,
}

// ── Focus context (changes on every arrow key) ──────────────────────

/// Keyboard focus state, isolated from base context to prevent header/nav
/// re-renders on every arrow key press.
///
/// Only cells consume this. Header and Nav components do NOT.
#[derive(Clone, Copy)]
pub struct CalendarFocusContext {
    /// The currently keyboard-focused date (if any).
    pub(crate) focused_date: Signal<Option<Date>>,
}

impl CalendarFocusContext {
    pub fn focused(&self) -> Option<Date> {
        (self.focused_date)()
    }

    pub fn set_focused(&mut self, date: Option<Date>) {
        self.focused_date.set(date);
    }

    pub fn is_focused(&self, date: Date) -> bool {
        (self.focused_date)() == Some(date)
    }
}

// ── Selection context (enum to avoid try_use_context bug) ───────────

/// Selection mode. Provided as a single context by Root (Single) or
/// RangeRoot (Range). Cells always `use_context::<SelectionContext>()` —
/// never `try_use_context` — avoiding Dioxus #4509 reactivity bug.
#[derive(Clone, Copy)]
pub enum SelectionContext {
    Single(SingleContext),
    Range(RangeContext),
}

// ── Single-select context ───────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SingleContext {
    pub(crate) selected_date: Signal<Option<Date>>,
    pub(crate) controlled_selected: Option<Signal<Option<Date>>>,
    pub(crate) on_value_change: Option<EventHandler<Option<Date>>>,
}

impl SingleContext {
    pub fn current_selected(&self) -> Option<Date> {
        match self.controlled_selected {
            Some(sig) => (sig)(),
            None => (self.selected_date)(),
        }
    }

    pub fn select(&mut self, date: Date) {
        let current = self.current_selected();
        // Toggle: clicking the selected date deselects it
        let new_val = if current == Some(date) { None } else { Some(date) };

        if let Some(mut controlled) = self.controlled_selected {
            controlled.set(new_val);
        } else {
            self.selected_date.set(new_val);
        }
        if let Some(handler) = &self.on_value_change {
            handler.call(new_val);
        }
    }

    pub fn is_selected(&self, date: Date) -> bool {
        self.current_selected() == Some(date)
    }
}

// ── Range-select context ────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct RangeContext {
    /// The first date clicked (start of range selection).
    pub(crate) anchor_date: Signal<Option<Date>>,
    /// Preview range during hover (before second click commits).
    pub(crate) highlighted_range: Signal<Option<DateRange>>,
    /// Committed range.
    pub(crate) selected_range: Signal<Option<DateRange>>,
    /// Consumer-controlled range signal.
    pub(crate) controlled_range: Option<Signal<Option<DateRange>>>,
    /// Callback when the committed range changes.
    pub(crate) on_range_change: Option<EventHandler<Option<DateRange>>>,
}

impl RangeContext {
    pub fn current_range(&self) -> Option<DateRange> {
        match self.controlled_range {
            Some(sig) => (sig)(),
            None => (self.selected_range)(),
        }
    }

    /// Handle a click on a date during range selection.
    pub fn click_date(&mut self, date: Date) {
        match (self.anchor_date)() {
            None => {
                // First click: set anchor, show single-day preview
                self.anchor_date.set(Some(date));
                self.highlighted_range.set(Some(DateRange::new(date, date)));
            }
            Some(anchor) => {
                // Second click: commit range
                let range = DateRange::new(anchor, date);
                self.anchor_date.set(None);
                self.highlighted_range.set(Some(range));

                if let Some(mut controlled) = self.controlled_range {
                    controlled.set(Some(range));
                } else {
                    self.selected_range.set(Some(range));
                }
                if let Some(handler) = &self.on_range_change {
                    handler.call(Some(range));
                }
            }
        }
    }

    /// Update the preview range on hover (after first click).
    pub fn hover_date(&mut self, date: Date) {
        if let Some(anchor) = (self.anchor_date)() {
            self.highlighted_range.set(Some(DateRange::new(anchor, date)));
        }
    }

    /// Reset range selection (e.g., on Escape).
    pub fn reset(&mut self) {
        self.anchor_date.set(None);
        self.highlighted_range.set(self.current_range());
    }

    /// The active range for rendering (highlighted preview or committed).
    pub fn active_range(&self) -> Option<DateRange> {
        (self.highlighted_range)().or_else(|| self.current_range())
    }

    /// Whether a date is in the active range.
    pub fn is_in_range(&self, date: Date) -> bool {
        self.active_range().is_some_and(|r| r.contains(date))
    }

    /// Position of a date within the active range.
    pub fn range_position(&self, date: Date) -> Option<&'static str> {
        let range = self.active_range()?;
        if date == range.start() {
            Some("start")
        } else if date == range.end() {
            Some("end")
        } else if range.contains_exclusive(date) {
            Some("middle")
        } else {
            None
        }
    }
}
