// Unit tests for scheduler.
// Layout and navigation tests live in their respective modules.
// This file holds integration-level tests.

use crate::types::*;
use chrono::NaiveDate;

#[test]
fn scheduler_view_data_attrs() {
    assert_eq!(SchedulerView::Day.as_data_attr(), "day");
    assert_eq!(SchedulerView::Week.as_data_attr(), "week");
    assert_eq!(SchedulerView::Agenda.as_data_attr(), "agenda");
}

#[test]
fn time_slot_to_datetime() {
    let slot = TimeSlotData {
        date: NaiveDate::from_ymd_opt(2026, 4, 4).unwrap(),
        hour: 14,
        minute: 30,
    };
    let dt = slot.to_datetime().unwrap();
    assert_eq!(dt.hour(), 14);
    assert_eq!(dt.minute(), 30);
}

use chrono::Timelike;

#[test]
fn time_slot_to_datetime_midnight() {
    let slot = TimeSlotData {
        date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        hour: 0,
        minute: 0,
    };
    let dt = slot.to_datetime().unwrap();
    assert_eq!(dt.hour(), 0);
    assert_eq!(dt.minute(), 0);
}
