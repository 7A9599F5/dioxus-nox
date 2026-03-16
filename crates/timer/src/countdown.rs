//! Countdown timer hook.

use dioxus::prelude::*;

use crate::time;
use crate::types::{CountdownControls, TimerState};

/// Countdown timer hook.
///
/// Returns `(remaining_seconds, state, controls)`.
///
/// Uses wall-clock calculation internally for accuracy across tab backgrounding.
/// The timer ticks approximately every 100ms for smooth UI updates.
///
/// # Parameters
///
/// - `on_complete`: Optional callback fired once when the countdown reaches zero.
///
/// # Example
///
/// ```rust,ignore
/// let (remaining, state, controls) = use_countdown(None);
/// controls.start.call(90); // Start a 90-second countdown
/// ```
pub fn use_countdown(
    on_complete: Option<Callback<()>>,
) -> (Signal<i64>, Signal<TimerState>, CountdownControls) {
    // Wall-clock end time (epoch ms). None when idle or paused.
    let mut end_time_ms = use_signal(|| None::<i64>);
    // Remaining ms when paused (so we can resume accurately).
    let mut paused_remaining_ms = use_signal(|| None::<i64>);
    // Current state.
    let mut state = use_signal(|| TimerState::Idle);
    // Remaining seconds (derived, updated by tick loop).
    let mut remaining = use_signal(|| 0i64);
    // Generation counter to cancel stale tick loops.
    let mut generation = use_signal(|| 0u32);

    // Tick loop effect: runs whenever generation changes.
    let gen_value = *generation.read();
    use_effect(move || {
        let current_gen = gen_value;
        spawn(async move {
            loop {
                time::sleep_ms(100).await;
                // Stop if generation changed (a new start/skip/dismiss happened).
                if *generation.read() != current_gen {
                    break;
                }
                let current_state = *state.read();
                if current_state != TimerState::Running {
                    if current_state == TimerState::Idle || current_state == TimerState::Complete {
                        break;
                    }
                    // Paused — keep looping but don't update.
                    continue;
                }
                let end_val = *end_time_ms.read();
                if let Some(end) = end_val {
                    let now = time::now_ms();
                    let remain_ms = (end - now).max(0);
                    let remain_secs = (remain_ms + 999) / 1000; // Ceiling division
                    remaining.set(remain_secs);

                    if remain_ms <= 0 {
                        state.set(TimerState::Complete);
                        remaining.set(0);
                        end_time_ms.set(None);
                        if let Some(ref cb) = on_complete {
                            cb.call(());
                        }
                        break;
                    }
                }
            }
        });
    });

    let controls = CountdownControls {
        start: Callback::new(move |duration_secs: i64| {
            let now = time::now_ms();
            end_time_ms.set(Some(now + duration_secs * 1000));
            paused_remaining_ms.set(None);
            remaining.set(duration_secs);
            state.set(TimerState::Running);
            let next_gen = generation.read().wrapping_add(1);
            generation.set(next_gen);
        }),
        pause: Callback::new(move |()| {
            if *state.read() == TimerState::Running {
                let end = *end_time_ms.read();
                if let Some(end) = end {
                    let remain = (end - time::now_ms()).max(0);
                    paused_remaining_ms.set(Some(remain));
                    end_time_ms.set(None);
                    state.set(TimerState::Paused);
                }
            }
        }),
        resume: Callback::new(move |()| {
            if *state.read() == TimerState::Paused {
                let remain = *paused_remaining_ms.read();
                if let Some(remain) = remain {
                    let now = time::now_ms();
                    end_time_ms.set(Some(now + remain));
                    paused_remaining_ms.set(None);
                    state.set(TimerState::Running);
                    let next_gen = generation.read().wrapping_add(1);
                    generation.set(next_gen);
                }
            }
        }),
        skip: Callback::new(move |()| {
            end_time_ms.set(None);
            paused_remaining_ms.set(None);
            remaining.set(0);
            state.set(TimerState::Idle);
            let next_gen = generation.read().wrapping_add(1);
            generation.set(next_gen);
        }),
        adjust: Callback::new(move |delta_secs: i64| {
            let current_state = *state.read();
            match current_state {
                TimerState::Running => {
                    let end = *end_time_ms.read();
                    if let Some(end) = end {
                        let new_end = end + delta_secs * 1000;
                        let now = time::now_ms();
                        if new_end <= now {
                            end_time_ms.set(Some(now));
                            remaining.set(0);
                        } else {
                            end_time_ms.set(Some(new_end));
                            let remain_ms = new_end - now;
                            remaining.set((remain_ms + 999) / 1000);
                        }
                    }
                }
                TimerState::Paused => {
                    let remain = *paused_remaining_ms.read();
                    if let Some(remain) = remain {
                        let new_remain = (remain + delta_secs * 1000).max(0);
                        paused_remaining_ms.set(Some(new_remain));
                        remaining.set((new_remain + 999) / 1000);
                    }
                }
                _ => {}
            }
        }),
        dismiss: Callback::new(move |()| {
            if *state.read() == TimerState::Complete {
                end_time_ms.set(None);
                paused_remaining_ms.set(None);
                remaining.set(0);
                state.set(TimerState::Idle);
                let next_gen = generation.read().wrapping_add(1);
                generation.set(next_gen);
            }
        }),
    };

    (remaining, state, controls)
}
