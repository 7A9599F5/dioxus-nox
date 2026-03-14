//! Virtual viewport calculations for fixed-height item lists.
//!
//! All functions are pure math with no Dioxus or web-sys dependencies,
//! making them fully testable on any host platform.

/// Tracks the visible window of a virtual list with fixed-height items.
///
/// All pixel values are in logical pixels (u32). Items outside the visible
/// range plus overscan are not rendered, reducing DOM node count dramatically
/// for large lists.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualViewport {
    /// Total number of items in the list.
    pub item_count: usize,
    /// Height of each item in pixels (fixed, all items same height).
    pub item_height: u32,
    /// Height of the scroll container in pixels.
    pub viewport_height: u32,
    /// Current scroll offset in pixels from the top.
    pub scroll_top: u32,
    /// Number of extra items to render beyond the visible window on each side.
    /// Reduces visual flicker during fast scrolling.
    pub overscan: usize,
}

impl VirtualViewport {
    /// Create a new viewport with default overscan of 5.
    pub fn new(item_count: usize, item_height: u32, viewport_height: u32) -> Self {
        Self {
            item_count,
            item_height,
            viewport_height,
            scroll_top: 0,
            overscan: 5,
        }
    }

    /// Compute the `[start, end)` range of item indices that should be rendered.
    ///
    /// Includes `overscan` extra items on each side. The range is clamped to
    /// `[0, item_count)`.
    ///
    /// Returns `(0, 0)` for empty lists or zero item height.
    pub fn visible_range(&self) -> (usize, usize) {
        if self.item_count == 0 || self.item_height == 0 {
            return (0, 0);
        }

        let first_visible = (self.scroll_top / self.item_height) as usize;
        let visible_count = if self.viewport_height == 0 {
            0
        } else {
            // Ceiling division: how many items fit in viewport_height
            self.viewport_height.div_ceil(self.item_height) as usize
        };

        let start = first_visible.saturating_sub(self.overscan);
        let end = (first_visible + visible_count + self.overscan + 1).min(self.item_count);

        (start, end)
    }

    /// Total height of all items combined (pixels).
    pub fn total_height(&self) -> u32 {
        self.item_count as u32 * self.item_height
    }

    /// Top offset of item at `idx` from the container top (pixels).
    pub fn offset_for_idx(&self, idx: usize) -> u32 {
        idx as u32 * self.item_height
    }

    /// Height of the top spacer element (fills space for items above the rendered range).
    pub fn top_spacer_height(&self) -> u32 {
        let (start, _) = self.visible_range();
        self.offset_for_idx(start)
    }

    /// Height of the bottom spacer element (fills space for items below the rendered range).
    pub fn bottom_spacer_height(&self) -> u32 {
        let (_, end) = self.visible_range();
        let rendered_end_offset = self.offset_for_idx(end);
        self.total_height().saturating_sub(rendered_end_offset)
    }

    /// Returns `true` when the visible range's end index is within
    /// `threshold` items of `item_count`.
    ///
    /// Useful for triggering data fetches for infinite scroll / load-more.
    /// Returns `false` for empty lists.
    pub fn is_near_end(&self, threshold: usize) -> bool {
        if self.item_count == 0 {
            return false;
        }
        let (_, end) = self.visible_range();
        end + threshold >= self.item_count
    }
}

impl Default for VirtualViewport {
    fn default() -> Self {
        Self {
            item_count: 0,
            item_height: 40,
            viewport_height: 400,
            scroll_top: 0,
            overscan: 5,
        }
    }
}
