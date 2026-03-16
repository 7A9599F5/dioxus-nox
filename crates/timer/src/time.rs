//! Cross-platform wall-clock time abstraction.
//!
//! Provides `now_ms()` for wall-clock timestamps and platform-appropriate
//! sleep for tick loops.

/// Returns the current wall-clock time as milliseconds since Unix epoch.
///
/// - **wasm32**: Uses `js_sys::Date::now()` (wall-clock, survives tab backgrounding).
/// - **non-wasm**: Uses `std::time::SystemTime`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn now_ms() -> i64 {
    js_sys::Date::now() as i64
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Platform-appropriate async sleep.
///
/// - **wasm32**: Uses `gloo_timers::future::TimeoutFuture`.
/// - **non-wasm**: Uses `tokio::time::sleep` (provided by Dioxus runtime).
#[cfg(target_arch = "wasm32")]
pub(crate) async fn sleep_ms(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn sleep_ms(ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}
