use chrono::{NaiveDate, NaiveDateTime};
use dioxus::prelude::*;

/// Trait for event data types used in the scheduler.
///
/// Consumers implement this for their domain types:
/// ```rust,ignore
/// struct Meeting { id: String, title: String, start: NaiveDateTime, end: NaiveDateTime }
///
/// impl ScheduleEvent for Meeting {
///     fn event_id(&self) -> &str { &self.id }
///     fn start(&self) -> NaiveDateTime { self.start }
///     fn end(&self) -> NaiveDateTime { self.end }
/// }
/// ```
pub trait ScheduleEvent: Clone + PartialEq + 'static {
    /// Unique identifier for this event.
    fn event_id(&self) -> &str;
    /// Event start time.
    fn start(&self) -> NaiveDateTime;
    /// Event end time.
    fn end(&self) -> NaiveDateTime;
    /// Whether this is an all-day event.
    fn all_day(&self) -> bool {
        false
    }
}

/// Scheduler view mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SchedulerView {
    /// Single day timeline.
    #[default]
    Day,
    /// 7-day side-by-side view.
    Week,
    /// Flat chronological event list.
    Agenda,
}

impl SchedulerView {
    /// Value for `data-scheduler-view` attribute.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Agenda => "agenda",
        }
    }
}

/// Time slot data for slot click events.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeSlotData {
    /// Date of the slot.
    pub date: NaiveDate,
    /// Hour (0–23).
    pub hour: u32,
    /// Minute (0 or 30 for half-hour slots).
    pub minute: u32,
}

impl TimeSlotData {
    /// Convert to a `NaiveDateTime`.
    pub fn to_datetime(&self) -> Option<NaiveDateTime> {
        self.date.and_hms_opt(self.hour, self.minute, 0)
    }
}

/// Computed position for an event in the time grid.
#[derive(Clone, Debug, PartialEq)]
pub struct EventPosition {
    /// Percentage offset from the day start (for CSS `top`).
    pub top_percent: f64,
    /// Percentage of day height (for CSS `height`).
    pub height_percent: f64,
    /// Overlap column index (0-based).
    pub column: usize,
    /// Total number of overlapping columns.
    pub total_columns: usize,
}

/// Event emitted when an event is drag-resized.
#[derive(Clone, Debug, PartialEq)]
pub struct EventResizeData {
    /// Event identifier.
    pub event_id: String,
    /// New start time.
    pub new_start: NaiveDateTime,
    /// New end time.
    pub new_end: NaiveDateTime,
}

/// Event emitted when an event is drag-dropped to a new time.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDropData {
    /// Event identifier.
    pub event_id: String,
    /// New start time.
    pub new_start: NaiveDateTime,
    /// New end time.
    pub new_end: NaiveDateTime,
}

/// Internal event entry for context tracking.
#[derive(Clone, Debug, PartialEq)]
pub struct EventEntry {
    /// Event identifier.
    pub id: String,
    /// Start time.
    pub start: NaiveDateTime,
    /// End time.
    pub end: NaiveDateTime,
    /// Whether all-day.
    pub all_day: bool,
}

/// Shared context for the scheduler compound component tree.
///
/// Provided by [`super::scheduler::Root`] and consumed by all sub-components.
/// All mutable fields are `Signal` so this is `Copy`.
#[derive(Clone, Copy)]
#[allow(dead_code)] // on_event_resize and on_event_drop reserved for `dnd` feature
pub struct SchedulerContext {
    /// Current view mode.
    pub(crate) view: Signal<SchedulerView>,
    /// Currently displayed date.
    pub(crate) current_date: Signal<NaiveDate>,
    /// Registered events.
    pub(crate) events: Signal<Vec<EventEntry>>,
    /// Currently selected event ID.
    pub(crate) selected_event: Signal<Option<String>>,
    /// Time slot granularity in minutes (30 or 60).
    pub(crate) slot_height_minutes: u32,
    /// First visible hour of the day (e.g., 6).
    pub(crate) day_start_hour: u32,
    /// Last visible hour of the day (e.g., 22).
    pub(crate) day_end_hour: u32,
    /// Callback when an event is clicked.
    pub(crate) on_event_click: Option<EventHandler<String>>,
    /// Callback when a time slot is clicked.
    pub(crate) on_slot_click: Option<EventHandler<TimeSlotData>>,
    /// Callback when an event is resized.
    pub(crate) on_event_resize: Option<EventHandler<EventResizeData>>,
    /// Callback when an event is dropped to a new time.
    pub(crate) on_event_drop: Option<EventHandler<EventDropData>>,
    /// Callback when view mode changes.
    pub(crate) on_view_change: Option<EventHandler<SchedulerView>>,
    /// Callback when the displayed date changes.
    pub(crate) on_date_change: Option<EventHandler<NaiveDate>>,
}

impl SchedulerContext {
    // ── View ────────────────────────────────────────────────────────────

    /// Get the current view mode.
    pub fn view(&self) -> SchedulerView {
        (self.view)()
    }

    /// Set the view mode.
    pub fn set_view(&mut self, view: SchedulerView) {
        self.view.set(view);
        if let Some(handler) = &self.on_view_change {
            handler.call(view);
        }
    }

    // ── Date navigation ─────────────────────────────────────────────────

    /// Get the current date.
    pub fn current_date(&self) -> NaiveDate {
        (self.current_date)()
    }

    /// Navigate to a specific date.
    pub fn go_to_date(&mut self, date: NaiveDate) {
        self.current_date.set(date);
        if let Some(handler) = &self.on_date_change {
            handler.call(date);
        }
    }

    /// Navigate to the next day or week (depending on view).
    pub fn go_next(&mut self) {
        let date = self.current_date();
        let next = match self.view() {
            SchedulerView::Day => date + chrono::Duration::days(1),
            SchedulerView::Week => date + chrono::Duration::weeks(1),
            SchedulerView::Agenda => date + chrono::Duration::weeks(1),
        };
        self.go_to_date(next);
    }

    /// Navigate to the previous day or week (depending on view).
    pub fn go_prev(&mut self) {
        let date = self.current_date();
        let prev = match self.view() {
            SchedulerView::Day => date - chrono::Duration::days(1),
            SchedulerView::Week => date - chrono::Duration::weeks(1),
            SchedulerView::Agenda => date - chrono::Duration::weeks(1),
        };
        self.go_to_date(prev);
    }

    /// Navigate to today.
    pub fn go_today(&mut self) {
        let today = chrono::Local::now().date_naive();
        self.go_to_date(today);
    }

    // ── Event selection ─────────────────────────────────────────────────

    /// Get the currently selected event ID.
    pub fn selected_event(&self) -> Option<String> {
        (self.selected_event)()
    }

    /// Select an event.
    pub fn select_event(&mut self, event_id: &str) {
        self.selected_event.set(Some(event_id.to_string()));
        if let Some(handler) = &self.on_event_click {
            handler.call(event_id.to_string());
        }
    }

    /// Deselect the current event.
    pub fn deselect_event(&mut self) {
        self.selected_event.set(None);
    }

    // ── Event registration ──────────────────────────────────────────────

    /// Register an event.
    pub fn register_event(&mut self, entry: EventEntry) {
        let mut events = self.events.write();
        if !events.iter().any(|e| e.id == entry.id) {
            events.push(entry);
        }
    }

    /// Deregister an event.
    pub fn deregister_event(&mut self, event_id: &str) {
        self.events.write().retain(|e| e.id != event_id);
    }

    // ── Time grid info ──────────────────────────────────────────────────

    /// Number of slots visible in a day.
    pub fn slots_per_day(&self) -> usize {
        let hours = self.day_end_hour.saturating_sub(self.day_start_hour);
        (hours as usize * 60) / self.slot_height_minutes as usize
    }

    /// Whether the given date is today.
    pub fn is_today(&self, date: NaiveDate) -> bool {
        date == chrono::Local::now().date_naive()
    }
}
