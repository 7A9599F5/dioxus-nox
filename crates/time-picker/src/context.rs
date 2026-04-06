use dioxus::prelude::*;
use time::Time;

/// Shared state for the time picker compound component.
#[derive(Clone, Copy)]
pub struct TimePickerContext {
    pub(crate) hour: Signal<Option<u8>>,
    pub(crate) minute: Signal<Option<u8>>,
    pub(crate) second: Signal<Option<u8>>,
    pub(crate) use_12_hour: bool,
    pub(crate) show_seconds: bool,
    pub(crate) disabled: bool,
    pub(crate) read_only: bool,
    pub(crate) on_change: Option<EventHandler<Option<Time>>>,
}

impl TimePickerContext {
    /// Try to build a valid Time from the current segments.
    pub fn try_time(&self) -> Option<Time> {
        let h = (self.hour)()?;
        let m = (self.minute)()?;
        let s = if self.show_seconds {
            (self.second)().unwrap_or(0)
        } else {
            0
        };
        Time::from_hms(h, m, s).ok()
    }

    /// Fire the on_change callback.
    pub(crate) fn notify(&self) {
        if let Some(handler) = &self.on_change {
            handler.call(self.try_time());
        }
    }

    /// Current hour (0-23 or 1-12 depending on mode).
    pub fn hour(&self) -> Option<u8> {
        (self.hour)()
    }

    /// Current minute (0-59).
    pub fn minute(&self) -> Option<u8> {
        (self.minute)()
    }

    /// Current second (0-59).
    pub fn second(&self) -> Option<u8> {
        (self.second)()
    }

    /// Whether 12-hour mode is active.
    pub fn is_12_hour(&self) -> bool {
        self.use_12_hour
    }

    /// Whether seconds are shown.
    pub fn shows_seconds(&self) -> bool {
        self.show_seconds
    }

    /// Whether AM (true) or PM (false) in 12-hour mode.
    pub fn is_am(&self) -> bool {
        (self.hour)().is_none_or(|h| h < 12)
    }

    /// Toggle AM/PM.
    pub fn toggle_period(&self) {
        if let Some(h) = (self.hour)() {
            let new_h = if h < 12 { h + 12 } else { h - 12 };
            let mut hour = self.hour;
            hour.set(Some(new_h));
            self.notify();
        }
    }
}

/// Clamp and wrap a time segment value.
pub fn clamp_time_segment(kind: TimeSegmentKind, value: i32) -> u8 {
    let max = kind.max_value() as i32;
    let min = kind.min_value() as i32;
    if value < min {
        max as u8
    } else if value > max {
        min as u8
    } else {
        value as u8
    }
}

/// Which time segment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeSegmentKind {
    Hour24,
    Hour12,
    Minute,
    Second,
}

impl TimeSegmentKind {
    pub fn min_value(self) -> u8 {
        match self {
            Self::Hour24 => 0,
            Self::Hour12 => 1,
            Self::Minute | Self::Second => 0,
        }
    }

    pub fn max_value(self) -> u8 {
        match self {
            Self::Hour24 => 23,
            Self::Hour12 => 12,
            Self::Minute | Self::Second => 59,
        }
    }

    pub fn max_digits(self) -> usize {
        2
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Hour24 | Self::Hour12 => "HH",
            Self::Minute => "MM",
            Self::Second => "SS",
        }
    }

    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Hour24 | Self::Hour12 => "Hour",
            Self::Minute => "Minute",
            Self::Second => "Second",
        }
    }
}
