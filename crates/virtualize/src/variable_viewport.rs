//! Variable-height viewport calculations for virtual lists.
//!
//! All functions are pure math with no Dioxus or web-sys dependencies,
//! making them fully testable on any host platform.
//!
//! Uses a prefix-sum array for O(log n) scroll-position-to-index lookups
//! via binary search. Heights can be individually measured or use a
//! default estimate until measurement occurs.
//!
//! ## Two-tier API
//!
//! - **[`VariableViewport`]** — mutable measurement accumulator. Accepts height
//!   updates and builds [`LayoutSnapshot`]s on demand.
//! - **[`LayoutSnapshot`]** — immutable, read-only view with all query methods
//!   taking `&self`. Safe to share across multiple readers with no lock
//!   contention.

// ── LayoutSnapshot ──────────────────────────────────────────────────────────

/// Read-only snapshot of computed layout data.
///
/// Built by [`VariableViewport::snapshot`], all query methods take `&self`.
/// Multiple readers can access a snapshot concurrently without contention.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSnapshot {
    /// Prefix sums: `prefix[i]` = sum of heights for items `0..i`.
    prefix: Vec<u32>,
    item_count: usize,
    viewport_height: u32,
    scroll_top: u32,
    overscan: usize,
}

impl LayoutSnapshot {
    /// Compute the `[start, end)` range of item indices to render,
    /// including `overscan` extra items on each side.
    ///
    /// Returns `(0, 0)` for empty lists.
    pub fn visible_range(&self) -> (usize, usize) {
        if self.item_count == 0 {
            return (0, 0);
        }

        let first_visible = self.index_at_offset(self.scroll_top);

        let viewport_end = self.scroll_top.saturating_add(self.viewport_height);
        let mut last_visible = first_visible;
        while last_visible < self.item_count && self.prefix[last_visible] < viewport_end {
            last_visible += 1;
        }

        let start = first_visible.saturating_sub(self.overscan);
        let end = (last_visible + self.overscan).min(self.item_count);

        (start, end)
    }

    /// Total height of all items combined (pixels).
    pub fn total_height(&self) -> u32 {
        self.prefix[self.item_count]
    }

    /// Top offset of item at `idx` from the container top (pixels).
    pub fn offset_for_idx(&self, idx: usize) -> u32 {
        if idx >= self.prefix.len() {
            return self.prefix[self.item_count];
        }
        self.prefix[idx]
    }

    /// Height of the top spacer element.
    pub fn top_spacer_height(&self) -> u32 {
        let (start, _) = self.visible_range();
        self.offset_for_idx(start)
    }

    /// Height of the bottom spacer element.
    pub fn bottom_spacer_height(&self) -> u32 {
        let (_, end) = self.visible_range();
        let rendered_end_offset = self.offset_for_idx(end);
        self.total_height().saturating_sub(rendered_end_offset)
    }

    /// Returns `true` when the visible range end is within `threshold`
    /// items of the total count.
    pub fn is_near_end(&self, threshold: usize) -> bool {
        if self.item_count == 0 {
            return false;
        }
        let (_, end) = self.visible_range();
        end + threshold >= self.item_count
    }

    /// Binary search: find the index of the first item whose top edge is
    /// at or below `offset`. O(log n).
    fn index_at_offset(&self, offset: u32) -> usize {
        if self.item_count == 0 {
            return 0;
        }
        let pos = self.prefix.partition_point(|&p| p <= offset);
        pos.saturating_sub(1).min(self.item_count.saturating_sub(1))
    }

    /// Item count in this snapshot.
    pub fn item_count(&self) -> usize {
        self.item_count
    }
}

// ── VariableViewport ────────────────────────────────────────────────────────

/// Mutable measurement accumulator for variable-height virtual lists.
///
/// Accepts height updates from the rendering layer and builds
/// [`LayoutSnapshot`]s on demand. Query methods still exist on this type
/// (taking `&mut self` for lazy rebuild) for standalone / non-Dioxus usage,
/// but the component layer should prefer [`Self::snapshot`] + [`LayoutSnapshot`]
/// to avoid write-lock contention.
#[derive(Clone, Debug)]
pub struct VariableViewport {
    item_count: usize,
    default_estimate: u32,
    viewport_height: u32,
    scroll_top: u32,
    overscan: usize,
    /// Per-item heights. `None` means use `default_estimate`.
    heights: Vec<Option<u32>>,
    /// Prefix sums: `prefix[i]` = sum of heights for items `0..i`.
    /// Length is always `item_count + 1`, with `prefix[0] = 0`.
    prefix: Vec<u32>,
    /// Whether prefix sums need rebuilding.
    dirty: bool,
    /// Incremented each time prefix sums are rebuilt.
    measure_gen: u64,
}

impl VariableViewport {
    /// Create a new variable viewport with default overscan of 5.
    pub fn new(item_count: usize, default_estimate: u32, viewport_height: u32) -> Self {
        let heights = vec![None; item_count];
        let mut vp = Self {
            item_count,
            default_estimate,
            viewport_height,
            scroll_top: 0,
            overscan: 5,
            heights,
            prefix: Vec::new(),
            dirty: true,
            measure_gen: 0,
        };
        vp.rebuild_prefix();
        vp
    }

    /// Set the measured height for a single item.
    ///
    /// Returns the signed delta (`new - old`) so the caller can compute
    /// scroll corrections. If the item was previously unmeasured, the delta
    /// is relative to the default estimate.
    pub fn set_measured_height_with_delta(&mut self, index: usize, height: u32) -> i32 {
        if index >= self.item_count {
            return 0;
        }
        let old = self.height_of(index);
        self.heights[index] = Some(height);
        self.dirty = true;
        height as i32 - old as i32
    }

    /// Set the measured height for a single item (ignoring delta).
    pub fn set_measured_height(&mut self, index: usize, height: u32) {
        if index >= self.item_count {
            return;
        }
        self.heights[index] = Some(height);
        self.dirty = true;
    }

    /// Effective height of item at `index` (measured or default estimate).
    pub fn height_of(&self, index: usize) -> u32 {
        if index >= self.item_count {
            return 0;
        }
        self.heights[index].unwrap_or(self.default_estimate)
    }

    /// Rebuild the prefix-sum array from current heights.
    fn rebuild_prefix(&mut self) {
        self.prefix.clear();
        self.prefix.reserve(self.item_count + 1);
        self.prefix.push(0);
        let mut sum = 0u32;
        for i in 0..self.item_count {
            sum = sum.saturating_add(self.height_of(i));
            self.prefix.push(sum);
        }
        self.dirty = false;
        self.measure_gen += 1;
    }

    /// Ensure prefix sums are up-to-date. Called lazily before queries.
    fn ensure_prefix_sums(&mut self) {
        if self.dirty {
            self.rebuild_prefix();
        }
    }

    /// Binary search: find the index of the first item whose top edge is
    /// at or below `offset`. O(log n).
    fn index_at_offset(&mut self, offset: u32) -> usize {
        self.ensure_prefix_sums();
        if self.item_count == 0 {
            return 0;
        }
        // Find the largest i where prefix[i] <= offset.
        // partition_point returns the first i where prefix[i] > offset,
        // so we subtract 1 (clamped to 0).
        let pos = self.prefix.partition_point(|&p| p <= offset);
        pos.saturating_sub(1).min(self.item_count.saturating_sub(1))
    }

    /// Compute the `[start, end)` range of item indices to render,
    /// including `overscan` extra items on each side.
    ///
    /// Returns `(0, 0)` for empty lists.
    pub fn visible_range(&mut self) -> (usize, usize) {
        if self.item_count == 0 {
            return (0, 0);
        }
        self.ensure_prefix_sums();

        let first_visible = self.index_at_offset(self.scroll_top);

        // Walk forward to find how many items fit in the viewport.
        let viewport_end = self.scroll_top.saturating_add(self.viewport_height);
        let mut last_visible = first_visible;
        while last_visible < self.item_count && self.prefix[last_visible] < viewport_end {
            last_visible += 1;
        }

        let start = first_visible.saturating_sub(self.overscan);
        let end = (last_visible + self.overscan).min(self.item_count);

        (start, end)
    }

    /// Total height of all items combined (pixels).
    pub fn total_height(&mut self) -> u32 {
        self.ensure_prefix_sums();
        self.prefix[self.item_count]
    }

    /// Top offset of item at `idx` from the container top (pixels).
    pub fn offset_for_idx(&mut self, idx: usize) -> u32 {
        self.ensure_prefix_sums();
        if idx >= self.prefix.len() {
            return self.prefix[self.item_count];
        }
        self.prefix[idx]
    }

    /// Height of the top spacer element.
    pub fn top_spacer_height(&mut self) -> u32 {
        let (start, _) = self.visible_range();
        self.offset_for_idx(start)
    }

    /// Height of the bottom spacer element.
    pub fn bottom_spacer_height(&mut self) -> u32 {
        let (_, end) = self.visible_range();
        let rendered_end_offset = self.offset_for_idx(end);
        self.total_height().saturating_sub(rendered_end_offset)
    }

    /// Returns `true` when the visible range end is within `threshold`
    /// items of the total count.
    ///
    /// Useful for infinite scroll / load-more triggers.
    /// Returns `false` for empty lists.
    pub fn is_near_end(&mut self, threshold: usize) -> bool {
        if self.item_count == 0 {
            return false;
        }
        let (_, end) = self.visible_range();
        end + threshold >= self.item_count
    }

    /// Update the total item count (e.g., after loading more items).
    pub fn set_item_count(&mut self, count: usize) {
        if count == self.item_count {
            return;
        }
        self.heights.resize(count, None);
        self.item_count = count;
        self.dirty = true;
    }

    /// Update the current scroll position.
    pub fn set_scroll_top(&mut self, scroll_top: u32) {
        self.scroll_top = scroll_top;
    }

    /// Update the viewport container height.
    pub fn set_viewport_height(&mut self, height: u32) {
        self.viewport_height = height;
    }

    /// Current generation counter. Incremented on each prefix-sum rebuild.
    pub fn measure_gen(&self) -> u64 {
        self.measure_gen
    }

    /// Current scroll position.
    pub fn scroll_top(&self) -> u32 {
        self.scroll_top
    }

    /// Current viewport height.
    pub fn viewport_height(&self) -> u32 {
        self.viewport_height
    }

    /// Total item count.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Default estimated height for unmeasured items.
    pub fn default_estimate(&self) -> u32 {
        self.default_estimate
    }

    /// Overscan count.
    pub fn overscan(&self) -> usize {
        self.overscan
    }

    /// Set overscan count.
    pub fn set_overscan(&mut self, overscan: usize) {
        self.overscan = overscan;
    }

    /// Build a read-only [`LayoutSnapshot`] for the current state.
    ///
    /// The snapshot includes freshly computed prefix sums and is parameterized
    /// by `scroll_top` and `viewport_height` so it can be called from a Memo
    /// that subscribes to those signals separately.
    ///
    /// Cost: O(n) for the prefix-sum build. Intended to be memoized at the
    /// component level so it runs once per measurement batch, not per scroll.
    pub fn snapshot(&self, scroll_top: u32, viewport_height: u32) -> LayoutSnapshot {
        let mut prefix = Vec::with_capacity(self.item_count + 1);
        prefix.push(0);
        let mut sum = 0u32;
        for i in 0..self.item_count {
            sum = sum.saturating_add(self.height_of(i));
            prefix.push(sum);
        }
        LayoutSnapshot {
            prefix,
            item_count: self.item_count,
            viewport_height,
            scroll_top,
            overscan: self.overscan,
        }
    }
}

impl Default for VariableViewport {
    fn default() -> Self {
        Self::new(0, 40, 400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list() {
        let mut vp = VariableViewport::new(0, 40, 400);
        assert_eq!(vp.visible_range(), (0, 0));
        assert_eq!(vp.total_height(), 0);
        assert!(!vp.is_near_end(5));
    }

    #[test]
    fn single_item() {
        let mut vp = VariableViewport::new(1, 40, 400);
        let (start, end) = vp.visible_range();
        assert_eq!(start, 0);
        assert_eq!(end, 1);
        assert_eq!(vp.total_height(), 40);
        assert!(vp.is_near_end(5));
    }

    #[test]
    fn uniform_heights_at_top() {
        let mut vp = VariableViewport::new(100, 40, 400);
        let (start, end) = vp.visible_range();
        assert_eq!(start, 0);
        // first_visible=0, viewport fits 10 items, + overscan 5 = 15
        assert!(end >= 10);
        assert!(end <= 20);
    }

    #[test]
    fn uniform_heights_mid_scroll() {
        let mut vp = VariableViewport::new(100, 40, 400);
        vp.set_scroll_top(2000); // item 50
        let (start, end) = vp.visible_range();
        assert!(start <= 50);
        assert!(start >= 45); // 50 - overscan 5
        assert!(end > 50);
        assert!(end <= 100);
    }

    #[test]
    fn measured_height_delta() {
        let mut vp = VariableViewport::new(10, 40, 400);
        // Measure item 3 as 60px (was estimated 40px)
        let delta = vp.set_measured_height_with_delta(3, 60);
        assert_eq!(delta, 20); // 60 - 40

        // Measure again with same value: delta = 0
        let delta2 = vp.set_measured_height_with_delta(3, 60);
        assert_eq!(delta2, 0);
    }

    #[test]
    fn measured_height_shrink() {
        let mut vp = VariableViewport::new(10, 40, 400);
        let delta = vp.set_measured_height_with_delta(0, 20);
        assert_eq!(delta, -20); // 20 - 40
    }

    #[test]
    fn total_height_with_measurements() {
        let mut vp = VariableViewport::new(5, 40, 400);
        // Default total: 5 * 40 = 200
        assert_eq!(vp.total_height(), 200);

        vp.set_measured_height(0, 100);
        vp.set_measured_height(2, 10);
        // 100 + 40 + 10 + 40 + 40 = 230
        assert_eq!(vp.total_height(), 230);
    }

    #[test]
    fn offset_for_idx_with_measurements() {
        let mut vp = VariableViewport::new(5, 40, 400);
        vp.set_measured_height(0, 100);
        vp.set_measured_height(1, 50);
        // prefix: [0, 100, 150, 190, 230, 270]
        assert_eq!(vp.offset_for_idx(0), 0);
        assert_eq!(vp.offset_for_idx(1), 100);
        assert_eq!(vp.offset_for_idx(2), 150);
        assert_eq!(vp.offset_for_idx(3), 190);
    }

    #[test]
    fn binary_search_accuracy() {
        let mut vp = VariableViewport::new(5, 40, 400);
        vp.set_measured_height(0, 100);
        vp.set_measured_height(1, 50);
        // prefix: [0, 100, 150, 190, 230, 270]

        // Offset 0 → item 0
        assert_eq!(vp.index_at_offset(0), 0);
        // Offset 99 → still item 0
        assert_eq!(vp.index_at_offset(99), 0);
        // Offset 100 → item 1
        assert_eq!(vp.index_at_offset(100), 1);
        // Offset 150 → item 2
        assert_eq!(vp.index_at_offset(150), 2);
        // Offset 269 → item 4
        assert_eq!(vp.index_at_offset(269), 4);
    }

    #[test]
    fn spacer_heights() {
        let mut vp = VariableViewport::new(100, 40, 400);
        // At top: top spacer = 0
        assert_eq!(vp.top_spacer_height(), 0);

        vp.set_scroll_top(2000);
        let top = vp.top_spacer_height();
        assert!(top > 0);

        let bottom = vp.bottom_spacer_height();
        let total = vp.total_height();
        let (start, end) = vp.visible_range();
        let rendered_height: u32 = (start..end).map(|i| vp.height_of(i)).sum();
        // top_spacer + rendered + bottom_spacer = total
        assert_eq!(top + rendered_height + bottom, total);
    }

    #[test]
    fn set_item_count_grows() {
        let mut vp = VariableViewport::new(5, 40, 400);
        vp.set_measured_height(2, 100);
        vp.set_item_count(10);
        // Old measurements preserved
        assert_eq!(vp.height_of(2), 100);
        // New items use estimate
        assert_eq!(vp.height_of(7), 40);
        assert_eq!(vp.item_count(), 10);
    }

    #[test]
    fn set_item_count_shrinks() {
        let mut vp = VariableViewport::new(10, 40, 400);
        vp.set_measured_height(8, 100);
        vp.set_item_count(5);
        // Measurement at index 8 is gone
        assert_eq!(vp.item_count(), 5);
        assert_eq!(vp.total_height(), 200);
    }

    #[test]
    fn is_near_end_variable() {
        let mut vp = VariableViewport::new(20, 40, 400);
        // At top: visible ~0..15, is_near_end(5) = 15+5=20 >= 20 → true
        assert!(vp.is_near_end(5));

        let mut vp2 = VariableViewport::new(100, 40, 400);
        assert!(!vp2.is_near_end(5));
    }

    #[test]
    fn measure_gen_increments() {
        let mut vp = VariableViewport::new(10, 40, 400);
        let gen1 = vp.measure_gen();
        vp.set_measured_height(0, 50);
        let _ = vp.visible_range(); // triggers rebuild
        let gen2 = vp.measure_gen();
        assert!(gen2 > gen1);
    }

    #[test]
    fn out_of_bounds_measurement() {
        let mut vp = VariableViewport::new(5, 40, 400);
        let delta = vp.set_measured_height_with_delta(100, 50);
        assert_eq!(delta, 0);
        assert_eq!(vp.height_of(100), 0);
    }

    #[test]
    fn default_viewport() {
        let mut vp = VariableViewport::default();
        assert_eq!(vp.item_count(), 0);
        assert_eq!(vp.default_estimate(), 40);
        assert_eq!(vp.viewport_height(), 400);
        assert_eq!(vp.visible_range(), (0, 0));
    }

    #[test]
    fn large_list() {
        let mut vp = VariableViewport::new(100_000, 30, 600);
        vp.set_scroll_top(500_000);
        let (start, end) = vp.visible_range();
        assert!(start > 0);
        assert!(end > start);
        assert!(end <= 100_000);
        // Should be fast — just prefix rebuild + binary search
    }

    // ── LayoutSnapshot tests ────────────────────────────────────────────

    #[test]
    fn snapshot_empty() {
        let vp = VariableViewport::new(0, 40, 400);
        let snap = vp.snapshot(0, 400);
        assert_eq!(snap.visible_range(), (0, 0));
        assert_eq!(snap.total_height(), 0);
        assert!(!snap.is_near_end(5));
    }

    #[test]
    fn snapshot_matches_viewport() {
        // LayoutSnapshot should produce identical results to VariableViewport
        let mut vp = VariableViewport::new(100, 40, 400);
        vp.set_measured_height(5, 80);
        vp.set_measured_height(10, 20);
        vp.set_scroll_top(1000);

        let snap = vp.snapshot(1000, 400);

        assert_eq!(snap.visible_range(), vp.visible_range());
        assert_eq!(snap.total_height(), vp.total_height());
        assert_eq!(snap.offset_for_idx(5), vp.offset_for_idx(5));
        assert_eq!(snap.top_spacer_height(), vp.top_spacer_height());
        assert_eq!(snap.bottom_spacer_height(), vp.bottom_spacer_height());
        assert_eq!(snap.is_near_end(5), vp.is_near_end(5));
    }

    #[test]
    fn snapshot_with_different_scroll() {
        let mut vp = VariableViewport::new(100, 40, 400);
        vp.set_measured_height(0, 100);

        // Snapshot at scroll=0
        let snap1 = vp.snapshot(0, 400);
        // Snapshot at scroll=2000 (same measurements, different position)
        let snap2 = vp.snapshot(2000, 400);

        assert_ne!(snap1.visible_range(), snap2.visible_range());
        // But total height is the same
        assert_eq!(snap1.total_height(), snap2.total_height());
    }

    #[test]
    fn snapshot_is_immutable_after_mutation() {
        let mut vp = VariableViewport::new(10, 40, 400);
        let snap_before = vp.snapshot(0, 400);

        // Mutate the viewport after taking a snapshot
        vp.set_measured_height(0, 200);

        let snap_after = vp.snapshot(0, 400);

        // snap_before should still reflect old state
        assert_eq!(snap_before.total_height(), 400); // 10 * 40
        assert_eq!(snap_after.total_height(), 560); // 200 + 9*40
        assert_ne!(snap_before, snap_after);
    }

    #[test]
    fn snapshot_spacer_invariant() {
        let mut vp = VariableViewport::new(100, 40, 400);
        vp.set_measured_height(3, 100);
        vp.set_measured_height(50, 10);

        let snap = vp.snapshot(1500, 400);
        let (start, end) = snap.visible_range();
        let rendered_height: u32 = (start..end).map(|i| vp.height_of(i)).sum();

        // top_spacer + rendered + bottom_spacer = total
        assert_eq!(
            snap.top_spacer_height() + rendered_height + snap.bottom_spacer_height(),
            snap.total_height()
        );
    }
}
