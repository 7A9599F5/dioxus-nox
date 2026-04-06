pub mod presets;
pub mod segment;

mod date_field_components;
mod date_picker_components;
mod date_range_picker_components;

/// Standalone segmented date input (no popover).
pub mod date_field {
    pub use crate::date_field_components::{Input, Root};
}

/// Date picker with popover calendar.
pub mod date_picker {
    pub use crate::date_picker_components::{Calendar, Input, Popover, Root, Trigger};
    pub use crate::presets::PresetList;
}

/// Date range picker with popover calendar.
pub mod date_range_picker {
    pub use crate::date_range_picker_components::{
        Calendar, InputEnd, InputStart, Popover, Root, Trigger,
    };
    pub use crate::presets::PresetList;
}

// Re-export key types
pub use presets::{last_month, last_n_days, last_year, this_month, this_week, this_year};
pub use segment::DateSegment;

#[cfg(test)]
mod tests;
