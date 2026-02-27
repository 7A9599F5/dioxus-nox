//! # dioxus-virtual
//!
//! Viewport math for virtual scrolling lists with fixed-height items.
//!
//! This crate provides pure, testable viewport calculations that can be
//! used to implement virtual list rendering in Dioxus or any other framework.
//!
//! ## Usage
//!
//! ```rust
//! use dioxus_virtual::VirtualViewport;
//!
//! let mut vp = VirtualViewport::new(1000, 40, 400);
//! vp.scroll_top = 800;
//!
//! let (start, end) = vp.visible_range();
//! // Renders items start..end, with spacers for the rest
//! let top_spacer = vp.top_spacer_height();
//! let bottom_spacer = vp.bottom_spacer_height();
//! ```

mod viewport;

pub use viewport::VirtualViewport;

#[cfg(test)]
mod tests {
    use crate::VirtualViewport;

    #[test]
    fn visible_range_at_top() {
        let vp = VirtualViewport::new(100, 40, 400);
        let (start, end) = vp.visible_range();
        assert_eq!(start, 0); // clamped by overscan
        // visible = 400/40 = 10 items + overscan 5 on each side + 1 = 16 total from 0
        assert!(end <= 100);
        assert!(end >= 10); // at least visible items
    }

    #[test]
    fn visible_range_mid_scroll() {
        let mut vp = VirtualViewport::new(100, 40, 400);
        vp.scroll_top = 2000; // scroll to item 50
        let (start, end) = vp.visible_range();
        // first_visible = 2000/40 = 50, start = 50-5=45
        assert_eq!(start, 45);
        // end = 50 + ceil(400/40) + 5 + 1 = 50 + 10 + 6 = 66
        assert!(end > start);
        assert!(end <= 100);
    }

    #[test]
    fn visible_range_overscan_clamp() {
        // At scroll_top=0, start should be 0 (not negative)
        let vp = VirtualViewport {
            item_count: 20,
            item_height: 40,
            viewport_height: 200,
            scroll_top: 0,
            overscan: 10,
        };
        let (start, _) = vp.visible_range();
        assert_eq!(start, 0);
    }

    #[test]
    fn visible_range_end_clamp() {
        // Near end of list, end should not exceed item_count
        let mut vp = VirtualViewport::new(10, 40, 400);
        vp.scroll_top = 0;
        let (_, end) = vp.visible_range();
        assert!(end <= 10);
    }

    #[test]
    fn visible_range_empty_list() {
        let vp = VirtualViewport::new(0, 40, 400);
        assert_eq!(vp.visible_range(), (0, 0));
    }

    #[test]
    fn visible_range_zero_item_height() {
        let vp = VirtualViewport::new(100, 0, 400);
        assert_eq!(vp.visible_range(), (0, 0));
    }

    #[test]
    fn total_height() {
        let vp = VirtualViewport::new(100, 40, 400);
        assert_eq!(vp.total_height(), 4000);
    }

    #[test]
    fn total_height_empty() {
        let vp = VirtualViewport::new(0, 40, 400);
        assert_eq!(vp.total_height(), 0);
    }

    #[test]
    fn offset_for_idx() {
        let vp = VirtualViewport::new(100, 40, 400);
        assert_eq!(vp.offset_for_idx(0), 0);
        assert_eq!(vp.offset_for_idx(5), 200);
        assert_eq!(vp.offset_for_idx(10), 400);
    }

    #[test]
    fn top_spacer_at_top() {
        let vp = VirtualViewport::new(100, 40, 400);
        // At top, start=0, spacer=0
        assert_eq!(vp.top_spacer_height(), 0);
    }

    #[test]
    fn top_spacer_mid_scroll() {
        let mut vp = VirtualViewport::new(100, 40, 400);
        vp.scroll_top = 2000; // first_visible=50, start=45
        let top = vp.top_spacer_height();
        assert_eq!(top, 45 * 40); // 1800px
    }

    #[test]
    fn bottom_spacer_near_end() {
        let mut vp = VirtualViewport::new(100, 40, 400);
        vp.scroll_top = 3600; // near end
        let bottom = vp.bottom_spacer_height();
        // Should be small, rendering near-end items
        assert!(bottom < 400);
    }

    #[test]
    fn default_viewport() {
        let vp = VirtualViewport::default();
        assert_eq!(vp.item_height, 40);
        assert_eq!(vp.overscan, 5);
        assert_eq!(vp.scroll_top, 0);
    }

    #[test]
    fn clone_and_partial_eq() {
        let vp1 = VirtualViewport::new(50, 30, 300);
        let vp2 = vp1.clone();
        assert_eq!(vp1, vp2);
    }
}
