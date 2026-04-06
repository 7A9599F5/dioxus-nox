mod components;
mod context;

/// Compound component namespace for time picker.
///
/// ```text
/// use dioxus_nox_time_picker::time_picker;
///
/// rsx! {
///     time_picker::Root {
///         time_picker::Hour {}
///         time_picker::Separator {}
///         time_picker::Minute {}
///         time_picker::Separator {}
///         time_picker::Second {}
///         time_picker::Period {}
///     }
/// }
/// ```
pub mod time_picker {
    pub use crate::components::{Hour, Minute, Period, Root, Second, Separator};
}

pub use context::TimePickerContext;

#[cfg(test)]
mod tests;
