//! Cycle hook implementation.

use dioxus::prelude::*;

use crate::types::CycleState;

/// Hook for cycling through ordered values.
///
/// Wraps around at both ends. Returns a [`CycleState`] with reactive signals
/// and callbacks for next/previous/set_index.
///
/// # Parameters
///
/// - `items`: Ordered list of values to cycle through. Must not be empty.
/// - `initial_index`: Starting index (default 0). Clamped to valid range.
///
/// # Panics
///
/// Panics if `items` is empty.
pub fn use_cycle<T: Clone + PartialEq + 'static>(
    items: &[T],
    initial_index: Option<usize>,
) -> CycleState<T> {
    assert!(!items.is_empty(), "use_cycle: items must not be empty");

    let len = items.len();
    let start = initial_index.unwrap_or(0).min(len - 1);

    // Store items in a signal so closures can capture it (Signal is Copy).
    let stored_items = use_signal(|| items.to_vec());

    let mut index = use_signal(|| start);
    let mut current = use_signal(|| stored_items.read()[start].clone());

    CycleState {
        current,
        index,
        next: Callback::new(move |()| {
            let new_idx = (*index.read() + 1) % len;
            index.set(new_idx);
            current.set(stored_items.read()[new_idx].clone());
        }),
        previous: Callback::new(move |()| {
            let cur = *index.read();
            let new_idx = if cur == 0 { len - 1 } else { cur - 1 };
            index.set(new_idx);
            current.set(stored_items.read()[new_idx].clone());
        }),
        set_index: Callback::new(move |target: usize| {
            let clamped = target.min(len - 1);
            index.set(clamped);
            current.set(stored_items.read()[clamped].clone());
        }),
    }
}
