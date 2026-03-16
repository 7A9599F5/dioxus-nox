//! Duration formatting utilities.

/// Format seconds as `M:SS` or `H:MM:SS`.
///
/// - Negative values are treated as `0:00`.
/// - Values under one hour display as `M:SS` (e.g., `1:05`).
/// - Values one hour or above display as `H:MM:SS` (e.g., `1:01:01`).
///
/// # Examples
///
/// ```
/// use dioxus_nox_timer::format_duration;
///
/// assert_eq!(format_duration(0), "0:00");
/// assert_eq!(format_duration(65), "1:05");
/// assert_eq!(format_duration(3661), "1:01:01");
/// assert_eq!(format_duration(-5), "0:00");
/// ```
pub fn format_duration(total_seconds: i64) -> String {
    let total_seconds = total_seconds.max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}
