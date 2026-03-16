//! Stopwatch hook — counts elapsed time from a start time.

use dioxus::prelude::*;

use crate::time;

/// Stopwatch hook — counts up from a start time.
///
/// Returns a signal with the elapsed seconds. Ticks every second via
/// wall-clock diff. Automatically stops ticking when `ended_at_ms` is provided.
///
/// # Parameters
///
/// - `started_at_ms`: Epoch milliseconds when the stopwatch started.
/// - `ended_at_ms`: Epoch milliseconds when it ended (`None` = still running).
///
/// # Example
///
/// ```rust,ignore
/// let started = dioxus_nox_timer::time::now_ms(); // capture start time
/// let elapsed = use_stopwatch(started, None); // None = still running
///
/// rsx! {
///     p { "Elapsed: {format_duration(*elapsed.read())}" }
/// }
/// ```
pub fn use_stopwatch(started_at_ms: i64, ended_at_ms: Option<i64>) -> Signal<i64> {
    let mut elapsed_secs = use_signal(|| {
        let end = ended_at_ms.unwrap_or_else(time::now_ms);
        ((end - started_at_ms).max(0)) / 1000
    });

    // If already ended, compute once and don't tick.
    let is_running = ended_at_ms.is_none();

    use_effect(move || {
        if !is_running {
            // Compute final elapsed once.
            if let Some(end) = ended_at_ms {
                elapsed_secs.set(((end - started_at_ms).max(0)) / 1000);
            }
            return;
        }
        spawn(async move {
            loop {
                time::sleep_ms(1000).await;
                let now = time::now_ms();
                let elapsed = ((now - started_at_ms).max(0)) / 1000;
                elapsed_secs.set(elapsed);
            }
        });
    });

    elapsed_secs
}
